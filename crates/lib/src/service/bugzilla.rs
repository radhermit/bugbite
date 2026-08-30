use std::collections::HashSet;
use std::fmt;
use std::str::{self, FromStr};
use std::sync::{Arc, LazyLock};

use indexmap::IndexSet;
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator, VariantNames};
use url::Url;

use crate::Error;
use crate::objects::bugzilla::BugzillaField;
use crate::traits::{Api, JsonResponse, Merge, ParseResponse, WebClient, WebService};

use super::{ClientParameters, ServiceKind};

pub mod attachment;
pub mod comment;
pub mod create;
pub mod fields;
mod get;
pub mod history;
pub mod search;
pub mod update;
pub mod user;
pub mod version;

/// Common default values used for unset fields.
pub(crate) static UNSET_VALUES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    ["unspecified", "Unspecified", "---", "--", "-", ""]
        .iter()
        .map(|s| s.to_string())
        .collect()
});

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
/// Bugzilla authentication information.
pub struct Authentication {
    /// API key
    pub key: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

// TODO: improve API for setting user info on config creation
/// Bugzilla service config.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Config {
    base: Url,
    pub name: String,
    #[serde(flatten)]
    pub auth: Authentication,
    #[serde(flatten)]
    pub client: ClientParameters,

    /// Maximum number of results that can be returned by a search request.
    #[serde(default = "default_max_search_results")]
    pub max_search_results: usize,
}

// TODO: replace with default field value when stabilized
// (https://github.com/rust-lang/rfcs/pull/3681)
/// Return the default size of bugzilla's max search results.
fn default_max_search_results() -> usize {
    10000
}

impl Config {
    /// Create a new Bugzilla service config.
    pub fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            name: Default::default(),
            auth: Default::default(),
            client: Default::default(),
            max_search_results: default_max_search_results(),
        })
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
struct Service {
    client: reqwest::Client,
    config: Config,
    _cache: ServiceCache,
}

#[derive(Debug)]
pub struct ServiceBuilder {
    config: Config,
}

impl ServiceBuilder {
    /// Create a new Bugzilla service builder.
    pub fn name(mut self, value: &str) -> Self {
        self.config.name = value.to_string();
        self
    }

    /// Set the client parameters for the service.
    pub fn client(mut self, value: ClientParameters) -> Self {
        self.config.client.merge(value);
        self
    }

    /// Set the user for the service.
    pub fn user(mut self, value: &str) -> Self {
        self.config.auth.user = Some(value.to_string());
        self
    }

    /// Set the user's password for the service.
    pub fn password(mut self, value: &str) -> Self {
        self.config.auth.password = Some(value.to_string());
        self
    }

    /// Create a new service.
    pub fn build(self) -> crate::Result<Bugzilla> {
        let client = self.config.client.build()?;
        Ok(Bugzilla(Arc::new(Service {
            config: self.config,
            _cache: Default::default(),
            client,
        })))
    }
}

#[derive(Debug, Clone)]
pub struct Bugzilla(Arc<Service>);

impl PartialEq for Bugzilla {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl fmt::Display for Bugzilla {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -- {}", self.kind(), self.base())
    }
}

impl Bugzilla {
    /// Create a new Service using a given base URL.
    pub fn new(base: &str) -> crate::Result<Self> {
        Self::builder(base)?.build()
    }

    /// Create a new Service builder using a given base URL.
    pub fn builder(base: &str) -> crate::Result<ServiceBuilder> {
        Ok(ServiceBuilder {
            config: Config::new(base)?,
        })
    }

    /// Create a new Service builder using a given base URL.
    pub fn config_builder(
        config: &crate::config::Config,
        name: Option<&str>,
    ) -> crate::Result<ServiceBuilder> {
        let config = config
            .get_kind(ServiceKind::Bugzilla, name)?
            .into_bugzilla()
            .unwrap();
        Ok(ServiceBuilder { config })
    }

    /// Return the service config.
    pub fn config(&self) -> &Config {
        &self.0.config
    }

