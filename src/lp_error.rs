//! Defines the `LPError` enum, which represents all possible errors that can occur in the application.
//!
//! This error type uses the [`thiserror`](https://docs.rs/thiserror/) crate for ergonomic error handling and
//! implements the `std::error::Error` trait. Each variant corresponds to a specific error source or context:
//!
//! - `Database`: Wraps errors originating from the `sqlx` crate.
//! - `DatabaseCustom`: Represents custom database errors as a string message.
//! - `Migration`: Wraps migration errors from `sqlx::migrate`.
//! - `Api`: Wraps errors from the `reqwest` HTTP client.
//! - `ApiCustom`: Represents custom API errors as a string message.
//! - `Serde`: Wraps errors from `serde_json`.
//! - `Env`: Represents environment variable errors as a string message.
//!
//! This enum allows for unified error handling throughout the application.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LPError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Database error: {0}")]
    DatabaseCustom(String),
    #[error("Database migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("API error: {0}")]
    Api(#[from] reqwest::Error),
    #[error("API error: {0}")]
    ApiCustom(String),
    #[error("serde_json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Environment variable error: {0}")]
    Env(String),
}
