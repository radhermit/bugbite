use std::pin::Pin;

use bugbite::service::redmine::search;
use bugbite::traits::RequestStream;
use futures_util::{Stream, TryStreamExt};
use pyo3::prelude::*;

use crate::error::Error;
use crate::utils::tokio;

use super::Issue;

#[pyclass(module = "bugbite.redmine")]
pub(super) struct SearchRequest(search::Request);

impl From<search::Request> for SearchRequest {
    fn from(value: search::Request) -> Self {
        Self(value)
    }
}

#[pymethods]
impl SearchRequest {
    fn __iter__(&self) -> SearchIter {
        SearchIter(Box::pin(self.0.clone().stream()))
    }

    fn subject(&mut self, value: &str) {
        self.0.params.subject = Some(vec![value.into()]);
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
