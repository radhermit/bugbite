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
}

impl From<redmine::Issue> for Issue {
    fn from(value: redmine::Issue) -> Self {
        Self(value)
    }
}
