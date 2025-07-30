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

pub use api::nhl::NhlApi;
pub use db::{DbPool, SqlxJob, SqlxJobSender};
pub use lp_error::LPError;
pub use models::nhl::{NhlFranchise, NhlSeason, NhlTeam};

use db::init_db_context;
use orchestrator::{
    get_nhl_all_games_in_season, get_nhl_franchises, get_nhl_seasons, get_nhl_teams,
};

#[tokio::main]
async fn main() -> Result<(), LPError> {
    _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("LOG_LEVEL"))
        .init();

    _ = &*CONFIG;

    let db_context = init_db_context().await?;
    let nhl_api: NhlApi = NhlApi::new();

    let seasons: Vec<NhlSeason> = get_nhl_seasons(&db_context, &nhl_api).await?;
    let franchises = get_nhl_franchises(&db_context, &nhl_api).await?;
    let teams = get_nhl_teams(&db_context, &nhl_api).await?;

    Ok(())
}
