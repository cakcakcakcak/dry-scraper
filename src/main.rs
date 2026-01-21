// use rand::seq::SliceRandom;
use tokio;
use tracing_subscriber;

mod any_primary_key;
mod common;
mod config;
mod data_sources;

use config::{AppContext, CONFIG, UI_CONFIG};

use common::{
    db::{DbContext, SqlxJob},
    errors::DSError,
};

use data_sources::nhl::{api::*, models::*, orchestrator::*};

#[tokio::main]
async fn main() -> Result<(), DSError> {
    _ = dotenvy::dotenv();

    // console_subscriber::init();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("LOG_LEVEL"))
        .init();

    _ = &*CONFIG;
    _ = &*UI_CONFIG;

    let db_context: DbContext = DbContext::connect().await?;
    let app_context: AppContext = AppContext::new();
    warm_nhl_key_cache(&app_context, &db_context).await?;
    let nhl_api: NhlApi = NhlApi::new();

    let mut seasons: Vec<NhlSeason> = get_nhl_seasons(&app_context, &db_context, &nhl_api).await?;
    _ = get_nhl_franchises(&app_context, &db_context, &nhl_api).await?;
    _ = get_nhl_teams(&app_context, &db_context, &nhl_api).await?;

    seasons.sort_by_key(|season| season.id);
    seasons.pop();
    seasons.reverse();

    for season in seasons {
        get_nhl_everything_in_season(&app_context, &db_context, &nhl_api, &season).await?;
    }

    Ok(())
}
