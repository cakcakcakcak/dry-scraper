//! Represents an NHL season with various configuration flags and metadata.
//!
//! This struct is used to deserialize season data from external sources (such as the NHL API),
//! and to persist season information in the database. It includes fields for season identifiers,
//! configuration flags (such as whether the all-star game or conferences are in use), date fields,
//! and other season-specific metadata.
//!
//! # Fields
//! - `id`: Unique identifier for the season, deserialized as a string.
//! - `all_star_game_in_use`: Indicates if the all-star game is in use for this season.
//! - `conferences_in_use`: Indicates if conferences are in use for this season.
//! - `divisions_in_use`: Indicates if divisions are in use for this season.
//! - `end_date`: The end date of the season.
//! - `entry_draft_in_use`: Indicates if the entry draft is in use for this season.
//! - `formatted_season_id`: Human-readable season identifier (e.g., "2023-24").
//! - `minimum_playoff_minutes_for_goalie_stats_leaders`: Minimum playoff minutes required for goalie stats leaders.
//! - `minimum_regular_games_for_goalie_stats_leaders`: Minimum regular season games required for goalie stats leaders.
//! - `nhl_stanley_cup_owner`: Indicates if the NHL owns the Stanley Cup for this season.
//! - `number_of_games`: Number of games in the season.
//! - `olympics_participation`: Indicates if Olympics participation is allowed for this season.
//! - `point_for_ot_loss_in_use`: Indicates if points for overtime loss are in use.
//! - `preseason_startdate`: Optional start date for the preseason.
//! - `regular_season_end_date`: End date of the regular season.
//! - `row_in_use`: Indicates if the "ROW" (Regulation plus Overtime Wins) tiebreaker is in use.
//! - `season_ordinal`: Ordinal number of the season.
//! - `start_date`: Start date of the season.
//! - `supplemental_draft_in_use`: Indicates if the supplemental draft is in use.
//! - `ties_in_use`: Indicates if ties are allowed in this season.
//! - `total_playoff_games`: Total number of playoff games in the season.
//! - `total_regular_season_games`: Total number of regular season games.
//! - `wildcard_in_use`: Indicates if the wildcard playoff format is in use.
//! - `api_cache_endpoint`: Optional API cache endpoint for this season.
//! - `raw_json`: Optional raw JSON data for this season.
//! - `last_updated`: Optional timestamp of the last update.
//!
//! # Methods
//! - `upsert`: Asynchronously inserts or updates the season in the database using an upsert operation.
//!
//! # Example
//! ```rust
//! let season: NhlSeason = serde_json::from_value(json_data).unwrap();
//! season.upsert(&pool).await?;
//! ```

use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::serde_helpers::int_to_bool;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlSeason {
    pub id: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub all_star_game_in_use: bool,
    #[serde(deserialize_with = "int_to_bool")]
    pub conferences_in_use: bool,
    #[serde(deserialize_with = "int_to_bool")]
    pub divisions_in_use: bool,
    pub end_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "int_to_bool")]
    pub entry_draft_in_use: bool,
    pub formatted_season_id: String,
    pub minimum_playoff_minutes_for_goalie_stats_leaders: i32,
    pub minimum_regular_games_for_goalie_stats_leaders: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub nhl_stanley_cup_owner: bool,
    pub number_of_games: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub olympics_participation: bool,
    #[serde(deserialize_with = "int_to_bool")]
    #[serde(rename = "pointForOTLossInUse")]
    pub point_for_ot_loss_in_use: bool,
    pub preseason_startdate: Option<chrono::NaiveDateTime>,
    pub regular_season_end_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "int_to_bool")]
    pub row_in_use: bool,
    pub season_ordinal: i32,
    pub start_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "int_to_bool")]
    pub supplemental_draft_in_use: bool,
    #[serde(deserialize_with = "int_to_bool")]
    pub ties_in_use: bool,
    pub total_playoff_games: i32,
    pub total_regular_season_games: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub wildcard_in_use: bool,
    pub api_cache_endpoint: Option<String>,
    pub raw_json: Option<serde_json::Value>,
    pub last_updated: Option<chrono::NaiveDateTime>,
}

