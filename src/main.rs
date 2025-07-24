use tokio;
use tracing_subscriber;

mod api;
mod config;
mod db;
mod lp_error;
mod models;
mod orchestrator;
mod serde_helpers;
mod util;

pub use config::CONFIG;

pub use api::nhl::{NhlStatsApi, NhlWebApi};
pub use db::DbPool;
pub use lp_error::LPError;

use db::init_db;
use orchestrator::{
    get_nhl_all_games_in_season, get_nhl_franchises, get_nhl_game, get_nhl_player, get_nhl_seasons,
    get_nhl_team, get_nhl_teams,
};

#[tokio::main]
async fn main() -> Result<(), LPError> {
    // load the .env file into the environment variables, if it exists
    _ = dotenvy::dotenv();

    // initialize logging with the level indicated by environment variable
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("LOG_LEVEL"))
        .init();

    // validate command line arguments and environment variables and initialize a static CONFIG struct
    _ = &*CONFIG;

    // initialize the lp database and return the pool of connections with which all db queries
    // will be made
    let pool: DbPool = init_db().await?;

    let nhl_stats_api: NhlStatsApi = NhlStatsApi::new();
    let nhl_web_api: NhlWebApi = NhlWebApi::new();

    let seasons = get_nhl_seasons(&pool, &nhl_stats_api).await?;
    _ = get_nhl_franchises(&pool, &nhl_stats_api).await?;
    _ = get_nhl_teams(&pool, &nhl_stats_api).await?;
    _ = get_nhl_team(&pool, &nhl_stats_api, 7288).await?;
    _ = get_nhl_player(&pool, &nhl_web_api, 8478402).await?;
    _ = get_nhl_game(&pool, &nhl_web_api, 2023020204).await?;

    for season in seasons {
        let games = get_nhl_all_games_in_season(&pool, &nhl_web_api, &season).await?;
    }

    Ok(())
}
