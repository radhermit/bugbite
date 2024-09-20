use std::pin::Pin;

use bugbite::objects::bugzilla;
use bugbite::service::bugzilla::search;
use bugbite::traits::RequestTemplate;
use futures_util::Stream;
use itertools::Itertools;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyIterator};

use crate::macros::stream_iterator;
use crate::traits::ToStr;

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

    pub(super) fn alias(&mut self, value: Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(value) = value.to_str() {
            self.0.alias(value);
        } else if let Ok(value) = value.downcast::<PyBool>() {
            self.0.alias(value.is_true());
        } else if let Ok(values) = value.downcast::<PyIterator>() {
            let values: Vec<_> = values
                .iter()?
                .filter_map(|x| x.ok())
                .map(|x| x.to_str_owned())
                .try_collect()?;
            self.0.alias(values);
        } else {
            return Err(PyTypeError::new_err(format!(
                "invalid alias value: {value:?}"
            )));
        }
        Ok(())
    }

    pub(super) fn created(&mut self, value: &str) -> PyResult<()> {
        self.0.created(value.parse()?);
        Ok(())
    }

    pub(super) fn updated(&mut self, value: &str) -> PyResult<()> {
        self.0.updated(value.parse()?);
        Ok(())
    }

    pub(super) fn closed(&mut self, value: &str) -> PyResult<()> {
        self.0.closed(value.parse()?);
        Ok(())
    }

    pub(super) fn summary(&mut self, value: &str) -> PyResult<()> {
        self.0.summary([value]);
        Ok(())
    }
}

#[pyclass(module = "bugbite.bugzilla")]
struct SearchIter(Pin<Box<dyn Stream<Item = bugbite::Result<bugzilla::Bug>> + Send>>);

stream_iterator!(SearchIter, Bug);
