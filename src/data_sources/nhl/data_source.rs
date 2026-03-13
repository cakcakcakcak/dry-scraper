use crate::common::{
    app_context::AppContext,
    data_source::DataSource,
    db::{DbContext, DbEntity},
    errors::DSError,
    models::ApiCache,
};
use crate::config::Config;

use super::api::NhlApi;
use super::models::*;
use futures::stream::{self, StreamExt};

#[allow(dead_code)]
pub struct NhlDataSource {
    pub api: NhlApi,
}

impl NhlDataSource {
    pub fn new() -> Self {
        Self::with_config(&Config::from_env_and_args())
    }

    pub fn with_config(config: &Config) -> Self {
        Self {
            api: NhlApi::with_config(config),
        }
    }
}

impl Default for NhlDataSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DataSource for NhlDataSource {
    fn name(&self) -> &'static str {
        "nhl"
    }

    async fn warm_cache(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
    ) -> Result<(), DSError> {
        tracing::debug!("warming NHL database key cache");

        // Clone DbContext to move into futures (they need to be 'static for buffer_unordered).
        let db_ctx = db_context.clone();

        // warm all entity caches concurrently using buffer_unordered with AppContext's concurrency limit
        let cache_warmers = vec![
            ApiCache::warm_key_cache(&db_ctx),
            NhlSeason::warm_key_cache(&db_ctx),
            NhlFranchise::warm_key_cache(&db_ctx),
            NhlTeam::warm_key_cache(&db_ctx),
            NhlPlayer::warm_key_cache(&db_ctx),
            NhlGame::warm_key_cache(&db_ctx),
            NhlRosterSpot::warm_key_cache(&db_ctx),
            NhlPlay::warm_key_cache(&db_ctx),
            NhlShift::warm_key_cache(&db_ctx),
            NhlPlayoffBracketSeries::warm_key_cache(&db_ctx),
            NhlPlayoffSeries::warm_key_cache(&db_ctx),
            NhlPlayoffSeriesGame::warm_key_cache(&db_ctx),
        ];

        stream::iter(cache_warmers)
            .buffer_unordered(app_context.config.db_concurrency_limit)
            .collect::<Vec<_>>()
            .await;

        tracing::debug!("NHL key cache warmed");
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