impl NhlSeason {
    pub async fn upsert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
        sqlx_operation_with_retries! (
            sqlx::query(r#"INSERT INTO nhl_season (
                                        id, 
                                        all_star_game_in_use, 
                                        conferences_in_use, 
                                        divisions_in_use, 
                                        end_date, 
                                        entry_draft_in_use, 
                                        formatted_season_id, 
                                        minimum_playoff_minutes_for_goalie_stats_leaders, 
                                        minimum_regular_games_for_goalie_stats_leaders, 
                                        nhl_stanley_cup_owner, 
                                        number_of_games, 
                                        olympics_participation, 
                                        point_for_ot_loss_in_use, 
                                        preseason_startdate, 
                                        regular_season_end_date, 
                                        row_in_use, 
                                        season_ordinal, 
                                        start_date, 
                                        supplemental_draft_in_use, 
                                        ties_in_use, 
                                        total_playoff_games, 
                                        total_regular_season_games, 
                                        wildcard_in_use,
                                        raw_json,
                                        api_cache_endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,
                                        $14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
                                    ON CONFLICT (id) DO UPDATE SET 
                                        all_star_game_in_use = EXCLUDED.all_star_game_in_use,
                                        conferences_in_use = EXCLUDED.conferences_in_use, 
                                        divisions_in_use = EXCLUDED.divisions_in_use,
                                        end_date = EXCLUDED.end_date,
                                        entry_draft_in_use = EXCLUDED.entry_draft_in_use,
                                        formatted_season_id = EXCLUDED.formatted_season_id,
                                        minimum_playoff_minutes_for_goalie_stats_leaders = EXCLUDED.minimum_playoff_minutes_for_goalie_stats_leaders,
                                        minimum_regular_games_for_goalie_stats_leaders = EXCLUDED.minimum_regular_games_for_goalie_stats_leaders,
                                        nhl_stanley_cup_owner = EXCLUDED.nhl_stanley_cup_owner,
                                        number_of_games = EXCLUDED.number_of_games,
                                        olympics_participation = EXCLUDED.olympics_participation,
                                        point_for_ot_loss_in_use = EXCLUDED.point_for_ot_loss_in_use,
                                        preseason_startdate = EXCLUDED.preseason_startdate,
                                        regular_season_end_date = EXCLUDED.regular_season_end_date,
                                        row_in_use = EXCLUDED.row_in_use,
                                        season_ordinal = EXCLUDED.season_ordinal,
                                        start_date = EXCLUDED.start_date,
                                        supplemental_draft_in_use = EXCLUDED.supplemental_draft_in_use,
                                        ties_in_use = EXCLUDED.ties_in_use,
                                        total_playoff_games = EXCLUDED.total_playoff_games,
                                        total_regular_season_games = EXCLUDED.total_regular_season_games,
                                        wildcard_in_use = EXCLUDED.wildcard_in_use,
                                        raw_json = EXCLUDED.raw_json,
                                        api_cache_endpoint = EXCLUDED.api_cache_endpoint,
                                        last_updated = now()
                                    "#)
                    .bind(self.id)
                    .bind(self.all_star_game_in_use)
                    .bind(self.conferences_in_use)
                    .bind(self.divisions_in_use)
                    .bind(self.end_date)
                    .bind(self.entry_draft_in_use)
                    .bind(&self.formatted_season_id)
                    .bind(self.minimum_playoff_minutes_for_goalie_stats_leaders)
                    .bind(self.minimum_regular_games_for_goalie_stats_leaders)
                    .bind(self.nhl_stanley_cup_owner)
                    .bind(self.number_of_games)
                    .bind(self.olympics_participation)
                    .bind(self.point_for_ot_loss_in_use)
                    .bind(&self.preseason_startdate)
                    .bind(self.regular_season_end_date)
                    .bind(self.row_in_use)
                    .bind(self.season_ordinal)
                    .bind(self.start_date)
                    .bind(self.supplemental_draft_in_use)
                    .bind(self.ties_in_use)
                    .bind(self.total_playoff_games)
                    .bind(self.total_regular_season_games)
                    .bind(self.wildcard_in_use)
                    .bind(&self.raw_json)
                    .bind(&self.api_cache_endpoint)
                    .execute(pool).await
        ).await?;
        Ok(())
    }
}
