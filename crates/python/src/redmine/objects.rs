use bugbite::objects::redmine;
use pyo3::prelude::*;
use pyo3::types::PyDateTime;

use crate::utils::datetime;

#[pyclass(module = "bugbite.redmine")]
pub(super) struct Issue(redmine::Issue);

#[pymethods]
impl Issue {
    #[getter]
    fn id(&self) -> u64 {
        self.0.id
    }

    #[getter]
    fn subject(&self) -> Option<&str> {
        self.0.subject.as_deref()
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    #[getter]
    fn priority(&self) -> Option<&str> {
        self.0.priority.as_deref()
    }

    #[getter]
    fn status(&self) -> Option<&str> {
        self.0.status.as_deref()
    }

    #[getter]
    fn tracker(&self) -> Option<&str> {
        self.0.tracker.as_deref()
    }

    #[getter]
    fn closed<'a>(&self, py: Python<'a>) -> Option<Bound<'a, PyDateTime>> {
        self.0.closed.map(|x| datetime(x, py))
    }

    #[getter]
    fn created<'a>(&self, py: Python<'a>) -> Option<Bound<'a, PyDateTime>> {
        self.0.created.map(|x| datetime(x, py))
    }

    #[getter]
    fn updated<'a>(&self, py: Python<'a>) -> Option<Bound<'a, PyDateTime>> {
        self.0.updated.map(|x| datetime(x, py))
    }

    #[getter]
    fn comments(&self) -> Vec<Comment> {
        self.0
            .comments
            .clone()
            .into_iter()
            .map(Into::into)
            .collect()
    }

    // TODO: switch to using str pyclass parameter when >=pyo3-0.23
    fn __str__(&self) -> String {
        self.0.to_string()
    }
}

impl From<redmine::Issue> for Issue {
    fn from(value: redmine::Issue) -> Self {
        Self(value)
    }
}

#[pyclass(module = "bugbite.redmine")]
pub(super) struct Comment(redmine::Comment);

#[pymethods]
impl Comment {
    #[getter]
    fn count(&self) -> u64 {
        self.0.count
    }

    #[getter]
    fn text(&self) -> &str {
        &self.0.text
    }

    #[getter]
    fn user(&self) -> String {
        self.0.user.to_string()
    }

    #[getter]
    fn created<'a>(&self, py: Python<'a>) -> Bound<'a, PyDateTime> {
        datetime(self.0.created, py)
    }
}

impl From<redmine::Comment> for Comment {
    fn from(value: redmine::Comment) -> Self {
        Self(value)
    }
}
