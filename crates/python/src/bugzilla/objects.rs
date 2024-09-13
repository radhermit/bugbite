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
    fn created<'a>(&self, py: Python<'a>) -> PyResult<Option<Bound<'a, PyDateTime>>> {
        self.0.created.map(|x| datetime(x, py)).transpose()
    }

    #[getter]
    fn updated<'a>(&self, py: Python<'a>) -> PyResult<Option<Bound<'a, PyDateTime>>> {
        self.0.updated.map(|x| datetime(x, py)).transpose()
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
    fn groups<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.groups)
    }

    #[getter]
    fn keywords<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.keywords)
    }

    #[getter]
    fn cc<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.cc)
    }

    #[getter]
    fn tags<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.tags)
    }

    #[getter]
    fn see_also<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.see_also)
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
    fn blocks<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.blocks)
    }

    #[getter]
    fn depends_on<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyFrozenSet>> {
        PyFrozenSet::new_bound(py, &self.0.depends_on)
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

    #[getter]
    fn history(&self) -> Vec<Event> {
        self.0.history.clone().into_iter().map(Into::into).collect()
    }

    // TODO: switch to using str pyclass parameter when >=pyo3-0.23
    fn __str__(&self) -> String {
        self.0.to_string()
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
    fn created<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDateTime>> {
        datetime(self.0.created, py)
    }

    #[getter]
    fn is_private(&self) -> bool {
        self.0.is_private
    }

    // TODO: switch to using str pyclass parameter when >=pyo3-0.23
    fn __str__(&self) -> String {
        self.0.to_string()
    }
}

impl From<bugzilla::Comment> for Comment {
    fn from(value: bugzilla::Comment) -> Self {
        Self(value)
    }
}

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Event(bugzilla::Event);

#[pymethods]
impl Event {
    #[getter]
    fn who(&self) -> &str {
        &self.0.who
    }

    #[getter]
    fn when<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDateTime>> {
        datetime(self.0.when, py)
    }

    #[getter]
    fn changes(&self) -> Vec<Change> {
        self.0.changes.clone().into_iter().map(Into::into).collect()
    }

    // TODO: switch to using str pyclass parameter when >=pyo3-0.23
    fn __str__(&self) -> String {
        self.0.to_string()
    }
}

impl From<bugzilla::Event> for Event {
    fn from(value: bugzilla::Event) -> Self {
        Self(value)
    }
}

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Change(bugzilla::Change);

#[pymethods]
impl Change {
    #[getter]
    fn field_name(&self) -> &str {
        &self.0.field_name
    }

    #[getter]
    fn removed(&self) -> Option<&str> {
        self.0.removed.as_deref()
    }

    #[getter]
    fn added(&self) -> Option<&str> {
        self.0.added.as_deref()
    }

    #[getter]
    fn attachment_id(&self) -> Option<u64> {
        self.0.attachment_id
    }
}

impl From<bugzilla::Change> for Change {
    fn from(value: bugzilla::Change) -> Self {
        Self(value)
    }
}
