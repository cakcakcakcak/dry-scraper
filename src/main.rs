use tokio;
use tracing_subscriber;

mod common;
mod config;
mod data_sources;

use config::CONFIG;

use common::{
    db::{SqlxJob, init_db_context},
    errors::LPError,
};

use data_sources::nhl::{api::*, models::*, orchestrator::*};

#[tokio::main]
async fn main() -> Result<(), LPError> {
    _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("LOG_LEVEL"))
        .init();

    _ = &*CONFIG;

    let db_context = init_db_context().await?;
    warm_nhl_key_cache(&db_context).await?;
    let nhl_api: NhlApi = NhlApi::new();

    let seasons: Vec<NhlSeason> = get_nhl_seasons(&db_context, &nhl_api).await?;
    let franchises: Vec<NhlFranchise> = get_nhl_franchises(&db_context, &nhl_api).await?;
    let teams: Vec<NhlTeam> = get_nhl_teams(&db_context, &nhl_api).await?;

    for season in seasons {
        let games: Vec<NhlGame> =
            get_nhl_all_games_in_season(&db_context, &nhl_api, &season).await?;
        for game in games {
            get_nhl_plays_in_game(&db_context, &nhl_api, &game).await?;
            get_nhl_roster_spots_in_game(&db_context, &nhl_api, &game).await?;
        }
    }

    Ok(())
}
