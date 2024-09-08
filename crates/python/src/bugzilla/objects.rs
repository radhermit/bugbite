use bugbite::objects::bugzilla;
use pyo3::prelude::*;

#[pyclass(module = "bugbite.bugzilla.objects")]
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
