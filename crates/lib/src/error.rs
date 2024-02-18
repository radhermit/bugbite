use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Config(String),
    #[error("no search terms specified")]
    EmptyQuery,
    #[error("invalid URL: {0}")]
    InvalidUrl(url::ParseError),
    #[error("{0}")]
    InvalidValue(String),
    #[error("{0}")]
    IO(String),
    #[error("{0}")]
    Json(serde_json::Error),
    #[error("bugzilla error: {message}")]
    Bugzilla { code: i64, message: String },
    #[error("{0}")]
    Request(reqwest::Error),
    #[error("{0}")]
    Unsupported(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Request(e.without_url())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(format!("{e}: {}", e.kind()))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::InvalidUrl(e)
    }
}
