pub mod bugzilla;
pub mod github;

pub enum Item {
    Bugzilla(bugzilla::Bug),
    Github(github::Issue),
}
