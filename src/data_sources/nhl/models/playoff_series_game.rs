use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::super::primary_key::*;
use crate::{
    bind,
    common::{
        db::{CacheKey, DbEntity, StaticPgQuery, StaticPgQueryAs},
        models::traits::IntoDbStruct,
    },
    data_sources::models::{
        GameOutcomeJson, GameType, LocalizedNameJson, LocalizedNameJsonExt,
        NhlPlayoffSeriesContext, PeriodDescriptorJson, PeriodTypeJson, TvBroadcastsJson,
    },
    impl_has_type_name, impl_pk_debug,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeriesGameTeamJson {
    pub id: i32,
    pub common_name: LocalizedNameJson,
    pub place_name: Option<LocalizedNameJson>,
    pub place_name_with_preposition: Option<LocalizedNameJson>,
    pub abbrev: String,
    pub score: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeriesStatusJson {
    pub top_seed_wins: i32,
    pub bottom_seed_wins: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeriesGameJson {
    pub id: i32,
    pub season: i32,
    pub game_type: GameType,
    pub game_number: i32,
    pub if_necessary: bool,
    pub venue: LocalizedNameJson,
    pub neutral_site: bool,
    #[serde(rename = "startTimeUTC")]
    pub start_time_utc: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "easternUTCOffset")]
    pub eastern_utc_offset: String,
    #[serde(rename = "venueUTCOffset")]
    pub venue_utc_offset: String,
    pub venue_timezone: String,
    pub game_state: String,
    pub game_schedule_state: String,
    pub tv_broadcasts: Vec<TvBroadcastsJson>,
    pub away_team: NhlPlayoffSeriesGameTeamJson,
    pub home_team: NhlPlayoffSeriesGameTeamJson,
    pub game_center_link: String,
    pub period_descriptor: PeriodDescriptorJson,
    pub series_status: NhlPlayoffSeriesStatusJson,
    pub game_outcome: GameOutcomeJson,
}
impl IntoDbStruct for NhlPlayoffSeriesGameJson {
    type DbStruct = NhlPlayoffSeriesGame;
    type Context = NhlPlayoffSeriesContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlPlayoffSeriesGameJson {
            id,
            season: season_id,
            game_type,
            game_number,
            if_necessary,
            venue,
            neutral_site,
            start_time_utc,
            eastern_utc_offset,
            venue_utc_offset,
            venue_timezone,
            game_state: _,
            game_schedule_state: _,
            tv_broadcasts: _,
            away_team,
            home_team,
            game_center_link,
            period_descriptor,
            series_status,
            game_outcome,
        } = self;
        let NhlPlayoffSeriesGameTeamJson {
            id: away_team_id,
            common_name: away_team_common_name,
            place_name: away_team_place_name,
            place_name_with_preposition: away_team_place_name_with_preposition,
            abbrev: away_team_abbreviation,
            score: away_team_score,
        } = away_team;
        let NhlPlayoffSeriesGameTeamJson {
            id: home_team_id,
            common_name: home_team_common_name,
            place_name: home_team_place_name,
            place_name_with_preposition: home_team_place_name_with_preposition,
            abbrev: home_team_abbreviation,
            score: home_team_score,
        } = home_team;
        let PeriodDescriptorJson {
            number: period_descriptor_number,
            period_type: period_descriptor_type,
            max_regulation_periods: period_descriptor_max_regulation_periods,
        } = period_descriptor;
        let NhlPlayoffSeriesStatusJson {
            bottom_seed_wins,
            top_seed_wins,
        } = series_status;
        let GameOutcomeJson {
            last_period_type: game_outcome_last_period_type,
        } = game_outcome;
        let NhlPlayoffSeriesContext {
            series_letter,
            endpoint,
            raw_json,
        } = context;
        NhlPlayoffSeriesGame {
            id,
            season_id,
            game_type,
            game_number,
            if_necessary,
            venue_name: venue.best_str(),
            neutral_site,
            start_time_utc,
            eastern_utc_offset,
            venue_utc_offset,
            venue_timezone,
            away_team_id,
            away_team_common_name: away_team_common_name.best_str(),
            away_team_place_name: away_team_place_name.best_str_or_none(),
            away_team_place_name_with_preposition: away_team_place_name_with_preposition
                .best_str_or_none(),
            away_team_abbreviation,
            away_team_score,
            home_team_id,
            home_team_common_name: home_team_common_name.best_str(),
            home_team_place_name: home_team_place_name.best_str_or_none(),
            home_team_place_name_with_preposition: home_team_place_name_with_preposition
                .best_str_or_none(),
            home_team_abbreviation,
            home_team_score,
            game_center_link,
            period_descriptor_number,
            period_descriptor_type,
            period_descriptor_max_regulation_periods,
            top_seed_wins,
            bottom_seed_wins,
            game_outcome_last_period_type,
            series_letter,
            raw_json,
            endpoint,
        }
    }
}

#[derive(Clone, FromRow)]
pub struct NhlPlayoffSeriesGame {
    pub id: i32,
    pub season_id: i32,
    pub game_type: GameType,
    pub game_number: i32,
    pub if_necessary: bool,
    pub venue_name: String,
    pub neutral_site: bool,
    pub start_time_utc: chrono::DateTime<chrono::Utc>,
    pub eastern_utc_offset: String,
    pub venue_utc_offset: String,
    pub venue_timezone: String,
    pub away_team_id: i32,
    pub away_team_common_name: String,
    pub away_team_place_name: Option<String>,
    pub away_team_place_name_with_preposition: Option<String>,
    pub away_team_abbreviation: String,
    pub away_team_score: i32,
    pub home_team_id: i32,
    pub home_team_common_name: String,
    pub home_team_place_name: Option<String>,
    pub home_team_place_name_with_preposition: Option<String>,
    pub home_team_abbreviation: String,
    pub home_team_score: i32,
    pub game_center_link: String,
    pub period_descriptor_number: i32,
    pub period_descriptor_type: PeriodTypeJson,
    pub period_descriptor_max_regulation_periods: i32,
    pub top_seed_wins: i32,
    pub bottom_seed_wins: i32,
    pub game_outcome_last_period_type: PeriodTypeJson,
    pub series_letter: String,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
}
#[async_trait]
impl DbEntity for NhlPlayoffSeriesGame {
    type Pk = NhlPlayoffSeriesGameKey;

