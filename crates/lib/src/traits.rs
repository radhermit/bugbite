use std::fmt;

use reqwest::RequestBuilder;
use url::Url;

use crate::service::ServiceKind;
use crate::Error;

/// Return true if a type contains a given object, otherwise false.
pub trait Contains<T> {
    fn contains(&self, obj: &T) -> bool;
}

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
    fn api(&self) -> String;
}

pub(crate) trait Request {
    type Output;
    async fn send(self) -> crate::Result<Self::Output>;
}

/// Placeholder request that does nothing.
pub(crate) struct NullRequest;

impl Request for NullRequest {
    type Output = ();
    async fn send(self) -> crate::Result<Self::Output> {
        Ok(())
    }
}

pub trait WebClient<'a> {
    type Service;
    type CreateParams: ServiceParams<'a>;
    type ModifyParams: ServiceParams<'a>;
    type SearchQuery: Query + ServiceParams<'a>;

    /// Return the service,
    fn service(&'a self) -> &'a Self::Service;

    /// Create a create params builder for the service.
    fn create_params(&'a self) -> Self::CreateParams;

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

pub(crate) trait WebService<'a>: WebClient<'a> + fmt::Display {
    const API_VERSION: &'static str;
    type Response;
    type GetRequest: Request;
    type CreateRequest: Request;
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
    fn get_request<S>(
        &'a self,
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
    fn create_request(&'a self, _params: Self::CreateParams) -> crate::Result<Self::CreateRequest> {
        Err(Error::Unsupported(format!(
            "{}: create requests unsupported",
            self.kind()
        )))
    }

    /// Create a modify request for bugs, issues, or tickets.
    fn modify_request<S>(
        &'a self,
        _ids: &[S],
        _params: Self::ModifyParams,
    ) -> crate::Result<Self::ModifyRequest>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: modify requests unsupported",
            self.kind()
        )))
    }

    /// Create a search request for bugs, issues, or tickets.
    fn search_request(&'a self, _query: Self::SearchQuery) -> crate::Result<Self::SearchRequest> {
        Err(Error::Unsupported(format!(
            "{}: search requests unsupported",
            self.kind()
        )))
    }
}
