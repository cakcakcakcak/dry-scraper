use async_trait::async_trait;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::{
    bind,
    common::{
        db::{DbContext, Persistable, PrimaryKey, RelationshipIntegrity, StaticPgQuery},
        errors::LPError,
        models::{
            ApiCache, ApiCacheKey,
            traits::{DbStruct, IntoDbStruct},
        },
    },
    impl_has_type_name, sqlx_operation_with_retries, verify_fk,
};
use super::{
    DefaultNhlContext, GameType, LocalizedNameJson, NhlGameKey, NhlPlayJson, NhlPrimaryKey,
    NhlRosterSpotJson, NhlSeason, NhlSeasonKey, NhlTeam, NhlTeamKey, PeriodDescriptorJson,
    PeriodTypeJson,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockJson {
    pub time_remaining: Option<String>,
    pub seconds_remaining: i32,
    pub running: bool,
    pub in_intermission: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvBroadcastsJson {
    pub id: i32,
    pub market: String,
    pub country_code: String,
    pub network: String,
    pub sequence_number: i32,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOutcomeJson {
    pub last_period_type: PeriodTypeJson,
}

#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
    #[serde(default)]
    pub clock: Option<ClockJson>,
    pub display_period: i32,
    pub max_periods: i32,
    pub game_outcome: GameOutcomeJson,
    pub plays: Vec<NhlPlayJson>,
    pub roster_spots: Vec<NhlRosterSpotJson>,
    pub reg_periods: i32,
}
impl IntoDbStruct for NhlGameJson {
    type DbStruct = NhlGame;
    type Context = DefaultNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
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
            game_state: _,
            game_schedule_state: _,
            period_descriptor,
            away_team,
            home_team,
            tv_broadcasts: _,
            shootout_in_use,
            ot_in_use,
            clock: _,
            display_period,
            max_periods,
            game_outcome,
            plays: _,
            roster_spots: _,
            reg_periods,
        } = self;
        let DefaultNhlContext { endpoint, raw_json } = context;
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
            endpoint,
            raw_json,
            last_updated: None,
        }
    }
}
#[derive(Clone, Debug, FromRow)]
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
    type IntoDbStruct = NhlGameJson;

    fn create_context_struct(&self) -> <<Self as DbStruct>::IntoDbStruct as IntoDbStruct>::Context {
        DefaultNhlContext {
            endpoint: self.endpoint.clone(),
            raw_json: self.raw_json.clone(),
        }
    }
}
#[async_trait]
impl Persistable for NhlGame {
    type Pk = NhlPrimaryKey;

    fn id(&self) -> Self::Pk {
        Self::Pk::Game(NhlGameKey { id: self.id })
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        verify_fk!(missing, db_context, Self::Pk::season(self.season));
        verify_fk!(missing, db_context, Self::Pk::team(self.away_team_id));
        verify_fk!(missing, db_context, Self::Pk::team(self.home_team_id));
        verify_fk!(missing, db_context, Self::Pk::api_cache(&self.endpoint));

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }

    fn create_upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_game (
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
            ),
            self.id,
            self.season,
            self.game_type,
            self.limited_scoring,
            self.game_date,
            self.venue,
            self.venue_location,
            self.start_time_utc,
            self.eastern_utc_offset,
            self.venue_utc_offset,
            self.period_descriptor_number,
            self.period_descriptor_type,
            self.period_descriptor_max_regulation_periods,
            self.away_team_id,
            self.away_team_name,
            self.away_team_abbrev,
            self.away_team_score,
            self.away_team_sog,
            self.away_team_logo,
            self.away_team_dark_logo,
            self.away_team_place_name,
            self.away_team_place_name_with_preposition,
            self.home_team_id,
            self.home_team_name,
            self.home_team_abbrev,
            self.home_team_score,
            self.home_team_sog,
            self.home_team_logo,
            self.home_team_dark_logo,
            self.home_team_place_name,
            self.home_team_place_name_with_preposition,
            self.shootout_in_use,
            self.ot_in_use,
            self.display_period,
            self.max_periods,
            self.game_outcome_last_period_type,
            self.reg_periods,
            self.endpoint,
            self.raw_json,
        )
    }
}

impl_has_type_name!(NhlGameJson);
impl_has_type_name!(NhlGame);
