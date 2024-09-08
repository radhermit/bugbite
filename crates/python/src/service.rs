use bugbite::traits::WebClient;
use pyo3::prelude::*;

#[pyclass(module = "bugbite.service")]
pub(super) struct Config(::bugbite::service::Config);

impl From<::bugbite::service::Config> for Config {
    fn from(value: ::bugbite::service::Config) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Config {
    #[getter]
    fn name(&self) -> &str {
        self.0.name()
    }

    #[getter]
    fn base(&self) -> &str {
        self.0.base().as_str()
    }
}

#[pymodule]
#[pyo3(name = "service")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Config>()?;
    Ok(())
}
