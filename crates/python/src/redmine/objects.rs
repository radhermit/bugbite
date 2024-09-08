use bugbite::objects::redmine;
use pyo3::prelude::*;

#[pyclass(module = "bugbite.redmine.objects")]
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
}

impl From<redmine::Issue> for Issue {
    fn from(value: redmine::Issue) -> Self {
        Self(value)
    }
}

#[pymodule]
#[pyo3(name = "redmine_objects")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Issue>()?;
    Ok(())
}
