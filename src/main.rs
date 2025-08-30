// use rand::seq::SliceRandom;
use tokio;
use tracing_subscriber;

mod any_primary_key;
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

    // console_subscriber::init();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("LOG_LEVEL"))
        .init();

    _ = &*CONFIG;

    let db_context = init_db_context().await?;
    warm_nhl_key_cache(&db_context).await?;
    let nhl_api: NhlApi = NhlApi::new();

    let mut seasons: Vec<NhlSeason> = get_nhl_seasons(&db_context, &nhl_api).await?;
    _ = get_nhl_franchises(&db_context, &nhl_api).await?;
    _ = get_nhl_teams(&db_context, &nhl_api).await?;

    seasons.sort_by_key(|season| season.id);
    seasons.pop();
    seasons.reverse();

    for season in seasons {
        get_nhl_everything_in_season(&db_context, &nhl_api, &season).await?;
    }

    Ok(())
}
