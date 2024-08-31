use std::{fmt, fs};

use camino::Utf8Path;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::objects::bugzilla::{Bug, Flag};
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestMerge, RequestSend, WebService};
use crate::utils::{or, prefix};
use crate::Error;

#[derive(Serialize, Debug)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a Service,
    #[serde(flatten)]
    pub params: Parameters,
}

impl RequestMerge<&Utf8Path> for Request<'_> {
    fn merge(&mut self, path: &Utf8Path) -> crate::Result<()> {
        let params = Parameters::from_path(path)?;
        self.params.merge(params);
        Ok(())
    }
}

impl<T: Into<Parameters>> RequestMerge<T> for Request<'_> {
    fn merge(&mut self, value: T) -> crate::Result<()> {
        self.params.merge(value);
        Ok(())
    }
}

impl RequestSend for Request<'_> {
    type Output = u64;

    async fn send(&self) -> crate::Result<Self::Output> {
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
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing id: {e}")))
    }
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
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
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
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

impl Parameters {
    /// Load parameters in TOML format from a file.
    fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Override parameters using the provided value if it exists.
    fn merge<T: Into<Self>>(&mut self, other: T) {
        let other = other.into();
        or!(self.alias, other.alias);
        or!(self.assignee, other.assignee);
        or!(self.blocks, other.blocks);
        or!(self.cc, other.cc);
        or!(self.component, other.component);
        or!(self.custom_fields, other.custom_fields);
        or!(self.depends, other.depends);
        or!(self.description, other.description);
        or!(self.flags, other.flags);
        or!(self.groups, other.groups);
        or!(self.keywords, other.keywords);
        or!(self.os, other.os);
        or!(self.platform, other.platform);
        or!(self.priority, other.priority);
        or!(self.product, other.product);
        or!(self.qa, other.qa);
        or!(self.resolution, other.resolution);
        or!(self.see_also, other.see_also);
        or!(self.status, other.status);
        or!(self.severity, other.severity);
        or!(self.target, other.target);
        or!(self.summary, other.summary);
        or!(self.url, other.url);
        or!(self.version, other.version);
        or!(self.whiteboard, other.whiteboard);
    }

    /// Encode parameters into the form required for the request.
    fn encode<'a>(&'a self, service: &'a Service) -> crate::Result<RequestParameters<'a>> {
        let params = RequestParameters {
            // required fields with defaults
            op_sys: self.os.as_deref().unwrap_or("All"),
            platform: self.platform.as_deref().unwrap_or("All"),
            priority: self.priority.as_deref().unwrap_or("Normal"),
            severity: self.severity.as_deref().unwrap_or("normal"),
            version: self.version.as_deref().unwrap_or("unspecified"),

            // required fields without defaults
            component: self.component.as_deref().unwrap_or_default(),
            description: self.description.as_deref().unwrap_or_default(),
            product: self.product.as_deref().unwrap_or_default(),
            summary: self.summary.as_deref().unwrap_or_default(),

            // optional fields
            alias: self.alias.as_deref(),
            assigned_to: self
                .assignee
                .as_deref()
                .map(|x| service.replace_user_alias(x)),
            blocks: self.blocks.as_deref(),
            cc: self.cc.as_deref(),
            depends_on: self.depends.as_deref(),
            flags: self.flags.as_deref(),
            groups: self.groups.as_deref(),
            keywords: self.keywords.as_deref(),
            qa_contact: self.qa.as_deref().map(|x| service.replace_user_alias(x)),
            resolution: self.resolution.as_deref(),
            see_also: self.see_also.as_deref(),
            status: self.status.as_deref(),
            target_milestone: self.target.as_deref(),
            url: self.url.as_deref(),
            whiteboard: self.whiteboard.as_deref(),

            // auto-prefix custom field names
            custom_fields: self.custom_fields.as_ref().map(|values| {
                values
                    .into_iter()
                    .map(|(name, value)| (prefix!("cf_", name), value))
                    .collect()
            }),
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
    custom_fields: Option<IndexMap<String, &'a String>>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        // TODO: improve API for setting user info on config creation
        let mut config = Config::new(server.uri()).unwrap();
        config.user = Some("user".to_string());
        config.password = Some("pass".to_string());
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
