pub mod args;
pub mod client;
pub mod error;
pub mod objects;
pub mod service;
pub mod services;
pub mod time;
pub mod traits;
pub mod utils;

pub use self::error::Error;

/// A `Result` alias where the `Err` case is `pkgcraft::Error`.
pub type Result<T> = std::result::Result<T, Error>;
