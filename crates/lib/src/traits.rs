use std::borrow::Cow;
use std::future::Future;
use std::io::{stdout, Write};
use std::{fmt, fs};

use async_stream::try_stream;
use camino::Utf8PathBuf;
use futures_util::Stream;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::service::ServiceKind;
use crate::utils::config_dir;
use crate::Error;

/// Return true if a type contains a given object, otherwise false.
pub trait Contains<T> {
    fn contains(&self, obj: &T) -> bool;
}

/// Render an object in search context into a formatted string using the given fields.
pub trait RenderSearch<T> {
    fn render(&self, fields: &[T]) -> String;
}

/// Encode a type into the expected API name.
pub(crate) trait Api {
    fn api(&self) -> String;
}

impl Api for String {
    fn api(&self) -> String {
        self.clone()
    }
}

impl Api for &str {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl<T: fmt::Display + Clone> Api for Cow<'_, T> {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for u64 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for usize {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for i64 {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl<T: Api> Api for &T {
    fn api(&self) -> String {
        (*self).api()
    }
}

pub trait MergeOption<T> {
    fn merge(&mut self, value: Option<T>) -> Self;
}

impl<T> MergeOption<T> for Option<T> {
    fn merge(&mut self, value: Option<T>) -> Self {
        value.or_else(|| self.take())
    }
}

pub trait Merge {
    fn merge(&mut self, other: Self);
}

pub trait RequestSend {
    type Output;

    fn send(&self) -> impl Future<Output = crate::Result<Self::Output>>;
}

pub trait RequestStream: RequestSend<Output = Vec<Self::Item>> + Clone {
    type Item;

    /// Return the page size if paging is enabled.
    fn paged(&mut self) -> Option<usize>;
    /// Modify the request to return the next page.
    fn next_page(&mut self, size: usize);

    // TODO: submit multiple requests at once?
    /// Send requests and return the stream of items for them.
    fn stream(&self) -> impl Stream<Item = crate::Result<Self::Item>> + '_ {
        let mut req = self.clone();
        let paged = req.paged();

        try_stream! {
            loop {
                let items = req.send().await?;
                let count = items.len();

                for item in items {
                    yield item;
                }

                match paged {
                    Some(size) if count == size => req.next_page(size),
                    _ => break,
                }
            }
        }
    }
}

pub trait RequestTemplate: Serialize {
    type Params: for<'a> Deserialize<'a> + Merge;
    type Service: WebClient;
    const TYPE: &'static str;

    fn service(&self) -> &Self::Service;
    fn params(&mut self) -> &mut Self::Params;

    /// Return the config path for a template file.
    fn config_path(&self, name: &str) -> crate::Result<Utf8PathBuf> {
        let service_name = self.service().name();
        if service_name.trim().is_empty() || name.contains(std::path::is_separator) {
            Ok(Utf8PathBuf::from(name))
        } else {
            let path = format!("templates/{service_name}/{}/{name}", Self::TYPE);
            config_dir().map(|x| x.join(path))
        }
    }

    /// Load a request template using the given name.
    fn load_template(&mut self, s: &str) -> crate::Result<&mut Self> {
        let name = s.trim();
        if name.is_empty() {
            return Err(Error::InvalidValue(format!("invalid template name: {s:?}")));
        }

        let path = self.config_path(name)?;
        let data = fs::read_to_string(&path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {name}: {e}")))?;
        let params = toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {name}: {e}")))?;
        self.params().merge(params);

        Ok(self)
    }

    /// Save a request template using the given name.
    fn save_template(&self, s: &str) -> crate::Result<()> {
        let name = s.trim();
        if name.is_empty() {
            return Err(Error::InvalidValue(format!("invalid template name: {s:?}")));
        }

        let data = toml::to_string(self)
            .map_err(|e| Error::InvalidValue(format!("failed serializing template: {e}")))?;
        if data.trim().is_empty() {
            return Err(Error::InvalidValue(format!(
                "empty request template: {name}"
            )));
        }

        if name == "-" {
            write!(stdout(), "{data}")?;
        } else {
            let path = self.config_path(name)?;
            fs::create_dir_all(path.parent().expect("invalid template path"))
                .map_err(|e| Error::IO(format!("failed creating template dir: {e}")))?;
            fs::write(&path, data)
                .map_err(|e| Error::IO(format!("failed saving template: {name}: {e}")))?;
        }

        Ok(())
    }
}

/// Inject service authentication data into a request.
pub(crate) trait InjectAuth: Sized {
    /// Authentication required for request.
    fn auth<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self>;

