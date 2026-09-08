pub mod get;
pub mod search;

// public request creation API
impl super::Github {
    pub fn get<I>(&self, _ids: I) -> get::Request
    where
        I: IntoIterator<Item = u64>,
    {
        todo!("get requests unsupported")
    }

    pub fn search(&self) -> search::Request {
        search::Request::new(self.clone())
    }
}
