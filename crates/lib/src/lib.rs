use chrono::{DateTime, Utc};
use std::collections::HashSet;

pub mod args;
pub mod client;
pub mod error;
pub mod service;
pub mod services;
pub mod time;
pub mod traits;

pub use self::error::Error;

/// A `Result` alias where the `Err` case is `pkgcraft::Error`.
pub type Result<T> = std::result::Result<T, Error>;

pub struct Change {
    id: u32,
    creator: String,
    created: DateTime<Utc>,
    changes: HashSet<String>,
    count: u32,
}

pub struct Comment {
    id: u32,
    creator: String,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    text: String,
    count: u32,
}

pub struct Attachment {
    id: u32,
    creator: String,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    filename: String,
    url: String,
    size: u32,
    mimetype: String,
    data: Vec<u8>,
}

pub struct Item {
    id: u32,
    title: String,
    creator: String,
    owner: String,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    status: String,
    url: String,
    cc: HashSet<String>,
    blocks: HashSet<u32>,
    depends: HashSet<u32>,
    comments: Vec<Comment>,
    attachments: Vec<Attachment>,
    changes: Vec<Change>,
}
