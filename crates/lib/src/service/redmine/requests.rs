pub mod get;
pub mod search;

// public request creation API
impl super::Redmine {
    pub fn get<I>(&self, ids: I) -> get::Request
    where
        I: IntoIterator<Item = u64>,
    {
        get::Request::new(self.clone(), ids)
    }

    pub fn search(&self) -> search::Request {
        search::Request::new(self.clone())
    }
}
