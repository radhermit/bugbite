use std::fs;

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::objects::bugzilla::Flag;
use crate::traits::{InjectAuth, Request, ServiceParams, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CreateRequest<'a> {
    url: url::Url,
    params: Params,
    service: &'a super::Service,
}

impl Request for CreateRequest<'_> {
    type Output = u64;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .post(self.url)
            .json(&self.params)
            .inject_auth(self.service, true)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let id = serde_json::from_value(data["id"].take())
            .map_err(|e| Error::InvalidValue(format!("failed deserializing id: {e}")))?;
        Ok(id)
    }
}

impl<'a> CreateRequest<'a> {
    pub(super) fn new(service: &'a super::Service, params: CreateParams) -> crate::Result<Self> {
        Ok(Self {
            url: service.base().join("rest/bug")?,
            params: params.build()?,
            service,
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
    alias: Option<Vec<String>>,
    assigned_to: Option<String>,
    blocks: Option<Vec<u64>>,
    cc: Option<Vec<String>>,
    depends_on: Option<Vec<u64>>,
    flags: Option<Vec<Flag>>,
    groups: Option<Vec<String>>,
    ids: Option<Vec<u64>>,
    keywords: Option<Vec<String>>,
    qa_contact: Option<String>,
    resolution: Option<String>,
    see_also: Option<Vec<String>>,
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

    pub fn alias<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.alias = Some(values.into_iter().map(Into::into).collect());
    }

    pub fn assignee(&mut self, value: &str) {
        let user = self.service.replace_user_alias(value);
        self.params.assigned_to = Some(user.into());
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = u64>,
    {
        self.params.blocks = Some(values.into_iter().collect());
    }

    pub fn cc<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.cc = Some(values.into_iter().map(Into::into).collect());
    }

    pub fn component<S: Into<String>>(&mut self, value: S) {
        self.params.component = value.into();
    }

    pub fn depends<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = u64>,
    {
        self.params.depends_on = Some(values.into_iter().collect());
    }

    pub fn description<S: Into<String>>(&mut self, value: S) {
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

    pub fn flags<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Flag>,
    {
        self.params.flags = Some(values.into_iter().collect());
    }

    pub fn groups<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.groups = Some(values.into_iter().map(Into::into).collect());
    }

    pub fn keywords<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.keywords = Some(values.into_iter().map(Into::into).collect());
    }

    pub fn os<S: Into<String>>(&mut self, value: S) {
        self.params.op_sys = value.into();
    }

    pub fn platform<S: Into<String>>(&mut self, value: S) {
        self.params.platform = value.into();
    }

    pub fn priority<S: Into<String>>(&mut self, value: S) {
        self.params.priority = value.into();
    }

    pub fn product<S: Into<String>>(&mut self, value: S) {
        self.params.product = value.into();
    }

    pub fn qa(&mut self, value: &str) {
        let user = self.service.replace_user_alias(value);
        self.params.qa_contact = Some(user.into());
    }

    pub fn resolution<S: Into<String>>(&mut self, value: S) {
        self.params.resolution = Some(value.into());
    }

    pub fn see_also<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.see_also = Some(values.into_iter().map(Into::into).collect());
    }

    pub fn severity<S: Into<String>>(&mut self, value: S) {
        self.params.severity = value.into();
    }

    pub fn status<S: Into<String>>(&mut self, value: S) {
        self.params.status = Some(value.into());
    }

    pub fn summary<S: Into<String>>(&mut self, value: S) {
        self.params.summary = value.into();
    }

    pub fn target<S: Into<String>>(&mut self, value: S) {
        self.params.target_milestone = Some(value.into());
    }

    pub fn url<S: Into<String>>(&mut self, value: S) {
        self.params.url = Some(value.into());
    }

    pub fn version<S: Into<String>>(&mut self, value: S) {
        self.params.version = value.into();
    }

    pub fn whiteboard<S: Into<String>>(&mut self, value: S) {
        self.params.whiteboard = Some(value.into());
    }
}
