use std::fmt;

use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::objects::bugzilla::{Bug, Flag};
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, Merge, MergeOption, RequestSend, RequestTemplate, WebService};
use crate::Error;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a Service,
    #[serde(flatten)]
    pub params: Parameters,
}

impl RequestSend for Request<'_> {
    type Output = u64;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.service.config.base.join("rest/bug")?;
        let params = self.encode()?;
        let request = self
            .service
            .client
            .post(url)
            .json(&params)
            .auth(self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        serde_json::from_value(data["id"].take())
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing id: {e}")))
    }
}

impl RequestTemplate for Request<'_> {
    type Params = Parameters;
    type Service = Service;
    const TYPE: &'static str = "create";

    fn service(&self) -> &Self::Service {
        self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
    }
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }

    /// Encode parameters into the form required for the request.
    fn encode(&self) -> crate::Result<RequestParameters> {
        let params = RequestParameters {
            // required fields with defaults
            op_sys: self.params.os.as_deref().unwrap_or("All"),
            platform: self.params.platform.as_deref().unwrap_or("All"),
            priority: self.params.priority.as_deref().unwrap_or("Normal"),
            severity: self.params.severity.as_deref().unwrap_or("normal"),
            version: self.params.version.as_deref().unwrap_or("unspecified"),

            // required fields without defaults
            component: self.params.component.as_deref().unwrap_or_default(),
            description: self.params.description.as_deref().unwrap_or_default(),
            product: self.params.product.as_deref().unwrap_or_default(),
            summary: self.params.summary.as_deref().unwrap_or_default(),

            // optional fields
            alias: self.params.alias.as_deref(),
            assigned_to: self
                .params
                .assignee
                .as_deref()
                .map(|x| self.service.replace_user_alias(x)),
            blocks: self.params.blocks.as_deref(),
            cc: self.params.cc.as_deref(),
            depends_on: self.params.depends.as_deref(),
            flags: self.params.flags.as_deref(),
            groups: self.params.groups.as_deref(),
            keywords: self.params.keywords.as_deref(),
            qa_contact: self
                .params
                .qa
                .as_deref()
                .map(|x| self.service.replace_user_alias(x)),
            resolution: self.params.resolution.as_deref(),
            see_also: self.params.see_also.as_deref(),
            status: self.params.status.as_deref(),
            target_milestone: self.params.target.as_deref(),
            url: self.params.url.as_deref(),
            whiteboard: self.params.whiteboard.as_deref(),
            custom_fields: self.params.custom_fields.as_ref(),
        };

        // verify required fields are non-empty
        let mut missing = vec![];
        for (value, name) in [
            (params.component, "component"),
            (params.description, "description"),
            (params.op_sys, "os"),
            (params.platform, "platform"),
            (params.priority, "priority"),
            (params.product, "product"),
            (params.severity, "severity"),
            (params.summary, "summary"),
            (params.version, "version"),
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

    pub fn alias<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.alias = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn assignee<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.assignee = Some(value.to_string());
        self
    }

    pub fn blocks<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.blocks = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn cc<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.cc = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn component<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.component = Some(value.to_string());
        self
    }

    pub fn depends<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.depends = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn description<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.description = Some(value.to_string());
        self
    }

    pub fn flags<I>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = Flag>,
    {
        self.params.flags = Some(value.into_iter().collect());
        self
    }

    pub fn groups<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.groups = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn keywords<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.keywords = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn os<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.os = Some(value.to_string());
        self
    }

    pub fn platform<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.platform = Some(value.to_string());
        self
    }

    pub fn priority<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.priority = Some(value.to_string());
        self
    }

    pub fn product<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.product = Some(value.to_string());
        self
    }

    pub fn qa<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.qa = Some(value.to_string());
        self
    }

    pub fn resolution<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.resolution = Some(value.to_string());
        self
    }

    pub fn see_also<I, S>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.params.see_also = Some(value.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn severity<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.severity = Some(value.to_string());
        self
    }

    pub fn status<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.status = Some(value.to_string());
        self
    }

    pub fn summary<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.summary = Some(value.to_string());
        self
    }

    pub fn target<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.target = Some(value.to_string());
        self
    }

    pub fn url<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.url = Some(value.to_string());
        self
    }

    pub fn version<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.version = Some(value.to_string());
        self
    }

    pub fn whiteboard<S>(mut self, value: S) -> Self
    where
        S: fmt::Display,
    {
        self.params.whiteboard = Some(value.to_string());
        self
    }

    pub fn custom_fields<I, S1, S2>(mut self, value: I) -> Self
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: fmt::Display,
        S2: fmt::Display,
    {
        self.params.custom_fields = Some(
            value
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        self
    }
}

