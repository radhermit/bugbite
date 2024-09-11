use bugbite::error::python::BugbiteError;
use bugbite::service::redmine;
use bugbite::traits::RequestSend;
use bugbite::traits::WebClient;
use pyo3::prelude::*;

use crate::utils::tokio;

mod objects;
use objects::*;
mod search;

#[pyclass(module = "bugbite.redmine")]
pub(super) struct Redmine(pub(crate) redmine::Redmine);

impl TryFrom<bugbite::service::Config> for Redmine {
    type Error = PyErr;

    fn try_from(value: bugbite::service::Config) -> Result<Self, Self::Error> {
        let config = value
            .into_redmine()
            .map_err(|c| BugbiteError::new_err(format!("invalid service type: {}", c.kind())))?;
        let service = config.into_service()?;
        Ok(Self(service))
    }
}

#[pymethods]
impl Redmine {
    #[new]
    fn new(base: &str) -> PyResult<Self> {
        let service = redmine::Redmine::new(base)?;
        Ok(Self(service))
    }

    #[pyo3(signature = (ids, *, comments=None))]
    fn get(&self, ids: Vec<u64>, comments: Option<bool>) -> PyResult<Vec<Issue>> {
        tokio().block_on(async {
            let issues = self
                .0
                .get(ids)
                .comments(comments.unwrap_or_default())
                .send()
                .await?;
            Ok(issues.into_iter().map(Into::into).collect())
        })
    }

    fn search(&self) -> search::SearchRequest {
        self.0.search().into()
    }
}

#[pymodule]
#[pyo3(name = "redmine")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Redmine>()?;
    m.add_class::<Issue>()?;
    Ok(())
}