    /// Return the service client.
    pub fn client(&self) -> &reqwest::Client {
        &self.0.client
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: fmt::Display>(&self, id: I) -> String {
        let base = self.base().as_str().trim_end_matches('/');
        format!("{base}/show_bug.cgi?id={id}")
    }

    /// Substitute user alias for matching value.
    // TODO: support pulling aliases from the config?
    fn replace_user_alias<'a>(&'a self, value: &'a str) -> &'a str {
        if value == "@me" {
            self.config().auth.user.as_deref().unwrap_or(value)
        } else {
            value
        }
    }

    /// Create a request to add attachments to the specified bugs.
    pub fn attachment_create<I, S>(&self, ids: I) -> attachment::create::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        attachment::create::Request::new(self.clone(), ids)
    }

    /// Create a request to get the specified attachments.
    pub fn attachment_get<I>(&self, ids: I) -> attachment::get::Request
    where
        I: IntoIterator<Item = u64>,
    {
        attachment::get::Request::new(self.clone(), ids)
    }

    /// Create a request to get attachments from the specified bugs.
    pub fn attachment_get_item<I, S>(&self, ids: I) -> attachment::get_item::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        attachment::get_item::Request::new(self.clone(), ids)
    }

    /// Create a request to search for attachments.
    pub fn attachment_search(&self) -> attachment::search::Request {
        attachment::search::Request::new(self.clone())
    }

    /// Create a request to update the specified attachments.
    pub fn attachment_update<I>(&self, ids: I) -> attachment::update::Request
    where
        I: IntoIterator<Item = u64>,
    {
        attachment::update::Request::new(self.clone(), ids)
    }

    /// Create a request to get the comments from the specified bugs.
    pub fn comment_get<I, S>(&self, ids: I) -> comment::get::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        comment::get::Request::new(self.clone(), ids)
    }

    /// Create a request to tag the comments from the specified bugs.
    pub fn comment_tag<I, S>(&self, ids: I) -> comment::tag::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        comment::tag::Request::new(self.clone(), ids)
    }

    /// Create a request to create a bug.
    pub fn create(&self) -> create::Request {
        create::Request::new(self.clone())
    }

    /// Create a request to get the Bugzilla service fields.
    pub fn fields(&self) -> fields::Request {
        fields::Request::new(self.clone())
    }

    /// Create a request to get the specified bugs.
    pub fn get<I, S>(&self, ids: I) -> get::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        get::Request::new(self.clone(), ids)
    }

    /// Create a request to get the history of the specified bugs.
    pub fn history<I, S>(&self, ids: I) -> history::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        history::Request::new(self.clone(), ids)
    }

    /// Create a request to search for bugs.
    pub fn search(&self) -> search::Request {
        search::Request::new(self.clone())
    }

    /// Create a request to update the specified bugs.
    pub fn update<I, S>(&self, ids: I) -> update::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        update::Request::new(self.clone(), ids)
    }

    /// Create a request to get the Bugzilla service version.
    pub fn version(&self) -> version::Request {
        version::Request::new(self.clone())
    }

    /// Create a request to create the specified users.
    pub fn user_create<I, S>(&self, emails: I) -> user::create::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        user::create::Request::new(self.clone(), emails)
    }

    /// Create a request to get the specified users.
    pub fn user_get<I, S>(&self, ids: I) -> user::get::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        user::get::Request::new(self.clone(), ids)
    }

    /// Create a request to update the specified users.
    pub fn user_update<I, S>(&self, ids: I) -> user::update::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        user::update::Request::new(self.clone(), ids)
    }
}

/// Bugzilla REST API error response.
#[derive(Deserialize, Debug)]
pub struct ServiceError {
    pub message: String,
    pub code: i32,
}

impl From<ServiceError> for Error {
    fn from(value: ServiceError) -> Self {
        Self::Service(super::ServiceError::Bugzilla(value))
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl JsonResponse for Bugzilla {}

impl ParseResponse for Bugzilla {
    type ServiceError = ServiceError;

    async fn parse_response<T>(&self, response: reqwest::Response) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        self.parse_json(response).await
    }
}

impl WebService for Bugzilla {
    const API_VERSION: &'static str = "v1";

    fn inject_auth(
        &self,
        request: RequestBuilder,
        required: bool,
    ) -> crate::Result<RequestBuilder> {
        let auth = &self.config().auth;
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
}

impl WebClient for Bugzilla {
    fn base(&self) -> &Url {
        self.config().base()
    }

    fn kind(&self) -> ServiceKind {
        self.config().kind()
    }

    fn name(&self) -> &str {
        self.config().name()
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

#[derive(Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct ServiceCache {
    fields: IndexSet<BugzillaField>,
    custom_fields: IndexSet<BugzillaField>,
}