/// Bug creation parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Parameters {
    pub alias: Option<Vec<String>>,
    pub assignee: Option<String>,
    pub blocks: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub component: Option<String>,
    pub depends: Option<Vec<String>>,
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

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            alias: self.alias.merge(other.alias),
            assignee: self.assignee.merge(other.assignee),
            blocks: self.blocks.merge(other.blocks),
            cc: self.cc.merge(other.cc),
            component: self.component.merge(other.component),
            custom_fields: self.custom_fields.merge(other.custom_fields),
            depends: self.depends.merge(other.depends),
            description: self.description.merge(other.description),
            flags: self.flags.merge(other.flags),
            groups: self.groups.merge(other.groups),
            keywords: self.keywords.merge(other.keywords),
            os: self.os.merge(other.os),
            platform: self.platform.merge(other.platform),
            priority: self.priority.merge(other.priority),
            product: self.product.merge(other.product),
            qa: self.qa.merge(other.qa),
            resolution: self.resolution.merge(other.resolution),
            see_also: self.see_also.merge(other.see_also),
            status: self.status.merge(other.status),
            severity: self.severity.merge(other.severity),
            target: self.target.merge(other.target),
            summary: self.summary.merge(other.summary),
            url: self.url.merge(other.url),
            version: self.version.merge(other.version),
            whiteboard: self.whiteboard.merge(other.whiteboard),
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
#[derive(Serialize)]
struct RequestParameters<'a> {
    // required fields
    component: &'a str,
    description: &'a str,
    op_sys: &'a str,
    platform: &'a str,
    priority: &'a str,
    product: &'a str,
    severity: &'a str,
    summary: &'a str,
    version: &'a str,

    // optional fields
    alias: Option<&'a [String]>,
    assigned_to: Option<&'a str>,
    blocks: Option<&'a [String]>,
    cc: Option<&'a [String]>,
    depends_on: Option<&'a [String]>,
    flags: Option<&'a [Flag]>,
    groups: Option<&'a [String]>,
    keywords: Option<&'a [String]>,
    qa_contact: Option<&'a str>,
    resolution: Option<&'a str>,
    see_also: Option<&'a [String]>,
    status: Option<&'a str>,
    target_milestone: Option<&'a str>,
    url: Option<&'a str>,
    whiteboard: Option<&'a str>,

    #[serde(flatten)]
    custom_fields: Option<&'a IndexMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        // TODO: improve API for setting user info on config creation
        let mut service = Service::new(server.uri()).unwrap();
        service.config.auth.user = Some("user".to_string());
        service.config.auth.password = Some("pass".to_string());

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

        // create new request with default fields set
        let request = || {
            service
                .create()
                .component("TestComponent")
                .product("TestProduct")
                .description("description")
                .summary("summary")
        };

        server.respond(200, path.join("create/creation.json")).await;

        // valid
        let id = request().send().await.unwrap();
        assert_eq!(id, 123);

        // alias
        request().alias(["alias1", "alias2"]).send().await.unwrap();

        // assignee
        request().assignee("user").send().await.unwrap();

        // blocks
        request().blocks([1]).send().await.unwrap();
        request().blocks(["alias1", "alias2"]).send().await.unwrap();

        // cc
        request().cc(["user@email.com"]).send().await.unwrap();
        request()
            .cc(["user1@email.com", "user2@email.com"])
            .send()
            .await
            .unwrap();

        // component
        request().component("component").send().await.unwrap();

        // depends
        request().depends([1]).send().await.unwrap();
        request()
            .depends(["alias1", "alias2"])
            .send()
            .await
            .unwrap();

        // description
        request().description("description").send().await.unwrap();

        // flags
        let flag1 = Flag::from_str("flag1+").unwrap();
        let flag2 = Flag::from_str("flag2-").unwrap();
        let flag3 = Flag::from_str("flag3?").unwrap();
        request().flags([flag1, flag2, flag3]).send().await.unwrap();

        // groups
        request().groups(["group1", "group2"]).send().await.unwrap();

        // keywords
        request().keywords(["kw1", "kw2"]).send().await.unwrap();

        // os
        request().os("os").send().await.unwrap();

        // platform
        request().platform("platform").send().await.unwrap();

        // priority
        request().priority("normal").send().await.unwrap();

        // product
        request().product("product").send().await.unwrap();

        // qa
        request().qa("user@email.com").send().await.unwrap();

        // resolution
        request().resolution("fixed").send().await.unwrap();

        // see also
        request().see_also([36]).send().await.unwrap();
        request().see_also(["36"]).send().await.unwrap();
        request()
            .see_also(["https://link/to/external/bug"])
            .send()
            .await
            .unwrap();

        // severity
        request().severity("normal").send().await.unwrap();

        // status
        request().status("closed").send().await.unwrap();

        // summary
        request().summary("summary").send().await.unwrap();

        // target
        request().target("milestone").send().await.unwrap();

        // url
        request().url("https://link/to/site").send().await.unwrap();

        // version
        request().version("1.2.3").send().await.unwrap();

        // whiteboard
        request().whiteboard("note").send().await.unwrap();

        // custom fields
        request()
            .custom_fields([("name", "value")])
            .send()
            .await
            .unwrap();
    }
}
