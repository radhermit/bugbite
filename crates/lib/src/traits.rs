use std::fmt;
use std::future::Future;

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

impl Api for u64 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for i64 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for i32 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl<T: Api> Api for &T {
    fn api(&self) -> String {
        (*self).api()
    }
}

pub trait RequestSend {
    type Output;
    type Service;

    fn send(self, service: &Self::Service) -> impl Future<Output = crate::Result<Self::Output>>;
}

/// Inject service authentication data into a request.
pub(crate) trait InjectAuth: Sized {
    /// Authentication required for request.
    fn auth<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self>;

    /// Authentication optional for request.
    fn auth_optional<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self>;
}

impl InjectAuth for RequestBuilder {
    fn auth<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self> {
        service.inject_auth(self, true)
    }

    fn auth_optional<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self> {
        service.inject_auth(self, false)
    }
}

pub(crate) trait WebService<'a>: fmt::Display {
    const API_VERSION: &'static str;
    type Response;

    /// Inject authentication into a request before it's sent.
    fn inject_auth(
        &self,
        _request: RequestBuilder,
        _required: bool,
    ) -> crate::Result<RequestBuilder> {
        unimplemented!("authentication unsupported")
    }

    /// Parse a raw response into a service response.
    async fn parse_response(&self, _response: reqwest::Response) -> crate::Result<Self::Response> {
        unimplemented!("request parsing unsupported")
    }
}

pub trait WebClient<'a> {
    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
}
