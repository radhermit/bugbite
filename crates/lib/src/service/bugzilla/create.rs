use std::fs;

use camino::Utf8Path;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::objects::bugzilla::{Bug, Flag};
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a super::Service,
    params: Parameters,
}

impl RequestSend for Request<'_> {
    type Output = u64;

    async fn send(self) -> crate::Result<Self::Output> {
        let url = self.service.config.base.join("rest/bug")?;
        let params = self.params.encode(self.service)?;
        let request = self
            .service
            .client
            .post(url)
            .json(&params)
            .auth(self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        serde_json::from_value(data["id"].take())
            .map_err(|e| Error::InvalidValue(format!("failed deserializing id: {e}")))
    }
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a super::Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }

    pub fn params(mut self, params: Parameters) -> Self {
        self.params = params;
        self
    }

    pub fn alias<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.alias = Some(value.into_iter().map(Into::into).collect());
        self
    }

    pub fn assignee<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.assignee = Some(value.into());
        self
    }

    pub fn blocks<I>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        self.params.blocks = Some(value.into_iter().collect());
        self
    }

    pub fn cc<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.cc = Some(value.into_iter().map(Into::into).collect());
        self
    }

    pub fn component<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.component = Some(value.into());
        self
    }

    pub fn depends<I>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        self.params.depends = Some(value.into_iter().collect());
        self
    }

    pub fn description<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.description = Some(value.into());
        self
    }

    pub fn flags<I, T>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Flag>,
    {
        self.params.flags = Some(value.into_iter().map(Into::into).collect());
        self
    }

    pub fn groups<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.groups = Some(value.into_iter().map(Into::into).collect());
        self
    }

    pub fn keywords<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.keywords = Some(value.into_iter().map(Into::into).collect());
        self
    }

    pub fn os<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.os = Some(value.into());
        self
    }

    pub fn platform<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.platform = Some(value.into());
        self
    }

    pub fn priority<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.priority = Some(value.into());
        self
    }

    pub fn product<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.product = Some(value.into());
        self
    }

    pub fn qa<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.qa = Some(value.into());
        self
    }

    pub fn resolution<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.resolution = Some(value.into());
        self
    }

    pub fn see_also<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.see_also = Some(value.into_iter().map(Into::into).collect());
        self
    }

    pub fn severity<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.severity = Some(value.into());
        self
    }

    pub fn status<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.status = Some(value.into());
        self
    }

    pub fn summary<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.summary = Some(value.into());
        self
    }

    pub fn target<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.target = Some(value.into());
        self
    }

    pub fn url<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.url = Some(value.into());
        self
    }

    pub fn version<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.version = Some(value.into());
        self
    }

    pub fn whiteboard<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.whiteboard = Some(value.into());
        self
    }

    pub fn custom_fields<I, S1, S2>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: Into<String>,
        S2: Into<String>,
    {
        self.params.custom_fields = Some(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }
}

/// Bug creation parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Parameters {
    pub alias: Option<Vec<String>>,
    pub assignee: Option<String>,
    pub blocks: Option<Vec<u64>>,
    pub cc: Option<Vec<String>>,
    pub component: Option<String>,
    pub depends: Option<Vec<u64>>,
    pub description: Option<String>,
    pub flags: Option<Vec<Flag>>,
    pub groups: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub os: Option<String>,
    pub platform: Option<String>,
    pub priority: Option<String>,
    pub product: Option<String>,
    pub qa: Option<String>,
    pub resolution: Option<String>,
    pub see_also: Option<Vec<String>>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub target: Option<String>,
    pub url: Option<String>,
    pub version: Option<String>,
    pub whiteboard: Option<String>,

    #[serde(flatten)]
    pub custom_fields: Option<IndexMap<String, String>>,
}

impl Parameters {
    /// Load parameters in TOML format from a file.
    pub fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Merge parameters using the provided value for fallbacks.
    pub fn merge(self, other: Self) -> Self {
        Self {
            alias: self.alias.or(other.alias),
            assignee: self.assignee.or(other.assignee),
            blocks: self.blocks.or(other.blocks),
            cc: self.cc.or(other.cc),
            component: self.component.or(other.component),
            custom_fields: self.custom_fields.or(other.custom_fields),
            depends: self.depends.or(other.depends),
            description: self.description.or(other.description),
            flags: self.flags.or(other.flags),
            groups: self.groups.or(other.groups),
            keywords: self.keywords.or(other.keywords),
            os: self.os.or(other.os),
            platform: self.platform.or(other.platform),
            priority: self.priority.or(other.priority),
            product: self.product.or(other.product),
            qa: self.qa.or(other.qa),
            resolution: self.resolution.or(other.resolution),
            see_also: self.see_also.or(other.see_also),
            status: self.status.or(other.status),
            severity: self.severity.or(other.severity),
            target: self.target.or(other.target),
            summary: self.summary.or(other.summary),
            url: self.url.or(other.url),
            version: self.version.or(other.version),
            whiteboard: self.whiteboard.or(other.whiteboard),
        }
    }

    /// Encode parameters into the form required for the request.
    fn encode(self, service: &super::Service) -> crate::Result<RequestParameters> {
        let params = RequestParameters {
            // required fields with defaults
            op_sys: self.os.unwrap_or_else(|| "All".to_string()),
            platform: self.platform.unwrap_or_else(|| "All".to_string()),
            priority: self.priority.unwrap_or_else(|| "Normal".to_string()),
            severity: self.severity.unwrap_or_else(|| "normal".to_string()),
            version: self.version.unwrap_or_else(|| "unspecified".to_string()),

            // required fields without defaults
            component: self.component.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
            product: self.product.unwrap_or_default(),
            summary: self.summary.unwrap_or_default(),

            // optional fields
            alias: self.alias,
            assigned_to: self.assignee.map(|x| service.replace_user_alias(&x).into()),
            blocks: self.blocks,
            cc: self.cc,
            depends_on: self.depends,
            flags: self.flags,
            groups: self.groups,
            keywords: self.keywords,
            qa_contact: self.qa.map(|x| service.replace_user_alias(&x).into()),
            resolution: self.resolution,
            see_also: self.see_also,
            status: self.status,
            target_milestone: self.target,
            url: self.url,
            whiteboard: self.whiteboard,

            // auto-prefix custom field names
            custom_fields: self.custom_fields.map(|values| {
                values
                    .into_iter()
                    .map(|(k, v)| {
                        if !k.starts_with("cf_") {
                            (format!("cf_{k}"), v)
                        } else {
                            (k, v)
                        }
                    })
                    .collect()
            }),
        };

        // verify required fields are non-empty
        let mut missing = vec![];
        for (value, name) in [
            (&params.component, "component"),
            (&params.description, "description"),
            (&params.op_sys, "os"),
            (&params.platform, "platform"),
            (&params.priority, "priority"),
            (&params.product, "product"),
            (&params.severity, "severity"),
            (&params.summary, "summary"),
            (&params.version, "version"),
        ] {
            if value.is_empty() {
                missing.push(name);
            }
        }

        if !missing.is_empty() {
            let fields = missing.iter().sorted().join(", ");
            return Err(Error::InvalidRequest(format!(
                "missing required fields: {fields}"
            )));
        }

        Ok(params)
    }
}

impl From<Bug> for Parameters {
    fn from(value: Bug) -> Self {
        Self {
            component: value.component,
            os: value.op_sys,
            platform: value.platform,
            priority: value.priority,
            product: value.product,
            severity: value.severity,
            version: value.version,
            ..Default::default()
        }
    }
}

/// Internal bug creation request parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct RequestParameters {
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
