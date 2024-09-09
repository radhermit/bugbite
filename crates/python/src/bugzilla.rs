use bugbite::service::bugzilla::{self, GroupField};
use bugbite::traits::RequestSend;
use bugbite::traits::WebClient;
use pyo3::prelude::*;

use crate::error::{BugbiteError, Error};
use crate::utils::tokio;

mod objects;
use objects::Bug;

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Bugzilla(pub(crate) bugzilla::Bugzilla);

impl TryFrom<bugbite::service::Config> for Bugzilla {
    type Error = PyErr;

    fn try_from(value: bugbite::service::Config) -> Result<Self, Self::Error> {
        let config = value
            .into_bugzilla()
            .map_err(|c| BugbiteError::new_err(format!("invalid service type: {}", c.kind())))?;
        let service = config.into_service().map_err(Error)?;
        Ok(Self(service))
    }
}

#[pymethods]
impl Bugzilla {
    #[new]
    fn new(base: &str) -> PyResult<Self> {
        let service = bugzilla::Bugzilla::new(base).map_err(Error)?;
        Ok(Self(service))
    }

    fn search(&self, value: &str) -> PyResult<Vec<Bug>> {
        tokio().block_on(async {
            let bugs = self
                .0
                .search()
                .fields([GroupField::Default])
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
    m.add_class::<Bug>()?;
    Ok(())
}
