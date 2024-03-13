use std::fs;
use std::num::NonZeroU64;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct ModifyRequest {
    url: url::Url,
    params: Params,
}

impl Request for ModifyRequest {
    type Output = ();
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().put(self.url).json(&self.params);
        let response = service.send(request).await?;
        let mut data = service.parse_response(response).await?;
        let _data = data["bugs"].take();
        Ok(())
    }
}

impl ModifyRequest {
    pub(super) fn new(
        service: &super::Service,
        ids: &[NonZeroU64],
        params: ModifyParams,
    ) -> crate::Result<Self> {
        let [id, ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut params = params.build()?;
        params.ids = ids.to_vec();

        Ok(Self {
            url: service.base().join(&format!("rest/bug/{id}"))?,
            params,
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct Comment {
    body: String,
    is_private: bool,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct Params {
    ids: Vec<NonZeroU64>,
    product: Option<String>,
    component: Option<String>,
    comment: Option<Comment>,
    status: Option<String>,
    resolution: Option<String>,
    dupe_of: Option<NonZeroU64>,
    summary: Option<String>,
}

/// Construct bug modification parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
pub struct ModifyParams(Params);

impl Default for ModifyParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ModifyParams {
    pub fn new() -> Self {
        Self(Params::default())
    }

    pub fn load(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        let params = toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))?;
        Ok(Self(params))
    }

    pub fn product(&mut self, value: &str) {
        self.0.product = Some(value.to_string());
    }

    pub fn component(&mut self, value: &str) {
        self.0.component = Some(value.to_string());
    }

    pub fn status(&mut self, value: &str) {
        self.0.status = Some(value.to_string());
    }

    pub fn resolution(&mut self, value: &str) {
        self.0.resolution = Some(value.to_string());
    }

    pub fn duplicate(&mut self, value: NonZeroU64) {
        self.0.dupe_of = Some(value);
    }

    pub fn summary(&mut self, value: &str) {
        self.0.summary = Some(value.to_string());
    }

    pub fn comment(&mut self, value: &str) {
        let comment = Comment {
            body: value.to_string(),
            is_private: false,
        };
        self.0.comment = Some(comment);
    }

    fn build(self) -> crate::Result<Params> {
        if self.0 == Params::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(self.0)
        }
    }
}
