use bugbite::error::python::BugbiteError;
use bugbite::service::bugzilla;
use bugbite::traits::RequestSend;
use bugbite::traits::WebClient;
use pyo3::prelude::*;

use crate::utils::tokio;

mod objects;
use objects::*;
mod search;

#[pyclass(module = "bugbite.bugzilla")]
pub(super) struct Bugzilla(pub(crate) bugzilla::Bugzilla);

impl TryFrom<bugbite::service::Config> for Bugzilla {
    type Error = PyErr;

    fn try_from(value: bugbite::service::Config) -> Result<Self, Self::Error> {
        let config = value
            .into_bugzilla()
            .map_err(|c| BugbiteError::new_err(format!("invalid service type: {}", c.kind())))?;
        let service = config.into_service()?;
        Ok(Self(service))
    }
}

#[pymethods]
impl Bugzilla {
    #[new]
    fn new(base: &str) -> PyResult<Self> {
        let service = bugzilla::Bugzilla::new(base)?;
        Ok(Self(service))
    }

    fn comment(&self, ids: Vec<String>) -> PyResult<Vec<Vec<Comment>>> {
        tokio().block_on(async {
            let comments = self.0.comment(ids).send().await?;
            Ok(comments
                .into_iter()
                .map(|x| x.into_iter().map(Into::into).collect())
                .collect())
        })
    }

    #[pyo3(signature = (ids, *, comments=None, history=None))]
    fn get(
        &self,
        ids: Vec<String>,
        comments: Option<bool>,
        history: Option<bool>,
    ) -> PyResult<Vec<Bug>> {
        tokio().block_on(async {
            let bugs = self
                .0
                .get(ids)
                .comments(comments.unwrap_or_default())
                .history(history.unwrap_or_default())
                .send()
                .await?;
            Ok(bugs.into_iter().map(Into::into).collect())
        })
    }

    fn search(&self) -> search::SearchRequest {
        self.0.search().into()
    }
}

#[pymodule]
#[pyo3(name = "bugzilla")]
pub(super) fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Bugzilla>()?;
    m.add_class::<Bug>()?;
    Ok(())
}
