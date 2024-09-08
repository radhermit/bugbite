use bugbite::objects::bugzilla;
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use pyo3::types::{timezone_utc_bound, PyDateTime};

#[pyclass(module = "bugbite.bugzilla.objects")]
pub(super) struct Bug(bugzilla::Bug);

fn datetime(value: DateTime<Utc>, py: Python<'_>) -> Bound<'_, PyDateTime> {
    let value = value.timestamp() as f64;
    let tz = timezone_utc_bound(py);
    PyDateTime::from_timestamp_bound(py, value, Some(&tz)).unwrap()
}

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
}

impl From<bugzilla::Bug> for Bug {
    fn from(value: bugzilla::Bug) -> Self {
        Self(value)
    }
}

#[pymodule]
#[pyo3(name = "objects")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Bug>()?;
    Ok(())
}
