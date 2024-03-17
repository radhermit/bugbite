use std::fs;
use std::num::NonZeroU64;

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::traits::{InjectAuth, Request, ServiceParams, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CreateRequest {
    url: url::Url,
    params: Params,
}

impl Request for CreateRequest {
    type Output = NonZeroU64;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service
            .client()
            .post(self.url)
            .json(&self.params)
            .inject_auth(service, true)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        Ok(serde_json::from_value(data["id"].take())?)
    }
}

impl CreateRequest {
    pub(super) fn new(service: &super::Service, params: CreateParams) -> crate::Result<Self> {
        Ok(Self {
            url: service.base().join("rest/bug")?,
            params: params.build()?,
        })
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct Params {
    // required fields
    component: String,
    description: String,
    op_sys: String,
    platform: String,
    priority: String,
    product: String,
    severity: String,
    summary: String,
    version: String,

    // optional fields
    alias: Option<String>,
    assigned_to: Option<String>,
    blocks: Option<Vec<NonZeroU64>>,
    cc: Option<String>,
    depends_on: Option<Vec<NonZeroU64>>,
    groups: Option<String>,
    ids: Option<Vec<NonZeroU64>>,
    keywords: Option<String>,
    resolution: Option<String>,
    see_also: Option<String>,
    status: Option<String>,
    target_milestone: Option<String>,
    url: Option<String>,
    whiteboard: Option<String>,

    #[serde(flatten)]
    custom_fields: Option<IndexMap<String, String>>,
}

/// Construct bug modification parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
pub struct CreateParams<'a> {
    service: &'a super::Service,
    params: Params,
}

impl<'a> ServiceParams<'a> for CreateParams<'a> {
    type Service = super::Service;

    fn new(service: &'a Self::Service) -> Self {
        Self {
            service,
            params: Params {
                op_sys: "All".to_string(),
                platform: "All".to_string(),
                priority: "Normal".to_string(),
                severity: "normal".to_string(),
                version: "unspecified".to_string(),
                ..Default::default()
            },
        }
    }
}

impl<'a> CreateParams<'a> {
    pub fn load(path: &Utf8Path, service: &'a super::Service) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        let params = toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))?;
        Ok(Self { service, params })
    }

    fn build(self) -> crate::Result<Params> {
        // TODO: verify all required fields are non-empty
        if self.params == Params::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(self.params)
        }
    }

    pub fn alias(&mut self, value: &str) {
        self.params.alias = Some(value.into());
    }

    pub fn assigned_to(&mut self, value: &str) {
        // TODO: support pulling aliases from the config?
        if value == "@me" {
            if let Some(user) = self.service.user() {
                self.params.assigned_to = Some(user.into());
            }
        } else {
            self.params.assigned_to = Some(value.into());
        }
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = NonZeroU64>,
    {
        self.params.blocks = Some(values.into_iter().collect());
    }

    pub fn cc<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.params.cc = Some(values.into_iter().collect());
    }

    pub fn component(&mut self, value: &str) {
        self.params.component = value.into();
    }

    pub fn depends_on<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = NonZeroU64>,
    {
        self.params.depends_on = Some(values.into_iter().collect());
    }

    pub fn description(&mut self, value: &str) {
        self.params.description = value.into();
    }

    pub fn custom_fields<I, K, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        self.params.custom_fields = Some(
            values
                .into_iter()
                .map(|(k, v)| match k.as_ref() {
                    k if k.starts_with("cf_") => (k.into(), v.into()),
                    k => (format!("cf_{k}"), v.into()),
                })
                .collect(),
        );
    }

    pub fn groups<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.params.groups = Some(values.into_iter().collect());
    }

    pub fn keywords<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.params.keywords = Some(values.into_iter().collect());
    }

    pub fn os(&mut self, value: &str) {
        self.params.op_sys = value.into();
    }

    pub fn platform(&mut self, value: &str) {
        self.params.platform = value.into();
    }

    pub fn priority(&mut self, value: &str) {
        self.params.priority = value.into();
    }

    pub fn product(&mut self, value: &str) {
        self.params.product = value.into();
    }

    pub fn resolution(&mut self, value: &str) {
        self.params.resolution = Some(value.into());
    }

    pub fn see_also<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.params.see_also = Some(values.into_iter().collect());
    }

    pub fn severity(&mut self, value: &str) {
        self.params.severity = value.into();
    }

    pub fn status(&mut self, value: &str) {
        self.params.status = Some(value.into());
    }

    pub fn summary(&mut self, value: &str) {
        self.params.summary = value.into();
    }

    pub fn target(&mut self, value: &str) {
        self.params.target_milestone = Some(value.into());
    }

    pub fn url(&mut self, value: &str) {
        self.params.url = Some(value.into());
    }

    pub fn version(&mut self, value: &str) {
        self.params.version = value.into();
    }

    pub fn whiteboard(&mut self, value: &str) {
        self.params.whiteboard = Some(value.into());
    }
}
