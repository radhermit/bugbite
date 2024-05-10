use ::bugbite::config;
use pyo3::prelude::*;

use crate::Error;

#[pyclass(module = "bugbite.config")]
pub(super) struct Config(config::Config);

#[pymethods]
impl Config {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let config = config::Config::load(path).map_err(Error)?;
        Ok(Self(config))
    }

    #[getter]
    fn connections(&self) -> Vec<Connection> {
        self.0
            .connections()
            .iter()
            .cloned()
            .map(Connection)
            .collect()
    }
}

#[pyclass(module = "bugbite.config")]
pub(super) struct Connection(config::Connection);

#[pymethods]
impl Connection {
    #[getter]
    fn name(&self) -> &str {
        self.0.name()
    }

    #[getter]
    fn base(&self) -> &str {
        self.0.base()
    }
}

#[pymodule]
#[pyo3(name = "config")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Config>()?;
    Ok(())
}
