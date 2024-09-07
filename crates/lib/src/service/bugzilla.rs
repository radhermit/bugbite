use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use indexmap::{IndexMap, IndexSet};
use once_cell::sync::Lazy;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::objects::bugzilla::{BugzillaField, BugzillaFieldName};
use crate::traits::{Api, Merge, MergeOption, WebClient, WebService};
use crate::Error;

use super::{ClientParameters, ServiceKind};

pub mod attachment;
pub mod comment;
pub mod create;
pub mod fields;
mod get;
pub mod history;
pub mod search;
pub mod update;
pub mod version;

/// Common default values used for unset fields.
pub(crate) static UNSET_VALUES: Lazy<HashSet<String>> = Lazy::new(|| {
    ["unspecified", "Unspecified", "---", "--", "-", ""]
        .iter()
        .map(|s| s.to_string())
        .collect()
});

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Authentication {
    pub key: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Merge for Authentication {
    fn merge(&mut self, other: Self) {
        *self = Self {
            key: self.key.merge(other.key),
            user: self.user.merge(other.user),
            password: self.password.merge(other.password),
        }
    }
}

// TODO: improve API for setting user info on config creation
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    pub name: String,
    #[serde(flatten)]
    pub auth: Authentication,
    #[serde(flatten)]
    pub client: ClientParameters,
    pub max_search_results: Option<usize>,
}

impl Config {
    pub(super) fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            name: Default::default(),
            auth: Default::default(),
            client: Default::default(),
            max_search_results: Default::default(),
        })
    }

    /// Maximum number of results that can be returned by a search request.
    ///
    /// Fallback to bugzilla's internal default of 10000.
    fn max_search_results(&self) -> usize {
        match self.max_search_results.unwrap_or_default() {
            0 => 10000,
            n => n,
        }
    }
}

impl WebClient for Config {
    fn base(&self) -> &Url {
        &self.base
    }

    fn kind(&self) -> ServiceKind {
        ServiceKind::Bugzilla
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
pub struct Service {
    pub config: Config,
    cache: ServiceCache,
    client: reqwest::Client,
}

impl Service {
    /// Create a new Service from a given base URL.
    pub fn new(base: &str) -> crate::Result<Self> {
        let config = Config::new(base)?;
        let client = config.client.build()?;
        Ok(Self {
            config,
            cache: Default::default(),
            client,
        })
    }

    /// Create a new Service from a Config.
    pub fn from_config(config: Config) -> crate::Result<Self> {
        let client = config.client.build()?;
        Ok(Self {
            config,
            cache: Default::default(),
            client,
        })
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: fmt::Display>(&self, id: I) -> String {
        let base = self.base().as_str().trim_end_matches('/');
        format!("{base}/show_bug.cgi?id={id}")
    }

    fn deserialize_custom_fields(
        &self,
        data: &mut serde_json::Value,
    ) -> IndexMap<BugzillaFieldName, String> {
        let mut custom_fields = IndexMap::new();

        if let Some(map) = data.as_object_mut() {
            for field in &self.cache.custom_fields {
                let Some(value) = map.remove(&field.name.id) else {
                    continue;
                };

                // TODO: handle different custom field value types
                let serde_json::Value::String(value) = value else {
                    continue;
                };

                if !UNSET_VALUES.contains(&value) {
                    custom_fields.insert(field.name.clone(), value);
                }
            }
        }

        custom_fields
    }

    /// Substitute user alias for matching value.
    // TODO: support pulling aliases from the config?
    fn replace_user_alias<'a>(&'a self, value: &'a str) -> &'a str {
        if value == "@me" {
            self.config.auth.user.as_deref().unwrap_or(value)
        } else {
            value
        }
    }

    pub fn attachment_create<I, S>(&self, ids: I) -> attachment::create::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        attachment::create::Request::new(self, ids)
    }

    pub fn attachment_get<I>(&self, ids: I) -> attachment::get::Request
    where
        I: IntoIterator<Item = u64>,
    {
        attachment::get::Request::new(self, ids)
    }

    pub fn attachment_get_item<I, S>(&self, ids: I) -> attachment::get_item::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        attachment::get_item::Request::new(self, ids)
    }

    pub fn attachment_update<I>(&self, ids: I) -> attachment::update::Request
    where
        I: IntoIterator<Item = u64>,
    {
        attachment::update::Request::new(self, ids)
    }

    pub fn comment<I, S>(&self, ids: I) -> comment::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        comment::Request::new(self, ids)
    }

    pub fn create(&self) -> create::Request {
        create::Request::new(self)
    }

    pub fn fields(&self) -> fields::Request {
        fields::Request::new(self)
    }

    pub fn get<I, S>(&self, ids: I) -> get::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        get::Request::new(self, ids)
    }

    pub fn history<I, S>(&self, ids: I) -> history::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        history::Request::new(self, ids)
    }

    pub fn search(&self) -> search::Request {
        search::Request::new(self)
    }

    pub fn update<I, S>(&self, ids: I) -> update::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        update::Request::new(self, ids)
    }

    pub fn version(&self) -> version::Request {
        version::Request::new(self)
    }
}

/// Return a bugzilla error if one is returned in the response data.
macro_rules! return_if_error {
    ($data:expr) => {{
        if $data.get("error").is_some() {
            let code = $data["code"].as_i64().unwrap_or_default();
            let message = if let Some(value) = $data["message"].as_str() {
                value.to_string()
            } else {
                format!("unknown error: {code}")
            };
            return Err(Error::Bugzilla { code, message });
        }
    }};
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -- {}", self.kind(), self.base())
    }
}

