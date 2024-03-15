use std::num::NonZeroU64;

use reqwest::RequestBuilder;
use url::Url;

use crate::service::ServiceKind;
use crate::Error;

pub trait Query {
    /// Returns true if no relevant parameters are defined, false otherwise.
    fn is_empty(&self) -> bool {
        true
    }
    /// Encode query parameters into the application/x-www-form-urlencoded string format.
    fn params(&mut self) -> crate::Result<String>;
}

pub trait ServiceParams<'a> {
    type Service;
    fn new(service: &'a Self::Service) -> Self;
}

/// Render an object in search context into a formatted string using the given fields.
pub trait RenderSearch<T> {
    fn render(&self, fields: &[T]) -> String;
}

/// Encode a type into the expected API name.
pub(crate) trait Api {
    type Output: std::fmt::Display;
    fn api(&self) -> Self::Output;
}

/// Scan a response for a web service error, raising it if one exists.
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

pub trait WebClient<'a> {
    type Service;
    type ModifyParams: ServiceParams<'a>;
    type SearchQuery: Query + ServiceParams<'a>;

    /// Return the service,
    fn service(&'a self) -> &'a Self::Service;

    /// Create a modification params builder for the service.
    fn modify_params(&'a self) -> Self::ModifyParams;

    /// Create a search query builder for the service.
    fn search_query(&'a self) -> Self::SearchQuery;
}

pub(crate) trait InjectAuth: Sized {
    fn inject_auth<'a, W: WebService<'a>>(
        self,
        service: &'a W,
        required: bool,
    ) -> crate::Result<Self>;
}

impl InjectAuth for RequestBuilder {
    fn inject_auth<'a, W: WebService<'a>>(
        self,
        service: &'a W,
        required: bool,
    ) -> crate::Result<Self> {
        service.inject_auth(self, required)
    }
}

pub(crate) trait WebService<'a>: WebClient<'a> {
    const API_VERSION: &'static str;
    type Response;
    type GetRequest: Request;
    type ModifyRequest: Request;
    type SearchRequest: Request;

    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
    /// Return the service client.
    fn client(&self) -> &reqwest::Client;
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
    fn get_request(
        &self,
        _ids: &[NonZeroU64],
        _attachments: bool,
        _comments: bool,
        _history: bool,
    ) -> crate::Result<Self::GetRequest> {
        Err(Error::Unsupported(format!(
            "{}: get requests unsupported",
            self.kind()
        )))
    }

    /// Create a modify request for bugs, issues, or tickets.
    fn modify_request(
        &self,
        _ids: &[NonZeroU64],
        _params: Self::ModifyParams,
    ) -> crate::Result<Self::ModifyRequest> {
        Err(Error::Unsupported(format!(
            "{}: modify requests unsupported",
            self.kind()
        )))
    }

    /// Create a search request for bugs, issues, or tickets.
    fn search_request<Q: Query>(&self, _query: Q) -> crate::Result<Self::SearchRequest> {
        Err(Error::Unsupported(format!(
            "{}: search requests unsupported",
            self.kind()
        )))
    }
}
