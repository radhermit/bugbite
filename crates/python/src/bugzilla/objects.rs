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
