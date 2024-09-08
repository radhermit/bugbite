use bugbite::service::redmine::Service;
use bugbite::traits::RequestSend;
use bugbite::traits::WebClient;
use pyo3::prelude::*;

use crate::error::{BugbiteError, Error};
use crate::utils::tokio;

mod objects;
use objects::Issue;

#[pyclass(module = "bugbite.redmine")]
pub(super) struct Redmine(pub(crate) Service);

impl TryFrom<bugbite::service::Config> for Redmine {
    type Error = PyErr;

    fn try_from(value: bugbite::service::Config) -> Result<Self, Self::Error> {
        let config = value
            .into_redmine()
            .map_err(|c| BugbiteError::new_err(format!("invalid service type: {}", c.kind())))?;
        let service = Service::from_config(config).map_err(Error)?;
        Ok(Self(service))
    }
}

#[pymethods]
impl Redmine {
    #[new]
    fn new(base: &str) -> PyResult<Self> {
        let service = Service::new(base).map_err(Error)?;
        Ok(Self(service))
    }

    fn search(&self, value: &str) -> PyResult<Vec<Issue>> {
        tokio().block_on(async {
            let issues = self
                .0
                .search()
                .subject([value])
                .send()
                .await
                .map_err(Error)?;
            Ok(issues.into_iter().map(Into::into).collect())
        })
    }
}

#[pymodule]
#[pyo3(name = "redmine")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Redmine>()?;
    Ok(())
}
