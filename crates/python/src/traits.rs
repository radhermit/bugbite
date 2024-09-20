use pyo3::prelude::*;
use pyo3::types::PyString;

pub(crate) trait ToStr {
    fn to_str(&self) -> PyResult<&str>;
    fn to_str_owned(&self) -> PyResult<String> {
        self.to_str().map(|x| x.to_string())
    }
}

pub(crate) trait ToStrWithBound {
    fn to_str_with_bound<'a>(&'a self, py: Python<'a>) -> PyResult<&'a str>;
}

impl ToStr for Bound<'_, PyAny> {
    fn to_str(&self) -> PyResult<&str> {
        self.downcast::<PyString>().map(|x| x.to_str())?
    }
}

impl ToStrWithBound for PyObject {
    fn to_str_with_bound<'a>(&'a self, py: Python<'a>) -> PyResult<&'a str> {
        self.downcast_bound::<PyString>(py).map(|x| x.to_str())?
    }
}
