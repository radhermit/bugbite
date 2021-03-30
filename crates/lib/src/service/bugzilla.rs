use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use chrono::offset::Utc;
use indexmap::IndexSet;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use reqwest::{Client, Request};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::time::TimeDelta;
use crate::traits::{Params, WebService};
use crate::Error;

use super::ServiceKind;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    cache: ServiceCache,
}

impl Config {
    pub(super) fn new(base: Url) -> Self {
        Self {
            base,
            cache: Default::default(),
        }
    }

    pub(super) fn service(self, client: Client) -> Service {
        Service {
            config: self,
            client,
        }
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    pub fn kind(&self) -> ServiceKind {
        ServiceKind::BugzillaRestV1
    }
}

pub struct Service {
    config: Config,
    client: reqwest::Client,
}

impl WebService for Service {
    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn get_request<S>(&self, id: S, _comments: bool, _attachments: bool) -> crate::Result<Request>
    where
        S: std::fmt::Display,
    {
        let url = self
            .base()
            .join(&format!("rest/bug/{id}"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;
        Ok(self.client.get(url).build()?)
    }

    fn search_request<P: Params>(&self, mut query: P) -> crate::Result<Request> {
        let url = self
            .base()
            .join(&format!("rest/bug?{}", query.params()))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;
        Ok(self.client.get(url).build()?)
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {
    fields: HashSet<String>,
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug, Default)]
pub struct QueryBuilder {
    query: ListOrderedMultimap<String, String>,
    advanced_count: u64,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn created_after(&mut self, interval: &TimeDelta) -> crate::Result<()> {
        let datetime = Utc::now() - interval.delta();
        let target = format!("{}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
        self.insert("creation_time", target);
        Ok(())
    }

    pub fn modified_after(&mut self, interval: &TimeDelta) -> crate::Result<()> {
        let datetime = Utc::now() - interval.delta();
        let target = format!("{}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
        self.insert("last_change_time", target);
        Ok(())
    }

    pub fn sort<I>(&mut self, terms: I)
    where
        I: IntoIterator<Item = SearchOrder>,
    {
        let order = terms.into_iter().map(|x| x.api()).join(",");
        self.insert("order", order);
    }

    pub fn commenter<I>(&mut self, values: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "commenter");
            self.insert(format!("o{num}"), "substring");
            self.insert(format!("v{num}"), value);
        }
        Ok(())
    }

    pub fn votes(&mut self, value: u32) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "votes");
        self.insert(format!("o{num}"), "greaterthaneq");
        self.insert(format!("v{num}"), format!("{value}"));
    }

    pub fn comments(&mut self, value: u32) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "longdescs.count");
        self.insert(format!("o{num}"), "greaterthaneq");
        self.insert(format!("v{num}"), format!("{value}"));
    }

    pub fn attachments(&mut self, value: bool) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "attach_data.thedata");
        if value {
            self.insert(format!("o{num}"), "isnotempty");
        } else {
            self.insert(format!("o{num}"), "isempty");
        }
    }

    pub fn fields<I>(&mut self, fields: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        // always include the bug ID field
        let mut include_fields = IndexSet::from(["id".to_string()]);
        include_fields.extend(fields);
        self.insert("include_fields", include_fields.iter().join(","));
        Ok(())
    }

    pub fn append<K, V>(&mut self, key: K, value: V)
    where
        K: ToString,
        V: ToString,
    {
        self.query.append(key.to_string(), value.to_string());
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: ToString,
        V: ToString,
    {
        self.query.insert(key.to_string(), value.to_string());
    }
}

impl Params for QueryBuilder {
    fn params(&mut self) -> String {
        // TODO: Move this parameter to the service struct since it's configurable on the server
        // and can be queried for the supported values.
        // only return open bugs by default
        if !self.query.contains_key("status") {
            for value in ["UNCONFIRMED", "CONFIRMED", "IN_PROGRESS"] {
                self.append("status", value);
            }
        }

        // limit requested fields by default to decrease bandwidth and speed up response
        if !self.query.contains_key("include_fields") {
            let fields = ["id", "assigned_to", "summary"];
            self.insert("include_fields", fields.iter().join(","));
        }

        let mut params = url::form_urlencoded::Serializer::new(String::new());
        params.extend_pairs(self.query.iter());
        params.finish()
    }
}

