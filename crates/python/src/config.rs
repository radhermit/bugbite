use pyo3::prelude::*;

use crate::service;
use crate::Error;

#[pyclass(module = "bugbite.config")]
pub(super) struct Config(::bugbite::config::Config);

#[pymethods]
impl Config {
    #[new]
    fn new() -> PyResult<Self> {
        let config = ::bugbite::config::Config::new().map_err(Error)?;
        Ok(Self(config))
    }

    fn __iter__(&self) -> PyResult<_Iter> {
        Ok(_Iter(self.0.clone().into_iter()))
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