    /// Authentication optional for request.
    fn auth_optional<'a, W: WebService<'a>>(self, service: &'a W) -> Self;
}

impl InjectAuth for RequestBuilder {
    fn auth<'a, W: WebService<'a>>(self, service: &'a W) -> crate::Result<Self> {
        service.inject_auth(self, true)
    }

    fn auth_optional<'a, W: WebService<'a>>(self, service: &'a W) -> Self {
        service
            .inject_auth(self, false)
            .expect("failed injecting optional auth")
    }
}

pub(crate) trait WebService<'a>: fmt::Display {
    const API_VERSION: &'static str;
    type Response;

    /// Inject authentication into a request before it's sent.
    fn inject_auth(&self, request: RequestBuilder, required: bool)
        -> crate::Result<RequestBuilder>;

    /// Parse a raw response into a service response.
    async fn parse_response(&self, response: reqwest::Response) -> crate::Result<Self::Response>;
}

pub trait WebClient {
    /// Return the base URL for a service.
    fn base(&self) -> &Url;
    /// Return the service variant.
    fn kind(&self) -> ServiceKind;
    /// Return the connection name.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::tempdir;

    use crate::service::bugzilla::Service;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request_template() {
        let server = TestServer::new().await;
        let service = Service::new(server.uri()).unwrap();
        let request1 = service.search();
        let mut request2 = service.search();

        // invalid names
        for name in [" ", "", "\t"] {
            let err = request1.save_template(name).unwrap_err();
            assert_err_re!(err, "invalid template name: ");
            let err = request2.load_template(name).unwrap_err();
            assert_err_re!(err, "invalid template name: ");
        }

        // empty template
        let err = request1.save_template("test").unwrap_err();
        assert_err_re!(err, "empty request template: test");

        // create temporary config dir
        let dir = tempdir().unwrap();
        env::set_current_dir(dir.path()).unwrap();
        let path = dir.path().join("dir/template");
        let path_str = path.to_str().unwrap();

        let time = "1d".parse().unwrap();
        let request1 = request1.created(time);

        // save to specific path
        request1.save_template(path_str).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap().trim(),
            r#"created = "1d""#
        );
        assert_ne!(request1, request2);
        request2.load_template(path_str).unwrap();
        assert_eq!(request1, request2);

        // unnamed services save to current working directory
        request1.save_template("test").unwrap();
        assert_eq!(
            fs::read_to_string("test").unwrap().trim(),
            r#"created = "1d""#
        );
        request2.load_template("test").unwrap();
        assert_eq!(request1, request2);

        // named services save to config dir path
        let mut service = Service::new(server.uri()).unwrap();
        service.config.name = "service".to_string();
        let time = "2d".parse().unwrap();
        let request1 = service.search().created(time);
        let mut request2 = service.search();

        // depends on linux specific config dir handling
        if cfg!(target_os = "linux") {
            // $XDG_CONFIG_HOME takes precedence over $HOME
            env::set_var("HOME", dir.path());
            env::set_var("XDG_CONFIG_HOME", dir.path());
            request1.save_template("test").unwrap();
            assert_eq!(
                fs::read_to_string("bugbite/templates/service/search/test")
                    .unwrap()
                    .trim(),
                r#"created = "2d""#
            );
            request2.load_template("test").unwrap();
            assert_eq!(request1, request2);

            // $HOME is used when $XDG_CONFIG_HOME is unset
            env::remove_var("XDG_CONFIG_HOME");
            request1.save_template("test").unwrap();
            assert_eq!(
                fs::read_to_string(".config/bugbite/templates/service/search/test")
                    .unwrap()
                    .trim(),
                r#"created = "2d""#
            );
            request2.load_template("test").unwrap();
            assert_eq!(request1, request2);
        }
    }
}
