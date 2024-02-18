pub mod args;
pub mod client;
pub mod config;
pub mod error;
pub mod objects;
pub(crate) mod serde;
pub mod service;
pub mod services;
#[cfg(test)]
pub(crate) mod test;
pub mod time;
pub mod traits;
pub mod utils;

pub use self::error::Error;

/// A `Result` alias where the `Err` case is `pkgcraft::Error`.
pub type Result<T> = std::result::Result<T, Error>;
