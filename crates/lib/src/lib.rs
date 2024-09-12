pub mod args;
pub mod config;
pub mod error;
pub mod objects;
#[cfg(feature = "output")]
pub mod output;
pub mod query;
pub(crate) mod serde;
pub mod service;
#[cfg(feature = "test")]
pub mod test;
pub mod time;
pub mod traits;
pub mod utils;

pub use self::error::Error;

/// A `Result` alias where the `Err` case is `pkgcraft::Error`.
pub type Result<T> = std::result::Result<T, Error>;
