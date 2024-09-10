use std::pin::Pin;

use bugbite::objects::bugzilla;
use bugbite::service::bugzilla::search;
use bugbite::traits::{RequestStream, RequestTemplate};
use futures_util::Stream;
use pyo3::prelude::*;

use crate::error::Error;
use crate::macros::stream_iterator;

use super::Bug;

#[pyclass(module = "bugbite.bugzilla")]
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

    fn load_template(&mut self, name: &str) -> PyResult<()> {
        self.0.load_template(name).map_err(Error)?;
        Ok(())
    }

    fn save_template(&mut self, name: &str) -> PyResult<()> {
        self.0.save_template(name).map_err(Error)?;
        Ok(())
    }

    fn summary(&mut self, value: &str) {
        self.0.params.summary = Some(vec![value.into()]);
    }
}

#[pyclass(module = "bugbite.bugzilla")]
struct SearchIter(Pin<Box<dyn Stream<Item = bugbite::Result<bugzilla::Bug>> + Send>>);

stream_iterator!(SearchIter, Bug);
