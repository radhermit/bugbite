use reqwest::Request;
use url::Url;

use crate::service::ServiceKind;
use crate::Error;

/// Encode into an application/x-www-form-urlencoded string format.
pub trait Params {
    fn params(&mut self) -> String;
}

pub trait WebService {
    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
    /// Return the service client.
    fn client(&self) -> &reqwest::Client;

    /// Create an bug, issue, or ticket request.
    fn get_request<S>(&self, _id: S, _comments: bool, _attachments: bool) -> crate::Result<Request>
    where
        S: std::fmt::Display,
    {
        Err(Error::Unsupported(format!(
            "{}: get requests unsupported",
            self.kind()
        )))
    }

    /// Create a search request for bugs, issues, or tickets.
    fn search_request<P: Params>(&self, _query: P) -> crate::Result<Request> {
        Err(Error::Unsupported(format!(
            "{}: search requests unsupported",
            self.kind()
        )))
    }
}
