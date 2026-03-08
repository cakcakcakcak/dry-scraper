use clap::Parser;
use dry_scraper::config::{
    cli_args::{CliArgs, Commands, ScrapeSource},
    AppContext, CONFIG, UI_CONFIG,
};

use dry_scraper::common::{db::DbContext, errors::DSError};

#[cfg(debug_assertions)]
use dry_scraper::common::db::init::reset_schema;

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

    let cli_args = CliArgs::parse();

    let db_context: DbContext = DbContext::connect().await?;
    let app_context: AppContext = AppContext::new();

    match cli_args.command {
        Some(Commands::Scrape { source }) => match source {
            ScrapeSource::Nhl {
                #[cfg(debug_assertions)]
                reset,
            } => {
                #[cfg(debug_assertions)]
                if reset {
                    reset_schema(&db_context.pool).await?;
                }

                tracing::info!("Starting NHL scrape");
                let _nhl_api: NhlApi = NhlApi::new();
                warm_nhl_key_cache(&app_context, &db_context).await?;
                tracing::info!("NHL scrape complete");
            }
        },
        None => {
            tracing::info!("No command specified. Use --help to see available commands.");
        }
    }

    Ok(())
}
