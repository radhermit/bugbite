use std::pin::Pin;

use bugbite::objects::redmine;
use bugbite::service::redmine::search;
use bugbite::traits::RequestTemplate;
use futures_util::Stream;
use pyo3::prelude::*;

use crate::macros::stream_iterator;

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
        SearchIter(Box::pin(self.0.stream()))
    }

    fn load_template(&mut self, name: &str) -> PyResult<()> {
        self.0.load_template(name)?;
        Ok(())
    }

    fn save_template(&mut self, name: &str) -> PyResult<()> {
        self.0.save_template(name)?;
        Ok(())
    }

    fn subject(&mut self, value: &str) -> PyResult<()> {
        self.0.subject([value]);
        Ok(())
    }
}

#[pyclass(module = "bugbite.redmine")]
struct SearchIter(Pin<Box<dyn Stream<Item = bugbite::Result<redmine::Issue>> + Send>>);

stream_iterator!(SearchIter, Issue);