/// Invertable search order sorting term.
#[derive(Debug, Clone)]
pub struct SearchOrder {
    descending: bool,
    term: SearchTerm,
}

impl FromStr for SearchOrder {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let term = s.strip_prefix('-').unwrap_or(s);
        let descending = term != s;
        let term = term
            .parse()
            .map_err(|_| Error::InvalidValue(format!("unknown search term: {term}")))?;
        Ok(Self { descending, term })
    }
}

impl fmt::Display for SearchOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.term.api();
        if self.descending {
            write!(f, "-{name}")
        } else {
            write!(f, "{name}")
        }
    }
}

impl SearchOrder {
    /// Translate a search order variant into the expected REST API v1 name.
    fn api(&self) -> String {
        let name = self.term.api();
        if self.descending {
            format!("{name} DESC")
        } else {
            format!("{name} ASC")
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone)]
#[strum(serialize_all = "kebab-case")]
pub enum SearchTerm {
    Alias,
    AssignedTo,
    Blocks,
    Comments,
    Component,
    Created,
    Id,
    Keywords,
    LastVisited,
    Modified,
    Priority,
    Reporter,
    Severity,
    Status,
    Summary,
    Votes,
}

impl SearchTerm {
    /// Translate a search order variant into the expected REST API v1 name.
    fn api(&self) -> &'static str {
        match self {
            Self::Alias => "alias",
            Self::AssignedTo => "assigned_to",
            Self::Blocks => "blocked",
            Self::Comments => "longdescs.count",
            Self::Component => "component",
            Self::Created => "opendate",
            Self::Id => "bug_id",
            Self::Keywords => "keywords",
            Self::LastVisited => "last_visit_ts",
            Self::Modified => "changeddate",
            Self::Priority => "priority",
            Self::Reporter => "reporter",
            Self::Severity => "bug_severity",
            Self::Status => "bug_status",
            Self::Summary => "short_desc",
            Self::Votes => "votes",
        }
    }
}

#[derive(AsRefStr, Display, EnumIter, EnumString, VariantNames, Debug, Clone)]
#[strum(serialize_all = "snake_case")]
enum BugAttributes {
    ActualTime,
    Alias,
    AssignedTo,
}

/*
    attributes = {
        'actual_time': 'Actual time',
        'alias': 'Alias',
        'assigned_to': 'Assignee',
        'attachments': 'Attachments',
        'blocks': 'Blocks',
        'cc': 'CC',
        'classification': 'Classification',
        'comments': 'Comments',
        'component': 'Component',
        'creation_time': 'Created',
        'creator': 'Reporter',
        'deadline': 'Deadline',
        'depends_on': 'Depends',
        'dupe_of': 'Duplicate of',
        'estimated_time': 'Estimated time',
        'flags': 'Flags',
        'groups': 'Groups',
        'history': 'History',
        'id': 'ID',
        'is_cc_accessible': 'Is CC Accessible',
        'is_confirmed': 'Confirmed',
        'is_creator_accessible': 'Is Creator Accessible',
        'keywords': 'Keywords',
        'last_change_time': 'Modified',
        'op_sys': 'Operating System',
        'platform': 'Platform',
        'priority': 'Priority',
        'product': 'Product',
        'qa_contact': 'QA Contact',
        'ref': 'Reference',
        'remaining_time': 'Remaining time',
        'resolution': 'Resolution',
        'see_also': 'See also',
        'severity': 'Severity',
        'status': 'Status',
        'summary': 'Title',
        'target_milestone': 'Target milestone',
        'url': 'URL',
        'version': 'Version',
        'whiteboard': 'Whiteboard',
    }

    attribute_aliases = {
        'owner': 'assigned_to',
        'modified': 'last_change_time',
        'created': 'creation_time',
        'depends': 'depends_on',
        'title': 'summary',
        'changes': 'history',
    }

    _print_fields = (
        ('summary', 'Title'),
        ('alias', 'Alias'),
        ('assigned_to', 'Assignee'),
        ('creator', 'Reporter'),
        ('qa_contact', 'QA Contact'),
        ('creation_time', 'Reported'),
        ('last_change_time', 'Updated'),
        ('status', 'Status'),
        ('resolution', 'Resolution'),
        ('dupe_of', 'Duplicate'),
        ('whiteboard', 'Whiteboard'),
        ('severity', 'Severity'),
        ('priority', 'Priority'),
        ('classification', 'Class'),
        ('product', 'Product'),
        ('component', 'Component'),
        ('platform', 'Platform'),
        ('op_sys', 'OS'),
        ('keywords', 'Keywords'),
        ('target_milestone', 'Target'),
        ('version', 'Version'),
        ('url', 'URL'),
        ('ref', 'Reference'),
        ('see_also', 'See also'),
        ('cc', 'CC'),
        ('id', 'ID'),
        ('blocks', 'Blocks'),
        ('depends_on', 'Depends'),
        ('flags', 'Flags'),
        ('groups', 'Groups'),
        ('estimated_time', 'Estimated'),
        ('deadline', 'Deadline'),
        ('actual_time', 'Actual'),
        ('remaining_time', 'Remaining'),
        #('is_cc_accessible', 'Is CC Accessible'),
        #('is_confirmed', 'Confirmed'),
        #('is_creator_accessible', 'Is Creator Accessible'),
        ('history', 'Changes'),
        ('comments', 'Comments'),
        ('attachments', 'Attachments'),
    )
*/

