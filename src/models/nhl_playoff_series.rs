use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::api::cacheable::CacheableApi;
use crate::api::nhl_stats_api::NhlStatsApi;
use crate::api::nhl_web_api::NhlWebApi;
use crate::lp_error::LPError;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffBracket {
    pub bracket_logo: String,
    pub bracket_logo_fr: String,
    pub series: Vec<NhlPlayoffSeries>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeries {
    pub season_id: Option<i32>,
    pub series_letter: String,
    pub series_url: String,
    pub series_title: String,
    pub series_abbrev: String,
    pub playoff_round: i32,
    pub top_seed_rank: i32,
    pub top_seed_rank_abbrev: String,
    pub top_seed_wins: i32,
    pub bottom_seed_rank: i32,
    pub bottom_seed_rank_abbrev: String,
    pub bottom_seed_wins: i32,
    pub winning_team_id: Option<i32>, // as far as i can tell, only optional because of the 1919 SCF, cancelled due to spanish flu
    pub losing_team_id: Option<i32>,
    pub top_seed_team_id: Option<i32>,
    pub top_seed_team_abbrev: Option<String>,
    pub top_seed_team_name: Option<String>,
    pub top_seed_team_common_name: Option<String>,
    pub top_seed_team_place_name_with_preposition: Option<String>,
    pub top_seed_team_logo: Option<String>,
    pub top_seed_team_dark_logo: Option<String>,
    pub bottom_seed_team_id: Option<i32>,
    pub bottom_seed_team_abbrev: Option<String>,
    pub bottom_seed_team_name: Option<String>,
    pub bottom_seed_team_common_name: Option<String>,
    pub bottom_seed_team_place_name_with_preposition: Option<String>,
    pub bottom_seed_team_logo: Option<String>,
    pub bottom_seed_team_dark_logo: Option<String>,
    pub api_cache_endpoint: Option<String>,
    pub raw_json: Option<serde_json::Value>,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl NhlPlayoffSeries {
    pub async fn verify_relationships(
        &self,
        nhl_stats_api: &NhlStatsApi,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<(), LPError> {
        let _ = nhl_stats_api
            .get_nhl_season(pool, self.season_id.unwrap())
            .await?;
        if let Some(winning_team_id) = self.winning_team_id {
            let _ = nhl_stats_api.get_nhl_team(pool, winning_team_id).await?;
        }
        if let Some(losing_team_id) = self.losing_team_id {
            let _ = nhl_stats_api.get_nhl_team(pool, losing_team_id).await?;
        }
        if let Some(top_seed_team_id) = self.top_seed_team_id {
            let _ = nhl_stats_api.get_nhl_team(pool, top_seed_team_id).await?;
        }
        if let Some(bottom_seed_team_id) = self.bottom_seed_team_id {
            let _ = nhl_stats_api
                .get_nhl_team(pool, bottom_seed_team_id)
                .await?;
        }
        if let Some(endpoint) = &self.api_cache_endpoint {
            let _ = nhl_stats_api.get_or_cache_endpoint(pool, endpoint).await?;
        }
        Ok(())
    }

    pub async fn upsert(
        &self,
        nhl_stats_api: &NhlStatsApi,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<(), LPError> {
        match self.verify_relationships(nhl_stats_api, pool).await {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        sqlx_operation_with_retries! (
            sqlx::query(r#"INSERT INTO nhl_playoff_series (
                                        season_id,
                                        series_letter,
                                        series_url,
                                        series_title,
                                        series_abbreviation,
                                        playoff_round,
                                        top_seed_rank,
                                        top_seed_rank_abbreviation,
                                        top_seed_wins,
                                        bottom_seed_rank,
                                        bottom_seed_rank_abbreviation,
                                        bottom_seed_wins,
                                        winning_team_id,
                                        losing_team_id,
                                        top_seed_team_id,
                                        top_seed_team_abbrev,
                                        top_seed_team_name,
                                        top_seed_team_common_name,
                                        top_seed_team_place_name_with_preposition,
                                        top_seed_team_logo,
                                        top_seed_team_dark_logo,
                                        bottom_seed_team_id,
                                        bottom_seed_team_abbrev,
                                        bottom_seed_team_name,
                                        bottom_seed_team_common_name,
                                        bottom_seed_team_place_name_with_preposition,
                                        bottom_seed_team_logo,
                                        bottom_seed_team_dark_logo,
                                        api_cache_endpoint,
                                        raw_json
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                        $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30)
                                    ON CONFLICT (season_id, series_letter) DO UPDATE SET
                                        series_url = EXCLUDED.series_url,
                                        series_title = EXCLUDED.series_title,
                                        series_abbreviation = EXCLUDED.series_abbreviation,
                                        playoff_round = EXCLUDED.playoff_round,
                                        top_seed_rank = EXCLUDED.top_seed_rank,
                                        top_seed_rank_abbreviation = EXCLUDED.top_seed_rank_abbreviation,
                                        top_seed_wins = EXCLUDED.top_seed_wins,
                                        bottom_seed_rank = EXCLUDED.bottom_seed_rank,
                                        bottom_seed_rank_abbreviation = EXCLUDED.bottom_seed_rank_abbreviation,
                                        bottom_seed_wins = EXCLUDED.bottom_seed_wins,
                                        winning_team_id = EXCLUDED.winning_team_id,
                                        losing_team_id = EXCLUDED.losing_team_id,
                                        top_seed_team_id = EXCLUDED.top_seed_team_id,
                                        top_seed_team_abbrev = EXCLUDED.top_seed_team_abbrev,
                                        top_seed_team_name = EXCLUDED.top_seed_team_name,
                                        top_seed_team_common_name = EXCLUDED.top_seed_team_common_name,
                                        top_seed_team_place_name_with_preposition = EXCLUDED.top_seed_team_place_name_with_preposition,
                                        top_seed_team_logo = EXCLUDED.top_seed_team_logo,
                                        top_seed_team_dark_logo = EXCLUDED.top_seed_team_dark_logo,
                                        bottom_seed_team_id = EXCLUDED.bottom_seed_team_id,
                                        bottom_seed_team_abbrev = EXCLUDED.bottom_seed_team_abbrev,
                                        bottom_seed_team_name = EXCLUDED.bottom_seed_team_name,
                                        bottom_seed_team_common_name = EXCLUDED.bottom_seed_team_common_name,
                                        bottom_seed_team_place_name_with_preposition = EXCLUDED.bottom_seed_team_place_name_with_preposition,
                                        bottom_seed_team_logo = EXCLUDED.bottom_seed_team_logo,
                                        bottom_seed_team_dark_logo = EXCLUDED.bottom_seed_team_dark_logo,
                                        api_cache_endpoint = EXCLUDED.api_cache_endpoint,
                                        raw_json = EXCLUDED.raw_json,
                                        last_updated = now()
                                    "#
            )
            .bind(&self.season_id)
            .bind(&self.series_letter)
            .bind(&self.series_url)
            .bind(&self.series_title)
            .bind(&self.series_abbrev)
            .bind(&self.playoff_round)
            .bind(&self.top_seed_rank)
            .bind(&self.top_seed_rank_abbrev)
            .bind(&self.top_seed_wins)
            .bind(&self.bottom_seed_rank)
            .bind(&self.bottom_seed_rank_abbrev)
            .bind(&self.bottom_seed_wins)
            .bind(&self.winning_team_id)
            .bind(&self.losing_team_id)
            .bind(&self.top_seed_team_id)
            .bind(&self.top_seed_team_abbrev)
            .bind(&self.top_seed_team_name)
            .bind(&self.top_seed_team_common_name)
            .bind(&self.top_seed_team_place_name_with_preposition)
            .bind(&self.top_seed_team_logo)
            .bind(&self.top_seed_team_dark_logo)
            .bind(&self.bottom_seed_team_id)
            .bind(&self.bottom_seed_team_abbrev)
            .bind(&self.bottom_seed_team_name)
            .bind(&self.bottom_seed_team_common_name)
            .bind(&self.bottom_seed_team_place_name_with_preposition)
            .bind(&self.bottom_seed_team_logo)
            .bind(&self.bottom_seed_team_dark_logo)
            .bind(&self.api_cache_endpoint)
            .bind(&self.raw_json)
            .execute(pool).await
        ).await?;
        Ok(())
    }
}
