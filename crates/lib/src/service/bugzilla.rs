use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::traits::{Api, WebClient, WebService};
use crate::Error;

use super::{ClientBuilder, ServiceKind};

pub mod attachment;
pub mod comment;
pub mod create;
mod get;
pub mod history;
pub mod search;
pub mod update;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    pub user: Option<String>,
    pub password: Option<String>,
    pub key: Option<String>,
    cache: ServiceCache,
}

impl Config {
    pub fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            user: None,
            password: None,
            key: None,
            cache: Default::default(),
        })
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    pub fn kind(&self) -> ServiceKind {
        ServiceKind::Bugzilla
    }
}

#[derive(Debug)]
pub struct Service {
    config: Config,
    client: reqwest::Client,
}

impl Service {
    pub fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        Ok(Self {
            config,
            client: builder.build()?,
        })
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: fmt::Display>(&self, id: I) -> String {
        let base = self.base().as_str().trim_end_matches('/');
        format!("{base}/show_bug.cgi?id={id}")
    }

    /// Return the website URL for a query.
    pub fn search_url(&self, params: search::Parameters) -> crate::Result<String> {
        let base = self.base().as_str().trim_end_matches('/');
        let params = params.encode(self)?;
        Ok(format!("{base}/buglist.cgi?{params}"))
    }

    /// Substitute user alias for matching value.
    // TODO: support pulling aliases from the config?
    pub(crate) fn replace_user_alias<'a>(&'a self, value: &'a str) -> &str {
        if value == "@me" {
            self.config.user.as_deref().unwrap_or(value)
        } else {
            value
        }
    }

    pub fn attachment_create<I, S>(
        &self,
        ids: I,
        attachments: Vec<attachment::create::CreateAttachment>,
    ) -> attachment::create::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        attachment::create::Request::new(self, ids, attachments)
    }

    pub fn attachment_get<I, S>(&self, ids: I) -> attachment::get::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
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

    pub fn attachment_update<I, S>(&self, ids: I) -> attachment::update::Request
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
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
        let config = &self.config;
        if let Some(key) = config.key.as_ref() {
            Ok(request.query(&[("Bugzilla_api_key", key)]))
        } else if let (Some(user), Some(pass)) = (&config.user, &config.password) {
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
                debug!("{data}");
                return_if_error!(&data);
                Ok(data)
            }
            Err(e) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    debug!("{data}");
                    return_if_error!(&data);
                }
                Err(e.into())
            }
        }
    }
}

impl<'a> WebClient<'a> for Service {
    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
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
    Default,
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
    #[default]
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

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
enum IdOrAlias {
    Id(u64),
    Alias(String),
}

impl fmt::Display for IdOrAlias {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Id(value) => value.fmt(f),
            Self::Alias(value) => value.fmt(f),
        }
    }
}

impl FromStr for IdOrAlias {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        Ok(s.into())
    }
}

impl From<&str> for IdOrAlias {
    fn from(s: &str) -> Self {
        if let Ok(value) = s.parse::<u64>() {
            Self::Id(value)
        } else {
            Self::Alias(s.to_string())
        }
    }
}

impl From<u64> for IdOrAlias {
    fn from(value: u64) -> Self {
        Self::Id(value)
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

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {
    fields: HashSet<String>,
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    use crate::traits::RequestSend;

    use super::*;

    #[tokio::test]
    async fn attachment_create() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service
            .attachment_create(ids, vec![])
            .send()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // no attachments
        let err = service
            .attachment_create([1], vec![])
            .send()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no attachments specified");
    }

    #[tokio::test]
    async fn attachment_get() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.attachment_get(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent
        server
            .respond(200, path.join("attachment/nonexistent.json"))
            .await;
        let err = service.attachment_get([1]).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidValue(_)));
        assert_err_re!(err, "nonexistent attachment: 1");

        server.reset().await;

