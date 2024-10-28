use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub struct Issue {
    pub id: u64,
    pub title: Option<String>,
}

impl Issue {
    pub fn search_display(&self) -> String {
        self.id.to_string()
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "ID: {}", self.id)
    }
}
