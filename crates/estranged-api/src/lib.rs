use std::{collections::BTreeSet, num::NonZero, sync::LazyLock};

use bytes::Bytes;
use estranged_types::{
    ChatId, Marker, Mid, NewMessageBody, Recipient, RequestResult, SendResult, Subscription,
    SubscriptionRequest, Update, UpdateType, Updates, UploadType, UploadedInfo, UploadsResponse,
    UserId,
};
use futures_util::{Stream, TryStream};
use genawaiter_try_stream::try_stream;
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
    #[error(transparent)]
    Acquire(#[from] tokio::sync::AcquireError),
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

    pub fn update_stream(
        &self,
        timeout: Option<u8>,
        types: &BTreeSet<UpdateType>,
    ) -> impl Stream<Item = Result<Update>> {
        try_stream(async move |co| {
            let mut m = None;
            loop {
                let Updates { updates, marker } = self.updates(timeout, m, types).await?;
                for update in updates {
                    co.yield_(update).await;
                }
                m = marker;
            }
        })
    }

    pub async fn send(
        &self,
        user_id: Option<UserId>,
        chat_id: Option<ChatId>,
        disable_link_preview: Option<bool>,
        message: &NewMessageBody,
    ) -> Result<SendResult> {
        let mut url = self.base_url();
        url.set_path("/messages");
        if let Some(user_id) = user_id {
            url.query_pairs_mut()
                .append_pair("user_id", &user_id.to_string());
        }
        if let Some(chat_id) = chat_id {
            url.query_pairs_mut()
                .append_pair("chat_id", &chat_id.to_string());
        }
        if let Some(disable_link_preview) = disable_link_preview {
            url.query_pairs_mut()
                .append_pair("disable_link_preview", &disable_link_preview.to_string());
        }
        self.start_url(Method::POST, url)
            .json(message)
            .pull_json()
            .await
    }

    pub async fn reply(
        &self,
        Recipient {
            chat_id, user_id, ..
        }: &Recipient,
        disable_link_preview: Option<bool>,
        message: &NewMessageBody,
    ) -> Result<SendResult> {
        self.send(*user_id, *chat_id, disable_link_preview, message)
            .await
    }

    pub async fn edit(&self, message_id: Mid, message: &NewMessageBody) -> Result<()> {
        let mut url = self.base_url();
        url.set_path("/messages");
        url.query_pairs_mut()
            .append_pair("message_id", &message_id.to_string());
        self.start_url(Method::PUT, url)
            .json(message)
            .pull_result()
            .await?;
        Ok(())
    }

    pub async fn upload<T>(
        &self,
        r#type: UploadType,
        stream: impl 'static
        + Send
        + TryStream<Ok = T, Error: Into<Box<dyn Send + Sync + std::error::Error>>>,
    ) -> Result<UploadedInfo>
    where
        Bytes: From<T>,
    {
        static SEMAPHORE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
        let _guard = SEMAPHORE.acquire().await?;
        let mut url = self.base_url();
        url.set_path("/uploads");
        url.query_pairs_mut()
            .append_pair("type", &r#type.to_string());
        let UploadsResponse { url } = self
            .start_url(Method::POST, url)
            .pull_json()
            .await
            .inspect_err(|e| tracing::error!("failed to start uploading: {e}"))?;
        self.client
            .post(url)
            .header("content-type", "application/json")
            .body(reqwest::Body::wrap_stream(stream))
            .pull_json()
            .await
            .inspect_err(|e| tracing::error!("failed to finish uploading: {e}"))
    }
}