        // deleted
        server
            .respond(200, path.join("attachment/deleted.json"))
            .await;
        let err = service.attachment_get([21]).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidValue(_)));
        assert_err_re!(err, "deleted attachment: 21");

        server.reset().await;

        // invalid response
        server
            .respond(200, path.join("attachment/invalid.json"))
            .await;
        let err = service.attachment_get([123]).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidValue(_)));
        assert_err_re!(err, "failed deserializing attachment: 123");

        server.reset().await;

        // single without data
        server
            .respond(200, path.join("attachment/single-without-data.json"))
            .await;
        let attachment = &service
            .attachment_get([123])
            .data(false)
            .send()
            .await
            .unwrap()[0];
        assert!(attachment.is_empty());

        server.reset().await;

        // single with plain text data
        server
            .respond(200, path.join("attachment/single-plain-text.json"))
            .await;
        let attachment = &service.attachment_get([123]).send().await.unwrap()[0];
        assert_eq!(attachment.id, 123);
        assert_eq!(attachment.bug_id, 321);
        assert_eq!(attachment.file_name, "test.txt");
        assert_eq!(attachment.summary, "test.txt");
        assert_eq!(attachment.size, 8);
        assert_eq!(attachment.creator, "person");
        assert_eq!(attachment.content_type, "text/plain");
        assert!(!attachment.is_private);
        assert!(!attachment.is_obsolete);
        assert!(!attachment.is_patch);
        assert_eq!(attachment.created.to_string(), "2024-02-19 08:35:02 UTC");
        assert_eq!(attachment.updated.to_string(), "2024-02-19 08:35:02 UTC");
        assert!(attachment.flags.is_empty());
        assert_eq!(String::from_utf8_lossy(attachment.as_ref()), "bugbite\n");

        server.reset().await;

        // multiple with plain text data
        server
            .respond(200, path.join("attachment/multiple-plain-text.json"))
            .await;
        let ids = [123, 124];
        let attachments = &service.attachment_get(ids).send().await.unwrap();
        assert_ordered_eq!(attachments.iter().map(|x| x.id), ids);
    }

    #[tokio::test]
    async fn attachment_get_item() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.attachment_get_item(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent bug
        server
            .respond(404, path.join("errors/nonexistent-bug.json"))
            .await;
        let err = service.attachment_get_item([1]).send().await.unwrap_err();
        assert!(
            matches!(err, Error::Bugzilla { code: 101, .. }),
            "unmatched error: {err:?}"
        );

        server.reset().await;

        // bug with no attachments
        server
            .respond(200, path.join("attachment/bug-with-no-attachments.json"))
            .await;
        let attachments = &service.attachment_get_item([12345]).send().await.unwrap()[0];
        assert!(attachments.is_empty());

        server.reset().await;

        // bugs with no attachments
        server
            .respond(200, path.join("attachment/bug-with-no-attachments.json"))
            .await;
        let attachments = &service
            .attachment_get_item([12345, 23456, 34567])
            .send()
            .await
            .unwrap();
        assert!(attachments.iter().all(|x| x.is_empty()));
    }

    #[tokio::test]
    async fn attachment_update() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.attachment_update(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }

    #[tokio::test]
    async fn comment() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.comment(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }

    #[tokio::test]
    async fn create() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // missing required fields without defaults
        let err = service.create().send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(
            err,
            "missing required fields: component, description, product, summary"
        );

        // empty required fields
        let err = service
            .create()
            .os("")
            .description("a")
            .summary("b")
            .send()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "missing required fields: component, os, product");
    }

    #[tokio::test]
    async fn get() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.get(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent bug
        server
            .respond(404, path.join("errors/nonexistent-bug.json"))
            .await;
        let err = service.get([1]).send().await.unwrap_err();
        assert!(
            matches!(err, Error::Bugzilla { code: 101, .. }),
            "unmatched error: {err:?}"
        );

        server.reset().await;

        // invalid response
        server.respond(200, path.join("get/invalid.json")).await;
        let err = service.get([12345]).send().await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidValue(_)),
            "unmatched error: {err:?}"
        );
        assert_err_re!(err, "invalid service response");

        server.reset().await;

        // single bug
        server.respond(200, path.join("get/single-bug.json")).await;
        let ids = [12345];
        let bugs = service.get(ids).send().await.unwrap();
        assert_ordered_eq!(bugs.iter().map(|x| x.id), ids);

        server.reset().await;

        // multiple bugs
        server
            .respond(200, path.join("get/multiple-bugs.json"))
            .await;
        let ids = [12345, 23456, 34567];
        let bugs = service.get(ids).send().await.unwrap();
        assert_ordered_eq!(bugs.iter().map(|x| x.id), ids);
    }

    #[tokio::test]
    async fn history() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.history(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }

    #[tokio::test]
    async fn search() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        server.respond(200, path.join("search/ids.json")).await;
        let bugs = service.search().summary(["test"]).send().await.unwrap();
        assert_eq!(bugs.len(), 5);
    }

    #[tokio::test]
    async fn update() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.update(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }
}
