use tokio;

mod api;
mod config;
mod lp_error;
mod models;
mod serde_helpers;
mod util;

use lp_error::LPError;

#[tokio::main]
async fn main() -> Result<(), LPError> {
    // validate and initialize a static ENVIRONMENT_VARIABLES struct
    _ = &*config::ENVIRONMENT_VARIABLES;

    let pool = config::init_db().await?;
    let _seasons = api::NhlStatsApi::new().get_nhl_seasons(&pool).await?;
    Ok(())
}
