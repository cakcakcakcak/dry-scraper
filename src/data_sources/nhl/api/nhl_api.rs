use std::fmt::Debug;

use futures::stream::{self, StreamExt};

use super::{nhl_stats_api::NhlStatsApi, nhl_web_api::NhlWebApi};
use crate::{
    common::{
        api::CacheableApi,
        app_context::AppContext,
        db::DbContext,
        models::ItemParsedWithContext,
        rate_limiter::{RateLimiter, RateLimiterConfig},
    },
    data_sources::models::{
        NhlFranchiseJson, NhlGameJson, NhlPlayerJson, NhlPlayoffBracketJson,
        NhlPlayoffBracketSeriesJson, NhlPlayoffSeriesJson, NhlSeasonContext, NhlSeasonJson,
        NhlShiftJson, NhlTeamJson,
    },
    DSError,
};

pub struct NhlApi {
    nhl_stats_api: NhlStatsApi,
    nhl_web_api: NhlWebApi,
}

impl Debug for NhlApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NhlApi")
            .field("nhl_stats_api", &self.nhl_stats_api)
            .field("nhl_web_api", &self.nhl_web_api)
            .finish()
    }
}
impl NhlApi {
    pub fn new(rate_limiter_config: RateLimiterConfig) -> Self {
        let rate_limiter = RateLimiter::new(rate_limiter_config);
        Self {
            nhl_stats_api: NhlStatsApi::new(rate_limiter.clone()),
            nhl_web_api: NhlWebApi::new(rate_limiter),
        }
    }

    // Season methods
    pub async fn get_list_seasons(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<Result<ItemParsedWithContext<NhlSeasonJson>, DSError>>, DSError> {
        let endpoint = self.nhl_stats_api.endpoint("/season");
        self.nhl_stats_api
            .fetch_and_parse(&endpoint, db_context)
            .await
    }

    // Team methods
    pub async fn get_list_teams(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<Result<ItemParsedWithContext<NhlTeamJson>, DSError>>, DSError> {
        let endpoint = self.nhl_stats_api.endpoint("/team");
        self.nhl_stats_api
            .fetch_and_parse(&endpoint, db_context)
            .await
    }

    pub async fn get_team(
        &self,
        db_context: &DbContext,
        team_id: i32,
    ) -> Result<ItemParsedWithContext<NhlTeamJson>, DSError> {
        let endpoint = self.nhl_stats_api.endpoint(&format!("/team/id/{team_id}"));
        let mut results = self
            .nhl_stats_api
            .fetch_and_parse(&endpoint, db_context)
            .await?;

        results
            .pop()
            .ok_or_else(|| DSError::ApiCustom(format!("NHL team with id {team_id} not found.")))?
    }

    // Franchise methods
    pub async fn get_list_franchises(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<Result<ItemParsedWithContext<NhlFranchiseJson>, DSError>>, DSError> {
        let endpoint = self.nhl_stats_api.endpoint("/franchise");
        self.nhl_stats_api
            .fetch_and_parse(&endpoint, db_context)
            .await
    }

    // Shift methods
    pub async fn get_list_shifts_for_game(
        &self,
        db_context: &DbContext,
        game_id: i32,
    ) -> Result<Vec<Result<ItemParsedWithContext<NhlShiftJson>, DSError>>, DSError> {
        let endpoint = self
            .nhl_stats_api
            .endpoint(&format!("/shiftcharts?cayenneExp=gameId={game_id}"));
        self.nhl_stats_api
            .fetch_and_parse(&endpoint, db_context)
            .await
    }

    // Player methods
    pub async fn get_player(
        &self,
        db_context: &DbContext,
        player_id: i32,
    ) -> Result<ItemParsedWithContext<NhlPlayerJson>, DSError> {
        let endpoint = self
            .nhl_web_api
            .endpoint(&format!("/player/{player_id}/landing"));
        self.nhl_web_api
            .fetch_and_parse::<NhlPlayerJson>(&endpoint, db_context)
            .await
    }

    pub async fn get_many_players(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
        player_ids: Vec<i32>,
    ) -> Vec<Result<ItemParsedWithContext<NhlPlayerJson>, DSError>> {
        let count = player_ids.len();
        tracing::debug!(count, "Fetching players");

        let result = stream::iter(player_ids)
            .map(|player_id| self.get_player(db_context, player_id))
            .buffer_unordered(app_context.config.api_concurrency_limit)
            .collect()
            .await;
        result
    }

    // Game methods
    pub async fn get_game(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
        game_id: i32,
    ) -> Result<ItemParsedWithContext<NhlGameJson>, DSError> {
        app_context.inc_progress(1);

        let endpoint = self
            .nhl_web_api
            .endpoint(&format!("/gamecenter/{game_id}/play-by-play"));
        self.nhl_web_api
            .fetch_and_parse::<NhlGameJson>(&endpoint, db_context)
            .await
    }

    pub async fn get_many_games(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
        game_ids: Vec<i32>,
    ) -> Vec<Result<ItemParsedWithContext<NhlGameJson>, DSError>> {
        let count = game_ids.len();
        tracing::debug!(count = game_ids.len(), "Fetching games");

        app_context.init_progress(Some(count), "Fetching games");
        let results = stream::iter(game_ids)
            .map(|game_id| self.get_game(app_context, db_context, game_id))
            .buffer_unordered(app_context.config.api_concurrency_limit)
            .collect()
            .await;
        app_context.finish_progress();

        results
    }

    // Playoff bracket methods
    pub async fn get_list_playoff_series_for_year(
        &self,
        db_context: &DbContext,
        year_id: i32,
    ) -> Result<Vec<Result<ItemParsedWithContext<NhlPlayoffBracketSeriesJson>, DSError>>, DSError>
    {
        let endpoint = self
            .nhl_web_api
            .endpoint(&format!("/playoff-bracket/{year_id}"));

        let raw_data = self
            .nhl_web_api
            .fetch_endpoint_cached(db_context, &endpoint)
            .await?;
        let bracket: NhlPlayoffBracketJson = serde_json::from_str(&raw_data).map_err(|e| {
            tracing::warn!(
                endpoint,
                "Failed to parse into `NhlPlayoffBracketJson`: {e}"
            );
            tracing::info!(raw_data);
            DSError::Serde(e)
        })?;

        let season_id: i32 = format!("{}{}", year_id - 1, year_id)
            .parse::<i32>()
            .map_err(DSError::Parse)?;
        Ok(bracket
            .series
            .into_iter()
            .map(|series| {
                let raw_json: serde_json::Value = serde_json::to_value(&series).map_err(|e| {
                    tracing::warn!(error = %e, "Failed to serialize playoff series to JSON");
                    DSError::Serde(e)
                })?;
                Ok(ItemParsedWithContext {
                    item: series,
                    context: NhlSeasonContext {
                        season_id,
                        endpoint: endpoint.clone(),
                        raw_json,
                    },
                })
            })
            .collect())
    }

    // Playoff series methods
    pub async fn get_playoff_series(
        &self,
        db_context: &DbContext,
        season_id: i32,
        series_letter: &str,
    ) -> Result<ItemParsedWithContext<NhlPlayoffSeriesJson>, DSError> {
        let endpoint = self.nhl_web_api.endpoint(&format!(
            "/schedule/playoff-series/{season_id}/{series_letter}"
        ));
        self.nhl_web_api
            .fetch_and_parse::<NhlPlayoffSeriesJson>(&endpoint, db_context)
            .await
    }
}
