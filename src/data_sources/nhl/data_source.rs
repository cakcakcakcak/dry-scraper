use crate::{
    common::{
        app_context::AppContext, data_source::DataSource, db::{DbContext, DbEntity}, errors::DSError,
        models::ApiCache,
    },
};

use super::api::NhlApi;
use super::models::*;

#[allow(dead_code)]
pub struct NhlDataSource {
    api: NhlApi,
}

impl NhlDataSource {
    pub fn new() -> Self {
        Self {
            api: NhlApi::new(),
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
        _app_context: &AppContext,
        db_context: &DbContext,
    ) -> Result<(), DSError> {
        tracing::debug!("warming NHL database key cache");

        // warm all entity caches
        let _ = ApiCache::warm_key_cache(db_context).await;
        let _ = NhlSeason::warm_key_cache(db_context).await;
        let _ = NhlFranchise::warm_key_cache(db_context).await;
        let _ = NhlTeam::warm_key_cache(db_context).await;
        let _ = NhlPlayer::warm_key_cache(db_context).await;
        let _ = NhlGame::warm_key_cache(db_context).await;
        let _ = NhlRosterSpot::warm_key_cache(db_context).await;
        let _ = NhlPlay::warm_key_cache(db_context).await;
        let _ = NhlShift::warm_key_cache(db_context).await;
        let _ = NhlPlayoffBracketSeries::warm_key_cache(db_context).await;
        let _ = NhlPlayoffSeries::warm_key_cache(db_context).await;
        let _ = NhlPlayoffSeriesGame::warm_key_cache(db_context).await;

        tracing::debug!("NHL key cache warmed");
        Ok(())
    }
}
