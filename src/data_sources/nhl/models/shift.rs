use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::postgres::types::PgInterval;
use sqlx::FromRow;

use crate::data_sources::models::NhlDefaultContext;
use crate::data_sources::NhlShiftKey;
use crate::impl_pk_debug;
use crate::{
    bind,
    common::{
        db::{CacheKey, DbEntity, StaticPgQuery, StaticPgQueryAs},
        models::traits::{DbStruct, IntoDbStruct},
    },
    impl_has_type_name,
};

use crate::common::serde_helpers::parse_mmss_to_pginterval;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlShiftJson {
    pub id: i32,
    pub detail_code: Option<i32>,
    pub duration: Option<String>,
    pub end_time: String,
    pub event_description: Option<String>,
    pub event_details: Option<String>,
    pub event_number: Option<i32>,
    pub first_name: String,
    pub game_id: i32,
    pub hex_value: Option<String>,
    pub last_name: String,
    pub period: i32,
    pub player_id: i32,
    pub shift_number: i32,
    pub start_time: String,
    pub team_abbrev: String,
    pub team_id: i32,
    pub team_name: String,
    pub type_code: i32,
}
impl IntoDbStruct for NhlShiftJson {
    type DbStruct = NhlShift;
    type Context = NhlDefaultContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlShiftJson {
            id,
            detail_code,
            duration,
            end_time,
            event_description,
            event_details,
            event_number,
            first_name,
            game_id,
            hex_value,
            last_name,
            period,
            player_id,
            shift_number,
            start_time,
            team_abbrev,
            team_id,
            team_name,
            type_code,
        } = self;
        let NhlDefaultContext { endpoint, raw_json } = context;
        let duration: PgInterval =
            parse_mmss_to_pginterval(&duration.unwrap_or("0:00".to_string()));
        let end_time: PgInterval = parse_mmss_to_pginterval(&end_time);
        let start_time: PgInterval = parse_mmss_to_pginterval(&start_time);
        NhlShift {
            id,
            detail_code,
            duration,
            end_time,
            event_description,
            event_details,
            event_number,
            first_name,
            game_id,
            hex_value,
            last_name,
            period,
            player_id,
            shift_number,
            start_time,
            team_abbrev,
            team_id,
            team_name,
            type_code,
            endpoint,
            raw_json,
        }
    }
}

#[derive(Clone, FromRow)]
pub struct NhlShift {
    pub id: i32,
    pub detail_code: Option<i32>,
    pub duration: PgInterval,
    pub end_time: PgInterval,
    pub event_description: Option<String>,
    pub event_details: Option<String>,
    pub event_number: Option<i32>,
    pub first_name: String,
    pub game_id: i32,
    pub hex_value: Option<String>,
    pub last_name: String,
    pub period: i32,
    pub player_id: i32,
    pub shift_number: i32,
    pub start_time: PgInterval,
    pub team_abbrev: String,
    pub team_id: i32,
    pub team_name: String,
    pub type_code: i32,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
}
impl DbStruct for NhlShift {
    type IntoDbStruct = NhlShiftJson;
}
#[async_trait]
impl DbEntity for NhlShift {
    type Pk = NhlShiftKey;

    fn pk(&self) -> Self::Pk {
        NhlShiftKey {
            game_id: self.game_id,
            player_id: self.player_id,
            shift_number: self.shift_number,
        }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT game_id, player_id, shift_number from nhl_shift")
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        vec![
            CacheKey {
                source: "nhl",
                table: "api_cache",
                id: self.endpoint.clone(),
            },
            CacheKey {
                source: "nhl",
                table: "game",
                id: self.game_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "player",
                id: self.player_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "team",
                id: self.team_id.to_string(),
            },
        ]
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_shift (
                                id,
                                detail_code,
                                duration,
                                end_time,
                                event_description,
                                event_details,
                                event_number,
                                first_name,
                                game_id,
                                hex_value,
                                last_name,
                                period,
                                player_id,
                                shift_number,
                                start_time,
                                team_abbrev,
                                team_id,
                                team_name,
                                type_code,
                                endpoint,
                                raw_json
                            ) VALUES (
                                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                $21
                            ) ON CONFLICT (game_id, player_id, shift_number) DO UPDATE SET
                                id = EXCLUDED.id,
                                detail_code = EXCLUDED.detail_code,
                                duration = EXCLUDED.duration,
                                end_time = EXCLUDED.end_time,
                                event_description = EXCLUDED.event_description,
                                event_details = EXCLUDED.event_details,
                                event_number = EXCLUDED.event_number,
                                first_name = EXCLUDED.first_name,
                                hex_value = EXCLUDED.hex_value,
                                last_name = EXCLUDED.last_name,
                                period = EXCLUDED.period,
                                start_time = EXCLUDED.start_time,
                                team_abbrev = EXCLUDED.team_abbrev,
                                team_id = EXCLUDED.team_id,
                                team_name = EXCLUDED.team_name,
                                type_code = EXCLUDED.type_code,
                                endpoint = EXCLUDED.endpoint,
                                raw_json = EXCLUDED.raw_json,
                                last_updated = now()
                "#
            ),
            self.id,
            self.detail_code,
            self.duration,
            self.end_time,
            self.event_description,
            self.event_details,
            self.event_number,
            self.first_name,
            self.game_id,
            self.hex_value,
            self.last_name,
            self.period,
            self.player_id,
            self.shift_number,
            self.start_time,
            self.team_abbrev,
            self.team_id,
            self.team_name,
            self.type_code,
            self.endpoint,
            self.raw_json,
        )
    }
}

impl_has_type_name!(NhlShiftJson);
impl_has_type_name!(NhlShift);
impl_pk_debug!(NhlShift);
