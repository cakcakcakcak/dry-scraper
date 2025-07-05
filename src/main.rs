use clap::Parser;
use tokio;
use tracing_subscriber;

mod api;
mod config;
mod db;
mod lp_error;
mod models;
mod serde_helpers;
mod util;

use config::CONFIG;

use api::nhl_stats_api::NhlStatsApi;
use api::nhl_web_api::NhlWebApi;
use db::init::init_db;
use lp_error::LPError;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), LPError> {
    // load the .env file into the environment variables, if it exists
    dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();

    // validate command line arguments and environment variables and initialize a static CONFIG struct
    _ = &*CONFIG;

    // initialize the lp database and return the pool of connections with which all db queries
    // will be made
    let pool = init_db().await?;

    NhlStatsApi::new().get_nhl_seasons(&pool).await?;
    NhlStatsApi::new().get_nhl_franchises(&pool).await?;
    NhlStatsApi::new().get_nhl_teams(&pool).await?;
    NhlWebApi::new().get_nhl_playoff_series(&pool).await?;
    Ok(())
}
