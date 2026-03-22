use std::{collections::BTreeSet, num::NonZero, sync::LazyLock};

use estranged_types::{
    Marker, RequestResult, Subscription, SubscriptionRequest, UpdateType, Updates,
};
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};
use itertools::Itertools;
use reqwest::{Client, Method, RequestBuilder, Url};
use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Types(#[from] estranged_types::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct MaxApi {
    access_token: String,
    client: Client,
}

trait BuilderExt: Sized + Send {
    fn pull_json<T: DeserializeOwned>(self) -> impl Send + Future<Output = Result<T>>;

    fn pull_result(self) -> impl Send + Future<Output = Result<()>> {
        async { Ok(self.pull_json::<RequestResult>().await?.into_result()?) }
    }
}

impl BuilderExt for RequestBuilder {
    async fn pull_json<T: DeserializeOwned>(self) -> Result<T> {
        static LIMITER: LazyLock<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> =
            LazyLock::new(|| RateLimiter::direct(Quota::per_second(NonZero::new(30).unwrap())));
        LIMITER.until_ready().await;
        Ok(self.send().await?.json().await?)
    }
}

impl MaxApi {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            client: Client::new(),
        }
    }

    fn start_url(&self, method: Method, url: Url) -> RequestBuilder {
        self.client
            .request(method, url)
            .header("authorization", &self.access_token)
    }

    fn base_url(&self) -> Url {
        Url::parse("https://platform-api.max.ru").unwrap()
    }

    fn start_path(&self, method: Method, path: &str) -> RequestBuilder {
        let mut url = self.base_url();
        url.set_path(path);
        self.start_url(method, url)
    }

    pub async fn subscriptions(&self) -> Result<Vec<Subscription>> {
        self.start_path(Method::GET, "/subscriptions")
            .pull_json()
            .await
    }

    pub async fn subscribe(&self, request: &SubscriptionRequest) -> Result<()> {
        self.start_path(Method::POST, "/subscriptions")
            .json(request)
            .pull_result()
            .await
    }

    pub async fn unsubscribe(&self, url: &Url) -> Result<()> {
        let value = url.as_str();
        let mut url = self.base_url();
        url.set_path("/subscriptions");
        url.query_pairs_mut().append_pair("url", value);
        self.start_url(Method::DELETE, url).pull_result().await
    }

    pub async fn updates(
        &self,
        timeout: Option<u8>,
        marker: Option<Marker>,
        types: &BTreeSet<UpdateType>,
    ) -> Result<Updates> {
        let mut url = self.base_url();
        url.set_path("/updates");
        if let Some(timeout) = timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        if let Some(marker) = marker {
            url.query_pairs_mut()
                .append_pair("marker", &marker.to_string());
        }
        if !types.is_empty() {
            url.query_pairs_mut()
                .append_pair("types", &types.iter().join(","));
        }
        self.start_url(Method::GET, url).pull_json().await
    }
}
