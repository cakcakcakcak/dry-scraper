use std::str::FromStr;

use async_trait::async_trait;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Execute as _;
use sqlx::Executor as _;
use sqlx::FromRow;
use sqlx::postgres::Postgres;

use crate::api::cacheable_api::CacheableApi;
use crate::api::nhl::nhl_stats_api::NhlStatsApi;
use crate::db::DbPool;
use crate::db::persistable::Persistable;
use crate::lp_error::LPError;
use crate::models::nhl::nhl_model_common::{
    GameType, LocalizedNameJson, PeriodDescriptorJson, PeriodTypeJson,
};
use crate::models::nhl::nhl_play::NhlPlayJson;
use crate::models::nhl::nhl_roster_spot::NhlRosterSpotJson;
use crate::models::traits::{DbStruct, IntoDbStruct};

use crate::impl_has_type_name;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ClockJson {
    pub time_remaining: String,
    pub seconds_remaining: i32,
    pub running: bool,
    pub in_intermission: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TvBroadcastsJson {
    pub id: i32,
    pub market: String,
    pub country_code: String,
    pub network: String,
    pub sequence_number: i32,
}
#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GameOutcomeJson {
    pub last_period_type: PeriodTypeJson,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TeamGameJson {
    pub id: i32,
    pub common_name: LocalizedNameJson,
    pub abbrev: String,
    pub score: i32,
    pub sog: Option<i32>,
    pub logo: String,
    pub dark_logo: String,
    pub place_name: LocalizedNameJson,
    pub place_name_with_preposition: LocalizedNameJson,
}
#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlGameJson {
    pub id: i32,
    pub season: i32,
    pub game_type: GameType,
    pub limited_scoring: bool,
    pub game_date: chrono::NaiveDate,
    pub venue: LocalizedNameJson,
    pub venue_location: LocalizedNameJson,
    #[serde(rename = "startTimeUTC")]
    pub start_time_utc: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "easternUTCOffset")]
    pub eastern_utc_offset: String,
    #[serde(rename = "venueUTCOffset")]
    pub venue_utc_offset: String,
    pub tv_broadcasts: Vec<TvBroadcastsJson>,
    pub game_state: String,
    pub game_schedule_state: String,
    pub period_descriptor: PeriodDescriptorJson,
    pub away_team: TeamGameJson,
    pub home_team: TeamGameJson,
    pub shootout_in_use: bool,
    pub ot_in_use: bool,
    pub clock: ClockJson,
    pub display_period: i32,
    pub max_periods: i32,
    pub game_outcome: GameOutcomeJson,
    pub plays: Vec<NhlPlayJson>,
    pub roster_spots: Vec<NhlRosterSpotJson>,
    pub reg_periods: i32,
}
impl IntoDbStruct for NhlGameJson {
    type U = NhlGame;

    fn to_db_struct(self) -> Self::U {
        let NhlGameJson {
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
            game_state,
            game_schedule_state,
            period_descriptor,
            away_team,
            home_team,
            tv_broadcasts,
            shootout_in_use,
            ot_in_use,
            clock,
            display_period,
            max_periods,
            game_outcome,
            plays,
            roster_spots,
            reg_periods,
        } = self;
        NhlGame {
            id,
            season,
            game_type,
            limited_scoring,
            game_date,
            venue: venue.default,
            venue_location: venue_location.default,
            start_time_utc,
            eastern_utc_offset,
            venue_utc_offset,
            period_descriptor_number: period_descriptor.number,
            period_descriptor_type: period_descriptor.period_type,
            period_descriptor_max_regulation_periods: period_descriptor.max_regulation_periods,
            away_team_id: away_team.id,
            away_team_name: away_team.common_name.default,
            away_team_abbrev: away_team.abbrev,
            away_team_score: away_team.score,
            away_team_sog: away_team.sog,
            away_team_logo: away_team.logo,
            away_team_dark_logo: away_team.dark_logo,
            away_team_place_name: away_team.place_name.default,
            away_team_place_name_with_preposition: away_team.place_name_with_preposition.default,
            home_team_id: home_team.id,
            home_team_name: home_team.common_name.default,
            home_team_abbrev: home_team.abbrev,
            home_team_score: home_team.score,
            home_team_sog: home_team.sog,
            home_team_logo: home_team.logo,
            home_team_dark_logo: home_team.dark_logo,
            home_team_place_name: home_team.place_name.default,
            home_team_place_name_with_preposition: home_team.place_name_with_preposition.default,
            shootout_in_use,
            ot_in_use,
            display_period,
            max_periods,
            game_outcome_last_period_type: game_outcome.last_period_type,
            reg_periods,
            endpoint: String::new(),
            raw_json: serde_json::Value::Null,
            last_updated: None,
        }
    }
}
#[derive(Debug, FromRow, Clone)]
pub struct NhlGame {
    pub id: i32,
    pub season: i32,
    pub game_type: GameType,
    pub limited_scoring: bool,
    pub game_date: chrono::NaiveDate,
    pub venue: String,
    pub venue_location: String,
    pub start_time_utc: chrono::DateTime<chrono::Utc>,
    pub eastern_utc_offset: String,
    pub venue_utc_offset: String,
    pub period_descriptor_number: i32,
    pub period_descriptor_type: PeriodTypeJson,
    pub period_descriptor_max_regulation_periods: i32,
    pub away_team_id: i32,
    pub away_team_name: String,
    pub away_team_abbrev: String,
    pub away_team_score: i32,
    pub away_team_sog: Option<i32>,
    pub away_team_logo: String,
    pub away_team_dark_logo: String,
    pub away_team_place_name: String,
    pub away_team_place_name_with_preposition: String,
    pub home_team_id: i32,
    pub home_team_name: String,
    pub home_team_abbrev: String,
    pub home_team_score: i32,
    pub home_team_sog: Option<i32>,
    pub home_team_logo: String,
    pub home_team_dark_logo: String,
    pub home_team_place_name: String,
    pub home_team_place_name_with_preposition: String,
    pub shootout_in_use: bool,
    pub ot_in_use: bool,
    pub display_period: i32,
    pub max_periods: i32,
    pub game_outcome_last_period_type: PeriodTypeJson,
    pub reg_periods: i32,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlGame {
    fn fill_context(&mut self, endpoint: String, raw_data: String) -> Result<(), LPError> {
        self.endpoint = endpoint;

        let raw_json = serde_json::Value::from_str(&raw_data)?;
        self.raw_json = raw_json;
        Ok(())
    }
}
#[async_trait]
impl Persistable for NhlGame {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM nhl_game WHERE id=$1"#)
                .bind(id)
                .fetch_optional(pool)
                .await
        )
        .await
        .map_err(LPError::from)
    }

    #[tracing::instrument(skip(pool))]
    async fn upsert(&self, pool: &DbPool) -> Result<(), LPError> {
        sqlx_operation_with_retries!(self.create_query().execute(pool).await).await?;
        Ok(())
    }

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
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
                                        endpoint,
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
                                        endpoint = EXCLUDED.endpoint,
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
            .bind(&self.endpoint)
            .bind(&self.raw_json)
    }
}

impl_has_type_name!(NhlGameJson);
impl_has_type_name!(NhlGame);
