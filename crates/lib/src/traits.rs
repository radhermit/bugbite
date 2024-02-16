use url::Url;

use crate::service::ServiceKind;
use crate::Error;

/// Encode into an application/x-www-form-urlencoded string format.
pub trait Params {
    fn params(&mut self) -> String;
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

pub(crate) trait WebService {
    type Response;
    type AttachmentsRequest: Request;
    type GetRequest: Request;
    type SearchRequest: Request;

    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
    /// Return the service client.
    fn client(&self) -> &reqwest::Client;
    /// Parse a raw response into a service response.
    async fn parse_response(&self, _response: reqwest::Response) -> crate::Result<Self::Response> {
        Err(Error::Unsupported(format!(
            "{}: request parsing unsupported",
            self.kind()
        )))
    }

    /// Create a request for attachments by attachment IDs.
    fn attachments_request<S>(
        &self,
        _ids: &[S],
        _data: bool,
    ) -> crate::Result<Self::AttachmentsRequest>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: attachments requests unsupported",
            self.kind()
        )))
    }

    /// Create a request for attachments by item IDs.
    fn item_attachments_request<S>(
        &self,
        _ids: &[S],
        _data: bool,
    ) -> crate::Result<Self::AttachmentsRequest>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: attachments requests unsupported",
            self.kind()
        )))
    }

    /// Create a request for bugs, issues, or tickets by their IDs.
    fn get_request<S>(
        &self,
        _ids: &[S],
        _comments: bool,
        _attachments: bool,
    ) -> crate::Result<Self::GetRequest>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: get requests unsupported",
            self.kind()
        )))
    }

    /// Create a search request for bugs, issues, or tickets.
    fn search_request<P: Params>(&self, _query: P) -> crate::Result<Self::SearchRequest> {
        Err(Error::Unsupported(format!(
            "{}: search requests unsupported",
            self.kind()
        )))
    }
}
