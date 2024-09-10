use bugbite::objects::bugzilla;
use pyo3::prelude::*;
use pyo3::types::{PyDateTime, PyFrozenSet};

use crate::utils::datetime;

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Bug(bugzilla::Bug);

#[pymethods]
impl Bug {
    #[getter]
    fn id(&self) -> u64 {
        self.0.id
    }

    #[getter]
    fn assigned_to(&self) -> Option<&str> {
        self.0.assigned_to.as_deref()
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
    fn creator(&self) -> Option<&str> {
        self.0.creator.as_deref()
    }

    #[getter]
    fn summary(&self) -> Option<&str> {
        self.0.summary.as_deref()
    }

    #[getter]
    fn status(&self) -> Option<&str> {
        self.0.status.as_deref()
    }

    #[getter]
    fn resolution(&self) -> Option<&str> {
        self.0.resolution.as_deref()
    }

    #[getter]
    fn whiteboard(&self) -> Option<&str> {
        self.0.whiteboard.as_deref()
    }

    #[getter]
    fn product(&self) -> Option<&str> {
        self.0.product.as_deref()
    }

    #[getter]
    fn component(&self) -> Option<&str> {
        self.0.component.as_deref()
    }

    #[getter]
    fn version(&self) -> Option<&str> {
        self.0.version.as_deref()
    }

    #[getter]
    fn platform(&self) -> Option<&str> {
        self.0.platform.as_deref()
    }

    #[getter]
    fn op_sys(&self) -> Option<&str> {
        self.0.op_sys.as_deref()
    }

    #[getter]
    fn target(&self) -> Option<&str> {
        self.0.target.as_deref()
    }

    #[getter]
    fn priority(&self) -> Option<&str> {
        self.0.priority.as_deref()
    }

    #[getter]
    fn severity(&self) -> Option<&str> {
        self.0.severity.as_deref()
    }

    #[getter]
    fn url(&self) -> Option<&str> {
        self.0.url.as_deref()
    }

    #[getter]
    fn duplicate_of(&self) -> Option<u64> {
        self.0.duplicate_of
    }

    #[getter]
    fn blocks<'a>(&self, py: Python<'a>) -> Bound<'a, PyFrozenSet> {
        PyFrozenSet::new_bound(py, &self.0.blocks).unwrap()
    }

    #[getter]
    fn depends_on<'a>(&self, py: Python<'a>) -> Bound<'a, PyFrozenSet> {
        PyFrozenSet::new_bound(py, &self.0.depends_on).unwrap()
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
}

impl From<bugzilla::Bug> for Bug {
    fn from(value: bugzilla::Bug) -> Self {
        Self(value)
    }
}

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Comment(bugzilla::Comment);

#[pymethods]
impl Comment {
    #[getter]
    fn id(&self) -> u64 {
        self.0.id
    }

    #[getter]
    fn bug_id(&self) -> u64 {
        self.0.bug_id
    }

    #[getter]
    fn attachment_id(&self) -> Option<u64> {
        self.0.attachment_id
    }

    #[getter]
    fn count(&self) -> usize {
        self.0.count
    }

    #[getter]
    fn text(&self) -> &str {
        &self.0.text
    }

    #[getter]
    fn creator(&self) -> &str {
        &self.0.creator
    }

    #[getter]
    fn created<'a>(&self, py: Python<'a>) -> Bound<'a, PyDateTime> {
        datetime(self.0.created, py)
    }

    #[getter]
    fn is_private(&self) -> bool {
        self.0.is_private
    }
}

impl From<bugzilla::Comment> for Comment {
    fn from(value: bugzilla::Comment) -> Self {
        Self(value)
    }
}