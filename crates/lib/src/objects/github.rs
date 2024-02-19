use std::fmt;

use super::Item;

#[derive(Debug, Eq, PartialEq)]
pub struct Issue {
    id: String,
}

impl Issue {
    pub fn search_display(&self) -> String {
        self.id.to_string()
    }
}

impl From<Issue> for Item {
    fn from(value: Issue) -> Self {
        Item::Github(Box::new(value))
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "ID: {}", self.id)
    }
}
