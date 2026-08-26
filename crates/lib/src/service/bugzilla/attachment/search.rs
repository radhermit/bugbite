use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::args::ExistsOrValues;
use crate::objects::RangeOrValue;
use crate::objects::bugzilla::Attachment;
use crate::service::bugzilla::{Bugzilla, search};
use crate::time::TimeDeltaOrStatic;
use crate::traits::{Merge, RequestPagedStream, RequestSend, RequestTemplate};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request {
    #[serde(skip)]
    service: Bugzilla,
    #[serde(flatten)]
    pub params: Parameters,
}

impl Request {
    /// Create a new request.
    pub(crate) fn new(service: Bugzilla) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }
}

impl RequestTemplate for Request {
    type Params = Parameters;
    type Service = Bugzilla;
    const TYPE: &'static str = "search";

    fn service(&self) -> &Self::Service {
        &self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
    }
}

/// Attachment search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Parameters {
    pub creator: Option<Vec<Vec<search::Match>>>,
    pub description: Option<Vec<Vec<search::Match>>>,
    pub filename: Option<Vec<Vec<search::Match>>>,
    pub ids: Option<Vec<ExistsOrValues<RangeOrValue<i64>>>>,
    pub mime: Option<Vec<Vec<search::Match>>>,
    pub size: Option<RangeOrValue<u64>>,

    pub created: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub updated: Option<RangeOrValue<TimeDeltaOrStatic>>,

    pub is_obsolete: Option<bool>,
    pub is_patch: Option<bool>,
    pub is_private: Option<bool>,
}

impl Parameters {
    fn matches(&self, attachment: &Attachment) -> bool {
        self.size
            .as_ref()
            .is_none_or(|x| x.matches(&attachment.size.as_u64()))
            && self
                .created
                .as_ref()
                .is_none_or(|x| x.matches(&attachment.created))
            && self
                .updated
                .as_ref()
                .is_none_or(|x| x.matches(&attachment.updated))
    }
}

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            creator: self.creator.merge(other.creator),
            description: self.description.merge(other.description),
            filename: self.filename.merge(other.filename),
            ids: self.ids.merge(other.ids),
            mime: self.mime.merge(other.mime),
            size: self.size.merge(other.size),
            created: self.created.merge(other.created),
            updated: self.updated.merge(other.updated),
            is_obsolete: self.is_obsolete.merge(other.is_obsolete),
            is_patch: self.is_patch.merge(other.is_patch),
            is_private: self.is_private.merge(other.is_private),
        }
    }
}

impl Merge<Parameters> for search::Parameters {
    fn merge(&mut self, other: Parameters) {
        *self = Self {
            attachment_creator: self.attachment_creator.merge(other.creator),
            attachment_description: self.attachment_description.merge(other.description),
            attachment_filename: self.attachment_filename.merge(other.filename),
            ids: self.ids.merge(other.ids),
            attachment_mime: self.attachment_mime.merge(other.mime),
            created: self.created.merge(other.created),
            updated: self.updated.merge(other.updated),
            attachment_is_obsolete: self.attachment_is_obsolete.merge(other.is_obsolete),
            attachment_is_patch: self.attachment_is_patch.merge(other.is_patch),
            attachment_is_private: self.attachment_is_private.merge(other.is_private),
            ..Default::default()
        }
    }
}

impl RequestSend for Request {
    type Output = Vec<Attachment>;

    async fn send(&self) -> crate::Result<Self::Output> {
        // search for bugs with matching attachments
        let mut request = self.service.search();
        request.params.merge(self.params.clone());
        let bugs = request.send().await?;
        let ids: Vec<_> = bugs.into_iter().map(|b| b.id).collect();

        let mut attachments = vec![];

        // request the attachments from the bugs
        if !ids.is_empty() {
            let mut request = self.service.attachment_get_item(ids);
            request.data(false);
            let data = request.send().await?;
            attachments.extend(
                data.into_iter()
                    .flatten()
                    .filter(|x| self.params.matches(x)),
            );
        }

        Ok(attachments)
    }
}
