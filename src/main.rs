use dry_scraper::config::{AppContext, CONFIG, UI_CONFIG};

use dry_scraper::common::{db::DbContext, errors::DSError};

use dry_scraper::data_sources::nhl::{api::*, orchestrator::*};

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
    let _nhl_api: NhlApi = NhlApi::new();

    Ok(())
}
