use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Bug;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

use super::attachment::AttachmentRequest;
use super::comment::CommentRequest;
use super::history::HistoryRequest;
use super::IdOrAlias;

#[derive(Debug)]
pub(crate) struct GetRequest {
    url: Url,
    ids: Vec<String>,
    attachments: Option<AttachmentRequest>,
    comments: Option<CommentRequest>,
    history: Option<HistoryRequest>,
}

impl GetRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        if ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        // TODO: use query builder instead of manual creation
        let mut url = service.base().join("rest/bug")?;
        let mut count = 1;
        url.query_pairs_mut()
            .append_pair(&format!("f{count}"), "OP");
        url.query_pairs_mut()
            .append_pair(&format!("j{count}"), "OR");

        let mut new_ids = vec![];
        for id in ids {
            let id = id.to_string();
            let id_or_alias = IdOrAlias::from(id.as_str());
            let field = match id_or_alias {
                IdOrAlias::Id(_) => "bug_id",
                IdOrAlias::Alias(_) => "alias",
            };

            count += 1;
            url.query_pairs_mut()
                .append_pair(&format!("f{count}"), field);
            url.query_pairs_mut()
                .append_pair(&format!("o{count}"), "equals");
            url.query_pairs_mut().append_pair(&format!("v{count}"), &id);
            new_ids.push(id);
        }

        count += 1;
        url.query_pairs_mut()
            .append_pair(&format!("f{count}"), "CP");

        // include personal tags
        url.query_pairs_mut()
            .append_pair("include_fields", "_default,tags");

        // drop useless token that is injected for authenticated requests
        url.query_pairs_mut()
            .append_pair("exclude_fields", "update_token");

        let attachments = if attachments {
            Some(service.item_attachment_request(ids, false)?)
        } else {
            None
        };
        let comments = if comments {
            Some(CommentRequest::new(service, ids, None)?)
        } else {
            None
        };
        let history = if history {
            Some(HistoryRequest::new(service, ids, None)?)
        } else {
            None
        };

        Ok(Self {
            url,
            ids: new_ids,
            attachments,
            comments,
            history,
        })
    }
}

impl Request for GetRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.url).inject_auth(service, false)?;
        let (bugs, attachments, comments, history) = (
            request.send(),
            self.attachments.map(|r| r.send(service)),
            self.comments.map(|r| r.send(service)),
            self.history.map(|r| r.send(service)),
        );

        let response = bugs.await?;
        let mut data = service.parse_response(response).await?;
        let Value::Array(data) = data["bugs"].take() else {
            return Err(Error::InvalidValue(
                "invalid service response to get request".to_string(),
            ));
        };
        let mut data = data.into_iter();

        let mut attachments = match attachments {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };
        let mut comments = match comments {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };
        let mut history = match history {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };

        let mut bugs = vec![];
        for id in &self.ids {
            let value = data
                .next()
                .ok_or_else(|| Error::InvalidValue(format!("nonexistent bug {id}")))?;
            let mut bug: Bug = serde_json::from_value(value)?;
            bug.attachments = attachments.next().unwrap_or_default();
            bug.comments = comments.next().unwrap_or_default();
            bug.history = history.next().unwrap_or_default();
            bugs.push(bug);
        }

        Ok(bugs)
    }
}
