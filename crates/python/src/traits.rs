use pyo3::prelude::*;
use pyo3::types::PyString;

pub(crate) trait ToStr {
    fn to_str<'a>(&'a self, py: Python<'a>) -> PyResult<&'a str>;
}

impl ToStr for PyObject {
    fn to_str<'a>(&'a self, py: Python<'a>) -> PyResult<&'a str> {
        self.downcast_bound::<PyString>(py).map(|x| x.to_str())?
    }
}
