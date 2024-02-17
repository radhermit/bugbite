use std::fmt;
use std::str::FromStr;

use chrono::offset::Utc;
use indexmap::IndexSet;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::debug;

use crate::objects::bugzilla::Bug;
use crate::time::TimeDelta;
use crate::traits::{Api, Params, Request, WebService};
use crate::Error;

use super::Field;

#[derive(Debug)]
pub(crate) struct SearchRequest(reqwest::Request);

impl Request for SearchRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.0).await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        debug!("search request data: {data}");
        Ok(serde_json::from_value(data)?)
    }
}

impl SearchRequest {
    pub(super) fn new<P: Params>(service: &super::Service, mut query: P) -> crate::Result<Self> {
        let url = service
            .base()
            .join(&format!("rest/bug?{}", query.params()?))?;
        Ok(Self(service.client.get(url).build()?))
    }
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
        I: IntoIterator<Item = Field>,
    {
        // always include the bug ID field
        let include_fields: IndexSet<_> = [Field::Id].into_iter().chain(fields).collect();
        self.insert(
            "include_fields",
            include_fields.iter().map(|f| f.api()).join(","),
        );
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
    fn params(&mut self) -> crate::Result<String> {
        if self.query.is_empty() {
            return Err(Error::EmptyQuery);
        }

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
        Ok(params.finish())
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
