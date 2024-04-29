use std::fs;

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::objects::bugzilla::{Bug, Flag};
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request {
    url: url::Url,
    params: Parameters,
}

impl RequestSend for Request {
    type Output = u64;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let params = self.params.encode(service)?;
        let request = service.client.post(self.url).json(&params).auth(service)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let id = serde_json::from_value(data["id"].take())
            .map_err(|e| Error::InvalidValue(format!("failed deserializing id: {e}")))?;
        Ok(id)
    }
}

impl Request {
    pub(super) fn new(service: &super::Service, params: Parameters) -> crate::Result<Self> {
        Ok(Self {
            url: service.config.base.join("rest/bug")?,
            params,
        })
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
            // inject defaults for required fields
            op_sys: self.os.unwrap_or_else(|| "All".to_string()),
            platform: self.platform.unwrap_or_else(|| "All".to_string()),
            priority: self.priority.unwrap_or_else(|| "Normal".to_string()),
            severity: self.severity.unwrap_or_else(|| "normal".to_string()),
            version: self.version.unwrap_or_else(|| "unspecified".to_string()),

            // error out on missing required fields
            component: self
                .component
                .ok_or_else(|| Error::InvalidValue("missing component".to_string()))?,
            description: self
                .description
                .ok_or_else(|| Error::InvalidValue("missing description".to_string()))?,
            product: self
                .product
                .ok_or_else(|| Error::InvalidValue("missing product".to_string()))?,
            summary: self
                .summary
                .ok_or_else(|| Error::InvalidValue("missing summary".to_string()))?,

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

        // TODO: verify all required fields are non-empty
        if params == RequestParameters::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(params)
        }
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
