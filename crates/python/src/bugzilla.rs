use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
use bugbite::traits::WebClient;
use pyo3::prelude::*;

use crate::error::{BugbiteError, Error};
use crate::utils::tokio;

mod objects;
use objects::Bug;

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Bugzilla(pub(crate) Service);

impl TryFrom<bugbite::service::Config> for Bugzilla {
    type Error = PyErr;

    fn try_from(value: bugbite::service::Config) -> Result<Self, Self::Error> {
        let config = value
            .into_bugzilla()
            .map_err(|c| BugbiteError::new_err(format!("invalid service type: {}", c.kind())))?;
        let service = Service::from_config(config).map_err(Error)?;
        Ok(crate::bugzilla::Bugzilla(service))
    }
}

#[pymethods]
impl Bugzilla {
    #[new]
    fn new(base: &str) -> PyResult<Self> {
        let service = Service::new(base).map_err(Error)?;
        Ok(Self(service))
    }

    fn search(&self, value: &str) -> PyResult<Vec<Bug>> {
        tokio().block_on(async {
            let bugs = self
                .0
                .search()
                .summary([value])
                .send()
                .await
                .map_err(Error)?;
            Ok(bugs.into_iter().map(Into::into).collect())
        })
    }
}

#[pymodule]
#[pyo3(name = "bugzilla")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Bugzilla>()?;
    Ok(())
}
