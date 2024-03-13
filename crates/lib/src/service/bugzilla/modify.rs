use std::num::NonZeroU64;

use serde::Serialize;
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

#[skip_serializing_none]
#[derive(Serialize, Debug, Default, Eq, PartialEq)]
struct Params {
    ids: Vec<NonZeroU64>,
    product: Option<String>,
    component: Option<String>,
    status: Option<String>,
    resolution: Option<String>,
    dupe_of: Option<NonZeroU64>,
}

/// Construct bug modification parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
pub struct ModifyParams {
    params: Params,
}

impl Default for ModifyParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ModifyParams {
    pub fn new() -> Self {
        Self {
            params: Params::default(),
        }
    }

    pub fn product(&mut self, value: &str) {
        self.params.product = Some(value.to_string());
    }

    pub fn component(&mut self, value: &str) {
        self.params.component = Some(value.to_string());
    }

    pub fn status(&mut self, value: &str) {
        self.params.status = Some(value.to_string());
    }

    pub fn resolution(&mut self, value: &str) {
        self.params.resolution = Some(value.to_string());
    }

    pub fn duplicate(&mut self, value: NonZeroU64) {
        self.params.dupe_of = Some(value);
    }

    fn build(self) -> crate::Result<Params> {
        if self.params == Params::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(self.params)
        }
    }
}