#[derive(Deserialize, Serialize, Debug)]
pub struct Attachment {
    name: String,
}

impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Attachment: {}", self.name)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Comment {
    text: String,
}

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.text)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(default)]
pub struct Bug {
    id: u64,
    assigned_to: Option<String>,
    #[serde(rename = "creator")]
    reporter: Option<String>,
    #[serde(rename = "alias")]
    aliases: Vec<String>,
    summary: Option<String>,
    status: Option<String>,
    cc: Vec<String>,
    blocks: Vec<u64>,
    comments: Vec<Comment>,
    attachments: Vec<Attachment>,
}

impl fmt::Display for Bug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(data) = self.summary.as_deref() {
            writeln!(f, "Summary: {data}")?;
        }
        if let Some(data) = self.assigned_to.as_deref() {
            writeln!(f, "Assignee: {data}")?;
        }
        if let Some(data) = self.reporter.as_deref() {
            writeln!(f, "Reporter: {data}")?;
        }
        if let Some(data) = self.status.as_deref() {
            writeln!(f, "Status: {data}")?;
        }
        writeln!(f, "ID: {}", self.id)?;
        if !self.aliases.is_empty() {
            writeln!(f, "Aliases: {}", self.aliases.iter().join(", "))?;
        }
        if !self.cc.is_empty() {
            writeln!(f, "CC: {}", self.cc.iter().join(", "))?;
        }
        if !self.blocks.is_empty() {
            writeln!(f, "Blocks: {}", self.blocks.iter().join(", "))?;
        }
        if !self.comments.is_empty() {
            writeln!(f, "Comments: {}", self.comments.len())?;
        }
        if !self.attachments.is_empty() {
            writeln!(f, "Attachment: {}", self.attachments.len())?;
        }
        for attachment in &self.attachments {
            write!(f, "{attachment}")?;
        }
        for comment in &self.comments {
            write!(f, "{comment}")?;
        }
        Ok(())
    }
}

impl Bug {
    pub fn search_display(&self) -> String {
        let id = self.id;
        match (self.assigned_to.as_deref(), self.summary.as_deref()) {
            (Some(assignee), Some(summary)) => format!("{id:<8} {assignee:<20} {summary}"),
            (Some(assignee), None) => format!("{id:<8} {assignee}"),
            (None, Some(summary)) => format!("{id:<8} {summary}"),
            (None, None) => format!("{id}"),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn reporter(&self) -> Option<&str> {
        self.reporter.as_deref()
    }
}
