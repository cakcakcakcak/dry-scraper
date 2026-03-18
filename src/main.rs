use clap::Parser;
use dry_scraper::config::{
    cli_args::{CliArgs, Commands, NhlCommand, ScrapeSource},
    Config,
};

use dry_scraper::common::app_context::AppContext;

use dry_scraper::common::{data_source::DataSource, db::DbContext, errors::DSError};

#[cfg(debug_assertions)]
use dry_scraper::common::db::init::reset_schema;

use dry_scraper::data_sources::nhl::{
    data_source::NhlDataSource,
    orchestrator::{
        get_nhl_everything_in_season, get_nhl_franchises, get_nhl_game, get_nhl_seasons,
        get_nhl_teams,
    },
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), DSError> {
    _ = dotenvy::dotenv();

    // Initialize tracing with indicatif support to prevent progress bar conflicts
    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();
    let filter = EnvFilter::try_from_env("LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("warn,dry_scraper::data_sources=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
        .init();

    let cli_args = CliArgs::parse();
    let config = Arc::new(Config::from_env_and_args());

    let db_context: DbContext = DbContext::connect(&config).await?;
    let mut app_context: AppContext = AppContext::new(config.clone(), cli_args.no_progress);

    // register data sources
    let sources: Vec<Arc<dyn dry_scraper::common::data_source::DataSource>> =
        vec![Arc::new(NhlDataSource::with_config(&config))];
    app_context = app_context.with_sources(sources);

    match cli_args.command {
        Some(Commands::Scrape { source }) => match source {
            ScrapeSource::Nhl { command } => {
                let nhl_source = app_context.sources[0]
                    .as_any()
                    .downcast_ref::<NhlDataSource>()
                    .expect("First source should be NhlDataSource");

                match command {
                    #[cfg(debug_assertions)]
                    NhlCommand::Reset => {
                        tracing::warn!("Resetting database schema");
                        reset_schema(&db_context.pool, &config).await?;
                        tracing::info!("Database schema reset complete");
                    }
                    NhlCommand::All | NhlCommand::Game { .. } | NhlCommand::Season { .. } => {
                        // Always warm cache and fetch base data before any NHL operation
                        nhl_source.warm_cache(&app_context, &db_context).await?;

                        // Fetch base data (franchises, teams, seasons)
                        _ = get_nhl_franchises(&app_context, &db_context, &nhl_source.api).await?;
                        let seasons =
                            get_nhl_seasons(&app_context, &db_context, &nhl_source.api).await?;
                        _ = get_nhl_teams(&app_context, &db_context, &nhl_source.api).await?;

                        match command {
                            NhlCommand::All => for _season in seasons {},
                            NhlCommand::Game { game_id } => {
                                let _game = get_nhl_game(
                                    &app_context,
                                    &db_context,
                                    &nhl_source.api,
                                    game_id as i32,
                                )
                                .await?;
                            }
                            NhlCommand::Season { season_id } => {
                                get_nhl_everything_in_season(
                                    &app_context,
                                    &db_context,
                                    &nhl_source.api,
                                    season_id as i32,
                                )
                                .await?;
                            }
                            #[cfg(debug_assertions)]
                            NhlCommand::Reset => unreachable!(),
                        }
                    }
                }
            }
        },
        None => {
            tracing::info!("No command specified. Use --help to see available commands.");
        }
    }

    Ok(())
}
