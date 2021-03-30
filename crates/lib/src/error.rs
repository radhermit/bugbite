#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    InvalidValue(String),
    #[error("{0}")]
    Json(serde_json::Error),
    #[error("bugzilla error: {message}")]
    Bugzilla { code: i64, message: String },
    #[error("{0}")]
    Request(String),
    #[error("{0}")]
    Unsupported(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Request(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}
