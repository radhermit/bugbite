pub mod bugzilla;
pub mod github;

pub enum Item {
    Bugzilla(Box<bugzilla::Bug>),
    Github(Box<github::Issue>),
}
