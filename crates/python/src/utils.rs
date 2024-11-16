use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use pyo3::types::{timezone_utc, PyDateTime};

/// Convert rust-based DateTime into PyDateTime.
pub(crate) fn datetime(value: DateTime<Utc>, py: Python<'_>) -> PyResult<Bound<'_, PyDateTime>> {
    let value = value.timestamp() as f64;
    let tz = timezone_utc(py);
    PyDateTime::from_timestamp(py, value, Some(&tz))
}

/// Return a static tokio runtime.
pub(crate) fn tokio() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}
