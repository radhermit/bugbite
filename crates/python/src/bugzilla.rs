use std::pin::Pin;

use bugbite::service::bugzilla;
use bugbite::traits::WebClient;
use bugbite::traits::{RequestSend, RequestStream};
use futures_util::{Stream, TryStreamExt};
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

    fn get(&self, ids: Vec<String>) -> PyResult<Vec<Bug>> {
        tokio().block_on(async {
            let bugs = self.0.get(ids).send().await.map_err(Error)?;
            Ok(bugs.into_iter().map(Into::into).collect())
        })
    }

    fn search(&self) -> SearchRequest {
        SearchRequest(self.0.search())
    }
}

#[pyclass(module = "bugbite.bugzilla")]
struct SearchRequest(bugbite::service::bugzilla::search::Request);

#[pymethods]
impl SearchRequest {
    fn __iter__(&self) -> SearchIter {
        SearchIter(Box::pin(self.0.clone().stream()))
    }

    fn summary(&mut self, value: &str) {
        self.0.params.summary = Some(vec![value.into()]);
    }
}

#[pyclass(module = "bugbite.bugzilla")]
struct SearchIter(
    Pin<Box<dyn Stream<Item = bugbite::Result<bugbite::objects::bugzilla::Bug>> + Send>>,
);

#[pymethods]
impl SearchIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyResult<Bug>> {
        tokio().block_on(async {
            match self.0.try_next().await {
                Err(e) => Some(Err(Error(e).into())),
                Ok(v) => v.map(|x| Ok(x.into())),
            }
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
