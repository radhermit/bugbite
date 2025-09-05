use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;

use crate::service;
use crate::traits::ToStrWithBound;

#[pyclass(mapping, module = "bugbite.config")]
pub(super) struct Config(bugbite::config::Config);

#[pymethods]
impl Config {
    #[new]
    fn new() -> PyResult<Self> {
        let config = bugbite::config::Config::new()?;
        Ok(Self(config))
    }

    /// Get a bugzilla service using a configured connection.
    fn bugzilla(&self, name: &str) -> PyResult<crate::bugzilla::Bugzilla> {
        bugbite::service::bugzilla::Bugzilla::config_builder(&self.0, Some(name))?.try_into()
    }

    /// Get a redmine service using a configured connection.
    fn redmine(&self, name: &str) -> PyResult<crate::redmine::Redmine> {
        bugbite::service::redmine::Redmine::config_builder(&self.0, Some(name))?.try_into()
    }

    fn __iter__(&self) -> PyResult<_Iter> {
        Ok(_Iter(self.0.services.clone().into_iter()))
    }

    fn __len__(&self) -> usize {
        self.0.services.len()
    }

    fn __contains__(&self, object: Py<PyAny>, py: Python<'_>) -> bool {
        if let Ok(name) = object.to_str_with_bound(py) {
            self.0.services.contains_key(name)
        } else {
            false
        }
    }

    fn __getitem__(&self, object: Py<PyAny>, py: Python<'_>) -> PyResult<service::Config> {
        if let Ok(name) = object.to_str_with_bound(py) {
            self.0
                .services
                .get(name)
                .map(|x| x.clone().into())
                .ok_or_else(|| PyKeyError::new_err(name.to_string()))
        } else {
            Err(PyTypeError::new_err("Config indices must be strings"))
        }
    }
}

#[pyclass]
struct _Iter(indexmap::map::IntoIter<String, bugbite::service::Config>);

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