impl<'a> WebService<'a> for Service {
    const API_VERSION: &'static str = "v1";
    type Response = serde_json::Value;

    fn inject_auth(
        &self,
        request: RequestBuilder,
        required: bool,
    ) -> crate::Result<RequestBuilder> {
        let auth = &self.config.auth;
        if let Some(key) = auth.key.as_ref() {
            Ok(request.query(&[("Bugzilla_api_key", key)]))
        } else if let (Some(user), Some(pass)) = (&auth.user, &auth.password) {
            Ok(request.query(&[("Bugzilla_login", user), ("Bugzilla_password", pass)]))
        } else if !required {
            Ok(request)
        } else {
            Err(Error::Auth)
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> crate::Result<Self::Response> {
        trace!("{response:?}");

        match response.error_for_status_ref() {
            Ok(_) => {
                let data: serde_json::Value = response.json().await?;
                debug!(
                    "response data:\n{}",
                    serde_json::to_string_pretty(&data).unwrap()
                );
                return_if_error!(&data);
                Ok(data)
            }
            Err(e) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    debug!("error:\n{}", serde_json::to_string_pretty(&data).unwrap());
                    return_if_error!(&data);
                }
                Err(e.into())
            }
        }
    }
}

impl WebClient for Service {
    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

    fn name(&self) -> &str {
        self.config.name()
    }
}

#[derive(
    Display,
    EnumIter,
    EnumString,
    VariantNames,
    DeserializeFromStr,
    SerializeDisplay,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Clone,
    Copy,
)]
#[strum(serialize_all = "kebab-case")]
pub enum GroupField {
    /// All possible fields
    All,
    /// All default fields
    Default,
    /// All extra fields
    Extra,
    /// All custom fields
    Custom,
}

impl From<GroupField> for FilterField {
    fn from(value: GroupField) -> Self {
        Self::Group(value)
    }
}

impl Api for GroupField {
    fn api(&self) -> String {
        let value = match self {
            Self::All => "_all",
            Self::Default => "_default",
            Self::Extra => "_extra",
            Self::Custom => "_custom",
        };
        value.to_string()
    }
}

#[derive(
    Display,
    EnumIter,
    EnumString,
    VariantNames,
    DeserializeFromStr,
    SerializeDisplay,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Clone,
    Copy,
)]
#[strum(serialize_all = "kebab-case")]
pub enum BugField {
    /// Unique aliases for the bug.
    Alias,
    /// User the bug is assigned to.
    Assignee,
    /// Bugs that are blocked by this bug.
    Blocks,
    /// Users in the CC list.
    Cc,
    /// Name of the bug component.
    Component,
    /// Time when the bug was created.
    Created,
    /// User who created the bug.
    Creator,
    /// Bug completion date.
    Deadline,
    /// Dependencies of the bug.
    Depends,
    /// Bug ID that this bug is a duplicate of.
    DuplicateOf,
    Flags,
    Id,
    Keywords,
    Os,
    Platform,
    Priority,
    Product,
    /// User who is the QA contact.
    Qa,
    Resolution,
    /// URLs to external trackers.
    SeeAlso,
    Severity,
    Status,
    Summary,
    Tags,
    Target,
    /// Time when the bug was last updated.
    Updated,
    /// URL related to the bug.
    Url,
    Version,
    Whiteboard,
}

impl From<BugField> for FilterField {
    fn from(value: BugField) -> Self {
        Self::Bug(value)
    }
}

impl Api for BugField {
    fn api(&self) -> String {
        let value = match self {
            Self::Alias => "alias",
            Self::Assignee => "assigned_to",
            Self::Blocks => "blocks",
            Self::Cc => "cc",
            Self::Component => "component",
            Self::Created => "creation_time",
            Self::Creator => "creator",
            Self::Deadline => "deadline",
            Self::Depends => "depends_on",
            Self::DuplicateOf => "dupe_of",
            Self::Flags => "flags",
            Self::Id => "id",
            Self::Keywords => "keywords",
            Self::Os => "op_sys",
            Self::Platform => "platform",
            Self::Priority => "priority",
            Self::Product => "product",
            Self::Qa => "qa_contact",
            Self::Resolution => "resolution",
            Self::SeeAlso => "see_also",
            Self::Severity => "severity",
            Self::Status => "status",
            Self::Summary => "summary",
            Self::Tags => "tags",
            Self::Target => "target_milestone",
            Self::Url => "url",
            Self::Updated => "last_change_time",
            Self::Version => "version",
            Self::Whiteboard => "whiteboard",
        };
        value.to_string()
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum FilterField {
    Bug(BugField),
    Group(GroupField),
}

impl FilterField {
    /// Return an iterator over all FilterField variants.
    pub fn iter() -> impl Iterator<Item = FilterField> {
        BugField::iter()
            .map(FilterField::Bug)
            .chain(GroupField::iter().map(FilterField::Group))
    }
}

impl fmt::Display for FilterField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bug(value) => value.fmt(f),
            Self::Group(value) => value.fmt(f),
        }
    }
}

impl FromStr for FilterField {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Ok(value) = BugField::from_str(s) {
            Ok(Self::Bug(value))
        } else if let Ok(value) = GroupField::from_str(s) {
            Ok(Self::Group(value))
        } else {
            Err(Error::InvalidValue(format!("invalid filter field: {s}")))
        }
    }
}

impl Api for FilterField {
    fn api(&self) -> String {
        match self {
            Self::Bug(value) => value.api(),
            Self::Group(value) => value.api(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ServiceCache {
    fields: IndexSet<BugzillaField>,
    custom_fields: IndexSet<BugzillaField>,
}
