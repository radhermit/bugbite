use bugbite::error::python::BugbiteError;
use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;

use crate::service;
use crate::traits::ToStr;

#[pyclass(mapping, module = "bugbite.config")]
pub(super) struct Config(::bugbite::config::Config);

#[pymethods]
impl Config {
    #[new]
    fn new() -> PyResult<Self> {
        let config = ::bugbite::config::Config::new()?;
        Ok(Self(config))
    }

    /// Get a bugzilla service using a configured connection.
    fn bugzilla(&self, name: &str) -> PyResult<crate::bugzilla::Bugzilla> {
        self.0
            .get(name)
            .cloned()
            .ok_or_else(|| BugbiteError::new_err(format!("unknown service: {name}")))?
            .try_into()
    }

    /// Get a redmine service using a configured connection.
    fn redmine(&self, name: &str) -> PyResult<crate::redmine::Redmine> {
        self.0
            .get(name)
            .cloned()
            .ok_or_else(|| BugbiteError::new_err(format!("unknown service: {name}")))?
            .try_into()
    }

    fn __iter__(&self) -> PyResult<_Iter> {
        Ok(_Iter(self.0.clone().into_iter()))
    }

    fn __len__(&self) -> usize {
        self.0.len()
    }

    fn __contains__(&self, object: PyObject, py: Python<'_>) -> bool {
        if let Ok(name) = object.to_str(py) {
            self.0.contains_key(name)
        } else {
            false
        }
    }

    fn __getitem__(&self, object: PyObject, py: Python<'_>) -> PyResult<service::Config> {
        if let Ok(name) = object.to_str(py) {
            self.0
                .get(name)
                .map(|x| x.clone().into())
                .ok_or_else(|| PyKeyError::new_err(name.to_string()))
        } else {
            Err(PyTypeError::new_err("Config indices must be strings"))
        }
    }
}

#[pyclass]
struct _Iter(indexmap::map::IntoIter<String, ::bugbite::service::Config>);

#[pymethods]
impl _Iter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(String, service::Config)> {
        slf.0.next().map(|(name, config)| (name, config.into()))
    }
}

#[pymodule]
#[pyo3(name = "config")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Config>()?;
    Ok(())
}
