use clap::Parser;
use dry_scraper::config::{
    cli_args::{CliArgs, Commands, ScrapeSource},
    CONFIG,
};

use dry_scraper::common::app_context::AppContext;

use dry_scraper::common::{db::DbContext, errors::DSError};

#[cfg(debug_assertions)]
use dry_scraper::common::db::init::reset_schema;

use dry_scraper::data_sources::nhl::data_source::NhlDataSource;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), DSError> {
    _ = dotenvy::dotenv();

    // console_subscriber::init();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("LOG_LEVEL"))
        .init();

    _ = &*CONFIG;

    let cli_args = CliArgs::parse();

    let db_context: DbContext = DbContext::connect(&CONFIG).await?;
    let mut app_context: AppContext =
        AppContext::new(std::sync::Arc::new((*CONFIG).clone()), cli_args.no_progress);

    // register data sources
    let sources: Vec<Arc<dyn dry_scraper::common::data_source::DataSource>> =
        vec![Arc::new(NhlDataSource::new())];
    app_context = app_context.with_sources(sources);

    match cli_args.command {
        Some(Commands::Scrape { source }) => match source {
            ScrapeSource::Nhl {
                #[cfg(debug_assertions)]
                reset,
            } => {
                #[cfg(debug_assertions)]
                if reset {
                    reset_schema(&db_context.pool, &CONFIG).await?;
                }

                tracing::info!("Starting NHL scrape");
                // call warm_cache via the registry
                let nhl_source = &app_context.sources[0];
                nhl_source.warm_cache(&app_context, &db_context).await?;
                tracing::info!("NHL scrape complete");
            }
        },
        None => {
            tracing::info!("No command specified. Use --help to see available commands.");
        }
    }

    Ok(())
}
