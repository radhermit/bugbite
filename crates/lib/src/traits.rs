use std::fmt;

use reqwest::RequestBuilder;
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

pub(crate) trait Request {
    type Output;
    type Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output>;
}

/// Placeholder request that does nothing.
pub(crate) struct NullRequest;

impl Request for NullRequest {
    type Output = ();
    type Service = ();

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        Ok(())
    }
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
    type GetRequest: Request;
    type CreateRequest: Request;
    type CreateParams;
    type UpdateRequest: Request;
    type UpdateParams;
    type SearchRequest: Request;
    type SearchParams;

    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
    /// Return the current service user if one exists.
    fn user(&self) -> Option<&str> {
        None
    }

    /// Inject authentication into a request before it's sent.
    fn inject_auth(
        &self,
        request: RequestBuilder,
        _required: bool,
    ) -> crate::Result<RequestBuilder> {
        Ok(request)
    }

    /// Parse a raw response into a service response.
    async fn parse_response(&self, _response: reqwest::Response) -> crate::Result<Self::Response> {
        Err(Error::Unsupported(format!(
            "{}: request parsing unsupported",
            self.kind()
        )))
    }

    /// Create a request for bugs, issues, or tickets by their IDs.
    fn get_request<S>(
        &self,
        _ids: &[S],
        _attachments: bool,
        _comments: bool,
        _history: bool,
    ) -> crate::Result<Self::GetRequest>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: get requests unsupported",
            self.kind()
        )))
    }

    /// Create a creation request for a bug, issue, or ticket.
    fn create_request(&self, _params: Self::CreateParams) -> crate::Result<Self::CreateRequest> {
        Err(Error::Unsupported(format!(
            "{}: create requests unsupported",
            self.kind()
        )))
    }

    /// Create an update request for bugs, issues, or tickets.
    fn update_request<S>(
        &self,
        _ids: &[S],
        _params: Self::UpdateParams,
    ) -> crate::Result<Self::UpdateRequest>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: update requests unsupported",
            self.kind()
        )))
    }

    /// Create a search request for bugs, issues, or tickets.
    fn search_request(&self, _query: Self::SearchParams) -> crate::Result<Self::SearchRequest> {
        Err(Error::Unsupported(format!(
            "{}: search requests unsupported",
            self.kind()
        )))
    }
}
