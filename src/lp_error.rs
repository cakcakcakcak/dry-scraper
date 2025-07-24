use std::{env::VarError, num::ParseIntError};

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
    Env(#[from] VarError),
    #[error("Parse error: {0}")]
    Parse(#[from] ParseIntError),
}
