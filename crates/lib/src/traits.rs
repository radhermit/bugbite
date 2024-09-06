use std::borrow::Cow;
use std::future::Future;
use std::{fmt, fs};

use async_stream::try_stream;
use camino::Utf8Path;
use futures_util::Stream;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::service::ServiceKind;
use crate::Error;

/// Return true if a type contains a given object, otherwise false.
pub trait Contains<T> {
    fn contains(&self, obj: &T) -> bool;
}

/// Render an object in search context into a formatted string using the given fields.
pub trait RenderSearch<T> {
    fn render(&self, fields: &[T]) -> String;
}

/// Encode a type into the expected API name.
pub(crate) trait Api {
    fn api(&self) -> String;
}

impl Api for String {
    fn api(&self) -> String {
        self.clone()
    }
}

impl Api for &str {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl<T: fmt::Display + Clone> Api for Cow<'_, T> {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for u64 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for usize {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for i64 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl<T: Api> Api for &T {
    fn api(&self) -> String {
        (*self).api()
    }
}

pub trait MergeOption<T> {
    fn merge(&mut self, value: Option<T>) -> Self;
}

impl<T> MergeOption<T> for Option<T> {
    fn merge(&mut self, value: Option<T>) -> Self {
        value.or_else(|| self.take())
    }
}

pub trait Merge {
    fn merge(&mut self, other: Self);
}

macro_rules! try_from_toml {
    ($x:ty, $desc:expr) => {
        impl TryFrom<&camino::Utf8Path> for $x {
            type Error = $crate::Error;

            fn try_from(path: &camino::Utf8Path) -> $crate::Result<Self> {
                let data = fs::read_to_string(path).map_err(|e| {
                    Error::InvalidValue(format!("failed loading {}: {path}: {e}", $desc))
                })?;
                toml::from_str(&data).map_err(|e| {
                    Error::InvalidValue(format!("failed parsing {}: {path}: {e}", $desc))
                })
            }
        }
    };
}
pub(crate) use try_from_toml;

pub trait RequestSend {
    type Output;

    fn send(&self) -> impl Future<Output = crate::Result<Self::Output>>;
}

pub trait RequestStream: RequestSend<Output = Vec<Self::Item>> + Clone {
    type Item;

    /// Return the page size if paging is enabled.
    fn paged(&mut self) -> Option<usize>;
    /// Modify the request to return the next page.
    fn next_page(&mut self, size: usize);

    // TODO: submit multiple requests at once?
    /// Send requests and return the stream of items for them.
    fn stream(&self) -> impl Stream<Item = crate::Result<Self::Item>> + '_ {
        let mut req = self.clone();
        let paged = req.paged();

        try_stream! {
            loop {
                let items = req.send().await?;
                let count = items.len();

                for item in items {
                    yield item;
                }

                match paged {
                    Some(size) if count == size => req.next_page(size),
                    _ => break,
                }
            }
        }
    }
}

pub trait RequestTemplate: for<'a> Deserialize<'a> + Serialize + Merge {
    fn merge_template(&mut self, path: &Utf8Path) -> crate::Result<()> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        let params = toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))?;
        self.merge(params);
        Ok(())
    }

    fn save_template(&self, path: &Utf8Path) -> crate::Result<()> {
        let data = toml::to_string(self)
            .map_err(|e| Error::InvalidValue(format!("failed serializing request: {e}")))?;
        fs::write(path, data)
            .map_err(|e| Error::IO(format!("failed saving template: {path}: {e}")))?;
        Ok(())
    }
}

/// Inject service authentication data into a request.
pub(crate) trait InjectAuth: Sized {
    /// Authentication required for request.
    fn auth<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self>;

    /// Authentication optional for request.
    fn auth_optional<'a, W: WebService<'a>>(self, service: &'a W) -> Self;
}

impl InjectAuth for RequestBuilder {
    fn auth<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self> {
        service.inject_auth(self, true)
    }

    fn auth_optional<'a, W: WebService<'a>>(self, service: &'a W) -> Self {
        service
            .inject_auth(self, false)
            .expect("failed injecting optional auth")
    }
}

pub(crate) trait WebService<'a>: fmt::Display {
    const API_VERSION: &'static str;
    type Response;

    /// Inject authentication into a request before it's sent.
    fn inject_auth(&self, request: RequestBuilder, required: bool)
        -> crate::Result<RequestBuilder>;

    /// Parse a raw response into a service response.
    async fn parse_response(&self, response: reqwest::Response) -> crate::Result<Self::Response>;
}

pub trait WebClient<'a> {
    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
}
