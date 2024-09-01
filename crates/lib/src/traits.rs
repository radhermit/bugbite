use std::borrow::Cow;
use std::fmt;
use std::future::Future;

use async_stream::try_stream;
use futures_util::Stream;
use reqwest::RequestBuilder;
use url::Url;

use crate::service::ServiceKind;

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

pub trait RequestMerge<T> {
    fn merge(&mut self, value: T) -> crate::Result<()>;
}

pub trait RequestSend {
    type Output;

    fn send(&self) -> impl Future<Output = crate::Result<Self::Output>>;
}

pub trait RequestStream: RequestSend<Output = Vec<Self::Item>> + Clone {
    type Item;

    fn max_search_results(&self) -> usize;
    fn limit(&self) -> Option<usize>;
    fn set_limit(&mut self, value: usize);
    fn offset(&self) -> Option<usize>;
    fn set_offset(&mut self, value: usize);

    // TODO: submit multiple requests at once?
    fn stream(&self) -> impl Stream<Item = crate::Result<Self::Item>> + '_ {
        let paged = self.limit().is_none();
        let limit = self.max_search_results();
        let mut offset = self.offset().unwrap_or_default();

        let mut req = self.clone();
        if paged {
            req.set_limit(limit);
        }

        try_stream! {
            loop {
                if paged {
                    req.set_offset(offset);
                    offset += limit;
                }

                let items = req.send().await?;
                let count = items.len();

                for item in items {
                    yield item;
                }

                if !paged || count != limit {
                    break;
                }
            }
        }
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
