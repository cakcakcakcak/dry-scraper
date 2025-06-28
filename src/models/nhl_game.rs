use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::models::game_type::GameType;
use crate::models::period_type::PeriodType;
use crate::serde_helpers::deserialize_default_to_string;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlGame {
    pub id: i32,
    pub season: i32,
    pub game_type: GameType,
    pub limited_scoring: bool,
    pub game_date: chrono::NaiveDate,
    #[serde(deserialize_with = "deserialize_default_to_string")]
    pub venue: String,
    #[serde(deserialize_with = "deserialize_default_to_string")]
    pub venue_location: String,
    #[serde(rename = "startTimeUTC")]
    pub start_time_utc: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "easternUTCOffset")]
    pub eastern_utc_offset: String,
    #[serde(rename = "venueUTCOffset")]
    pub venue_utc_offset: String,
    pub period_descriptor_number: Option<i32>,
    pub period_descriptor_type: Option<PeriodType>,
    pub period_descriptor_max_regulation_periods: Option<i32>,
    pub away_team_id: Option<i32>,
    pub away_team_name: Option<String>,
    pub away_team_abbrev: Option<String>,
    pub away_team_score: Option<i32>,
    pub away_team_sog: Option<i32>,
    pub away_team_logo: Option<String>,
    pub away_team_dark_logo: Option<String>,
    pub away_team_place_name: Option<String>,
    pub away_team_place_name_with_preposition: Option<String>,
    pub home_team_id: Option<i32>,
    pub home_team_name: Option<String>,
    pub home_team_abbrev: Option<String>,
    pub home_team_score: Option<i32>,
    pub home_team_sog: Option<i32>,
    pub home_team_logo: Option<String>,
    pub home_team_dark_logo: Option<String>,
    pub home_team_place_name: Option<String>,
    pub home_team_place_name_with_preposition: Option<String>,
    pub shootout_in_use: bool,
    pub ot_in_use: bool,
    pub display_period: i32,
    pub max_periods: Option<i32>,
    pub game_outcome_last_period_type: Option<PeriodType>,
    pub reg_periods: i32,
    pub api_cache_endpoint: Option<String>,
    pub raw_json: Option<serde_json::Value>,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl NhlGame {
    pub async fn upsert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
        sqlx_operation_with_retries! (
            sqlx::query(r#"INSERT INTO nhl_game (
                                        id,
                                        season,
                                        game_type,
                                        limited_scoring,
                                        game_date,
                                        venue,
                                        venue_location,
                                        start_time_utc,
                                        eastern_utc_offset,
                                        venue_utc_offset,
                                        period_descriptor_number,
                                        period_descriptor_type,
                                        period_descriptor_max_regulation_periods,
                                        away_team_id,
                                        away_team_name,
                                        away_team_abbrev,
                                        away_team_score,
                                        away_team_sog,
                                        away_team_logo,
                                        away_team_dark_logo,
                                        away_team_place_name,
                                        away_team_place_name_with_preposition,
                                        home_team_id,
                                        home_team_name,
                                        home_team_abbrev,
                                        home_team_score,
                                        home_team_sog,
                                        home_team_logo,
                                        home_team_dark_logo,
                                        home_team_place_name,
                                        home_team_place_name_with_preposition,
                                        shootout_in_use,
                                        ot_in_use,
                                        display_period,
                                        max_periods,
                                        game_outcome_last_period_type,
                                        reg_periods,
                                        api_cache_endpoint,
                                        raw_json
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                        $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                                        $31,$32,$33,$34,$35,$36,$37,$38,$39)
                                    ON CONFLICT (id) DO UPDATE SET
                                        season = EXCLUDED.season,
                                        game_type = EXCLUDED.game_type,
                                        limited_scoring = EXCLUDED.limited_scoring,
                                        game_date = EXCLUDED.game_date,
                                        venue = EXCLUDED.venue,
                                        venue_location = EXCLUDED.venue_location,
                                        start_time_utc = EXCLUDED.start_time_utc,
                                        eastern_utc_offset = EXCLUDED.eastern_utc_offset,
                                        venue_utc_offset = EXCLUDED.venue_utc_offset,
                                        period_descriptor_number = EXCLUDED.period_descriptor_number,
                                        period_descriptor_type = EXCLUDED.period_descriptor_type,
                                        period_descriptor_max_regulation_periods = EXCLUDED.period_descriptor_max_regulation_periods,
                                        away_team_id = EXCLUDED.away_team_id,
                                        away_team_name = EXCLUDED.away_team_name,
                                        away_team_abbrev = EXCLUDED.away_team_abbrev,
                                        away_team_score = EXCLUDED.away_team_score,
                                        away_team_sog = EXCLUDED.away_team_sog,
                                        away_team_logo = EXCLUDED.away_team_logo,
                                        away_team_dark_logo = EXCLUDED.away_team_dark_logo,
                                        away_team_place_name = EXCLUDED.away_team_place_name,
                                        away_team_place_name_with_preposition = EXCLUDED.away_team_place_name_with_preposition,
                                        home_team_id = EXCLUDED.home_team_id,
                                        home_team_name = EXCLUDED.home_team_name,
                                        home_team_abbrev = EXCLUDED.home_team_abbrev,
                                        home_team_score = EXCLUDED.home_team_score,
                                        home_team_sog = EXCLUDED.home_team_sog,
                                        home_team_logo = EXCLUDED.home_team_logo,
                                        home_team_dark_logo = EXCLUDED.home_team_dark_logo,
                                        home_team_place_name = EXCLUDED.home_team_place_name,
                                        home_team_place_name_with_preposition = EXCLUDED.home_team_place_name_with_preposition,
                                        shootout_in_use = EXCLUDED.shootout_in_use,
                                        ot_in_use = EXCLUDED.ot_in_use,
                                        display_period = EXCLUDED.display_period,
                                        max_periods = EXCLUDED.max_periods,
                                        game_outcome_last_period_type = EXCLUDED.game_outcome_last_period_type,
                                        reg_periods = EXCLUDED.reg_periods,
                                        api_cache_endpoint = EXCLUDED.api_cache_endpoint,
                                        raw_json = EXCLUDED.raw_json,
                                        last_updated = now()
                                    "#
            )
            .bind(&self.id)
            .bind(&self.season)
            .bind(&self.game_type)
            .bind(&self.limited_scoring)
            .bind(&self.game_date)
            .bind(&self.venue)
            .bind(&self.venue_location)
            .bind(&self.start_time_utc)
            .bind(&self.eastern_utc_offset)
            .bind(&self.venue_utc_offset)
            .bind(&self.period_descriptor_number)
            .bind(&self.period_descriptor_type)
            .bind(&self.period_descriptor_max_regulation_periods)
            .bind(&self.away_team_id)
            .bind(&self.away_team_name)
            .bind(&self.away_team_abbrev)
            .bind(&self.away_team_score)
            .bind(&self.away_team_sog)
            .bind(&self.away_team_logo)
            .bind(&self.away_team_dark_logo)
            .bind(&self.away_team_place_name)
            .bind(&self.away_team_place_name_with_preposition)
            .bind(&self.home_team_id)
            .bind(&self.home_team_name)
            .bind(&self.home_team_abbrev)
            .bind(&self.home_team_score)
            .bind(&self.home_team_sog)
            .bind(&self.home_team_logo)
            .bind(&self.home_team_dark_logo)
            .bind(&self.home_team_place_name)
            .bind(&self.home_team_place_name_with_preposition)
            .bind(&self.shootout_in_use)
            .bind(&self.ot_in_use)
            .bind(&self.display_period)
            .bind(&self.max_periods)
            .bind(&self.game_outcome_last_period_type)
            .bind(&self.reg_periods)
            .bind(&self.api_cache_endpoint)
            .bind(&self.raw_json)
            .execute(pool).await
        ).await?;
        Ok(())
    }
}
