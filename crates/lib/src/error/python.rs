use pyo3::exceptions::PyException;
use pyo3::{PyErr, create_exception};

use super::Error;

create_exception!(bugbite, BugbiteError, PyException, "Generic bugbite error.");

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        BugbiteError::new_err(err.to_string())
    }
}
