use std::fmt::Display;

use estranged_types::Secret;
use headers::Header;
use http::{HeaderName, HeaderValue};
use itertools::Itertools;

pub struct MaxBotApiSecret(pub Secret);

trait LogThenInvalid {
    type T;
    fn log_then_invalid(self) -> Result<Self::T, headers::Error>;
}

impl<T, E: Display> LogThenInvalid for Result<T, E> {
    type T = T;

    fn log_then_invalid(self) -> Result<Self::T, headers::Error> {
        self.inspect_err(|e| tracing::error!("{e}"))
            .map_err(|_| headers::Error::invalid())
    }
}

impl Header for MaxBotApiSecret {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("x-max-bot-api-secret");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.exactly_one().log_then_invalid()?;
        let value = value.to_str().log_then_invalid()?;
        value.parse().map(Self).log_then_invalid()
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(
            HeaderValue::from_str(&format!("x-max-bot-api-secret: {}", self.0)).log_then_invalid(),
        );
    }
}
