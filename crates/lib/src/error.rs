use std::io;

#[cfg(feature = "python")]
pub mod python;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("authentication required")]
    Auth,
    #[error("{0}")]
    Config(String),
    #[error("no parameters specified")]
    EmptyParams,
    #[error("invalid URL: {0}")]
    InvalidUrl(url::ParseError),
    #[error("{0}")]
    InvalidRequest(String),
    #[error("invalid service response: {0}")]
    InvalidResponse(String),
    #[error("{0}")]
    InvalidValue(String),
    #[error("{0}")]
    IO(String),
    #[error("bugzilla: {message}")]
    Bugzilla { code: i64, message: String },
    #[error("redmine: {0}")]
    Redmine(String),
    #[error("{0}")]
    Request(reqwest::Error),
    #[error("request timed out")]
    Timeout,
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        // drop URL from error to avoid potentially leaking authentication parameters
        let e = e.without_url();
        if e.is_timeout() {
            Error::Timeout
        } else {
            Error::Request(e)
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(format!("{e}: {}", e.kind()))
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::InvalidUrl(e)
    }
}
