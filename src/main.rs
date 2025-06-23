use tokio;
use tracing_subscriber;

mod api;
mod config;
mod db;
mod lp_error;
mod models;
mod serde_helpers;
mod util;

use api::nhl::NhlStatsApi;
use config::env::ENVIRONMENT_VARIABLES;
use db::init::init_db;
use lp_error::LPError;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), LPError> {
    // validate environment variables and initialize a static ENVIRONMENT_VARIABLES struct
    _ = &*ENVIRONMENT_VARIABLES;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();

    // initialize the lp database and return the pool of connections with which all db queries
    // will be made
    let pool = init_db().await?;

    let _seasons = NhlStatsApi::new().get_nhl_seasons(&pool).await?;
    let _franchises = NhlStatsApi::new().get_nhl_franchises(&pool).await?;
    let _teams = NhlStatsApi::new().get_nhl_teams(&pool).await?;
    Ok(())
}
