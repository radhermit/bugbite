pub mod attachment;
pub mod comment;
pub mod create;
pub mod fields;
pub mod get;
pub mod history;
pub mod search;
pub mod update;
pub mod user;
pub mod version;

// public request creation API
impl super::Bugzilla {
    /// Create a request to add attachments to the specified bugs.
    pub fn attachment_create<I>(&self, ids: I) -> attachment::create::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
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
    pub fn attachment_get_item<I>(&self, ids: I) -> attachment::get_item::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
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
    pub fn comment_get<I>(&self, ids: I) -> comment::get::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        comment::get::Request::new(self.clone(), ids)
    }

    /// Create a request to tag the comments from the specified bugs.
    pub fn comment_tag<I>(&self, ids: I) -> comment::tag::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
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
    pub fn get<I>(&self, ids: I) -> get::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        get::Request::new(self.clone(), ids)
    }

    /// Create a request to get the history of the specified bugs.
    pub fn history<I>(&self, ids: I) -> history::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        history::Request::new(self.clone(), ids)
    }

    /// Create a request to search for bugs.
    pub fn search(&self) -> search::Request {
        search::Request::new(self.clone())
    }

    /// Create a request to update the specified bugs.
    pub fn update<I>(&self, ids: I) -> update::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        update::Request::new(self.clone(), ids)
    }

    /// Create a request to get the Bugzilla service version.
    pub fn version(&self) -> version::Request {
        version::Request::new(self.clone())
    }

    /// Create a request to create the specified users.
    pub fn user_create<I>(&self, emails: I) -> user::create::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        user::create::Request::new(self.clone(), emails)
    }

    /// Create a request to get the specified users.
    pub fn user_get<I>(&self, ids: I) -> user::get::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        user::get::Request::new(self.clone(), ids)
    }

    /// Create a request to update the specified users.
    pub fn user_update<I>(&self, ids: I) -> user::update::Request
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        user::update::Request::new(self.clone(), ids)
    }
}
