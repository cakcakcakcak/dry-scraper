use std::{env::VarError, num::ParseIntError};

use thiserror::Error;

use crate::SqlxJob;

#[derive(Error, Debug)]
pub enum DSError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Database error: {0}")]
    DatabaseCustom(String),
    #[error("Failed to send SQLx Job: {0}")]
    SqlxJobSend(Box<tokio::sync::mpsc::error::SendError<SqlxJob>>),
    #[error("Failed to receive result of SQLx Job: {0}")]
    SqlxJobRecv(#[from] tokio::sync::oneshot::error::RecvError),
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

impl From<tokio::sync::mpsc::error::SendError<SqlxJob>> for DSError {
    fn from(err: tokio::sync::mpsc::error::SendError<SqlxJob>) -> Self {
        DSError::SqlxJobSend(Box::new(err))
    }
}
