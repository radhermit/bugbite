use std::pin::Pin;

use bugbite::service::redmine;
use bugbite::traits::RequestStream;
use bugbite::traits::WebClient;
use futures_util::{Stream, TryStreamExt};
use pyo3::prelude::*;

use crate::error::{BugbiteError, Error};
use crate::utils::tokio;

mod objects;
use objects::Issue;

#[pyclass(module = "bugbite.redmine")]
pub(super) struct Redmine(pub(crate) redmine::Redmine);

impl TryFrom<bugbite::service::Config> for Redmine {
    type Error = PyErr;

    fn try_from(value: bugbite::service::Config) -> Result<Self, Self::Error> {
        let config = value
            .into_redmine()
            .map_err(|c| BugbiteError::new_err(format!("invalid service type: {}", c.kind())))?;
        let service = config.into_service().map_err(Error)?;
        Ok(Self(service))
    }
}

#[pymethods]
impl Redmine {
    #[new]
    fn new(base: &str) -> PyResult<Self> {
        let service = redmine::Redmine::new(base).map_err(Error)?;
        Ok(Self(service))
    }

    fn search(&self, value: &str) -> SearchIter {
        let stream = self.0.search().subject([value]).stream();
        SearchIter(Box::pin(stream))
    }
}

#[pyclass(module = "bugbite.redmine")]
struct SearchIter(
    Pin<Box<dyn Stream<Item = bugbite::Result<bugbite::objects::redmine::Issue>> + Send>>,
);

#[pymethods]
impl SearchIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyResult<Issue>> {
        tokio().block_on(async {
            match self.0.try_next().await {
                Err(e) => Some(Err(Error(e).into())),
                Ok(v) => v.map(|x| Ok(x.into())),
            }
        })
    }
}

#[pymodule]
#[pyo3(name = "redmine")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Redmine>()?;
    m.add_class::<Issue>()?;
    Ok(())
}