    fn pk(&self) -> Self::Pk {
        NhlPlayoffSeriesGameKey { id: self.id }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT id from nhl_playoff_series_game")
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        vec![
            CacheKey {
                source: "api_cache",
                table: "api_cache",
                id: self.endpoint.clone(),
            },
            CacheKey {
                source: "nhl",
                table: "season",
                id: self.season_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "team",
                id: self.away_team_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "team",
                id: self.home_team_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "playoff_series",
                id: format!("{}:{}", self.season_id, self.series_letter),
            },
            CacheKey {
                source: "nhl",
                table: "playoff_bracket_series",
                id: format!("{}:{}", self.season_id, self.series_letter),
            },
        ]
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_playoff_series_game (
                                    id,
                                    season_id,
                                    game_type,
                                    game_number,
                                    if_necessary,
                                    venue_name,
                                    neutral_site,
                                    start_time_utc,
                                    eastern_utc_offset,
                                    venue_utc_offset,
                                    venue_timezone,
                                    away_team_id,
                                    away_team_common_name,
                                    away_team_place_name,
                                    away_team_place_name_with_preposition,
                                    away_team_abbreviation,
                                    away_team_score,
                                    home_team_id,
                                    home_team_common_name,
                                    home_team_place_name,
                                    home_team_place_name_with_preposition,
                                    home_team_abbreviation,
                                    home_team_score,
                                    game_center_link,
                                    period_descriptor_number,
                                    period_descriptor_type,
                                    period_descriptor_max_regulation_periods,
                                    top_seed_wins,
                                    bottom_seed_wins,
                                    game_outcome_last_period_type,
                                    series_letter,
                                    raw_json,
                                    endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                        $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                                        $31,$32,$33)
                                    ON CONFLICT (id) DO UPDATE SET
                                        season_id = EXCLUDED.season_id,
                                        game_type = EXCLUDED.game_type,
                                        game_number = EXCLUDED.game_number,
                                        if_necessary = EXCLUDED.if_necessary,
                                        venue_name = EXCLUDED.venue_name,
                                        neutral_site = EXCLUDED.neutral_site,
                                        start_time_utc = EXCLUDED.start_time_utc,
                                        eastern_utc_offset = EXCLUDED.eastern_utc_offset,
                                        venue_utc_offset = EXCLUDED.venue_utc_offset,
                                        venue_timezone = EXCLUDED.venue_timezone,
                                        away_team_id = EXCLUDED.away_team_id,
                                        away_team_common_name = EXCLUDED.away_team_common_name,
                                        away_team_place_name = EXCLUDED.away_team_place_name,
                                        away_team_place_name_with_preposition = EXCLUDED.away_team_place_name_with_preposition,
                                        away_team_abbreviation = EXCLUDED.away_team_abbreviation,
                                        away_team_score = EXCLUDED.away_team_score,
                                        home_team_id = EXCLUDED.home_team_id,
                                        home_team_common_name = EXCLUDED.home_team_common_name,
                                        home_team_place_name = EXCLUDED.home_team_place_name,
                                        home_team_place_name_with_preposition = EXCLUDED.home_team_place_name_with_preposition,
                                        home_team_abbreviation = EXCLUDED.home_team_abbreviation,
                                        home_team_score = EXCLUDED.home_team_score,
                                        game_center_link = EXCLUDED.game_center_link,
                                        period_descriptor_number = EXCLUDED.period_descriptor_number,
                                        period_descriptor_type = EXCLUDED.period_descriptor_type,
                                        period_descriptor_max_regulation_periods = EXCLUDED.period_descriptor_max_regulation_periods,
                                        top_seed_wins = EXCLUDED.top_seed_wins,
                                        bottom_seed_wins = EXCLUDED.bottom_seed_wins,
                                        game_outcome_last_period_type = EXCLUDED.game_outcome_last_period_type,
                                        series_letter = EXCLUDED.series_letter,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                "#
            ),
            self.id,
            self.season_id,
            self.game_type,
            self.game_number,
            self.if_necessary,
            self.venue_name,
            self.neutral_site,
            self.start_time_utc,
            self.eastern_utc_offset,
            self.venue_utc_offset,
            self.venue_timezone,
            self.away_team_id,
            self.away_team_common_name,
            self.away_team_place_name,
            self.away_team_place_name_with_preposition,
            self.away_team_abbreviation,
            self.away_team_score,
            self.home_team_id,
            self.home_team_common_name,
            self.home_team_place_name,
            self.home_team_place_name_with_preposition,
            self.home_team_abbreviation,
            self.home_team_score,
            self.game_center_link,
            self.period_descriptor_number,
            self.period_descriptor_type,
            self.period_descriptor_max_regulation_periods,
            self.top_seed_wins,
            self.bottom_seed_wins,
            self.game_outcome_last_period_type,
            self.series_letter,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlPlayoffSeriesGameJson);
impl_has_type_name!(NhlPlayoffSeriesGame);
impl_pk_debug!(NhlPlayoffSeriesGame);
