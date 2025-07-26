use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;
use sqlx::postgres::types::PgInterval;

use crate::models::nhl::{DefendingSide, GameNhlContext, PeriodDescriptorJson, PeriodTypeJson};
use crate::models::traits::{DbStruct, IntoDbStruct};
use crate::serde_helpers::{JsonExt, parse_mmss_to_pginterval};

use crate::impl_has_type_name;
use crate::make_deserialize_to_type;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayJson {
    pub event_id: i32,
    pub period_descriptor: PeriodDescriptorJson,
    pub time_in_period: String,
    pub time_remaining: String,
    #[serde(default, deserialize_with = "deserialize_to_option_i32")]
    pub situation_code: Option<i32>,
    pub home_team_defending_side: Option<DefendingSide>,
    pub type_code: i32,
    pub type_desc_key: String,
    pub sort_order: i32,
    pub details: Option<serde_json::Value>,
}
impl IntoDbStruct for NhlPlayJson {
    type DbStruct = NhlPlay;
    type Context = GameNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlPlayJson {
            event_id,
            period_descriptor,
            time_in_period,
            time_remaining,
            situation_code,
            home_team_defending_side,
            type_code,
            type_desc_key,
            sort_order,
            details,
        } = self;
        let GameNhlContext {
            endpoint,
            game_id,
            raw_json,
        } = context;
        let PeriodDescriptorJson {
            number: period_descriptor_number,
            period_type: period_descriptor_type,
            max_regulation_periods: period_descriptor_max_regulation_periods,
        } = period_descriptor;
        let time_in_period = parse_mmss_to_pginterval(&time_in_period);
        let time_remaining = parse_mmss_to_pginterval(&time_remaining);
        NhlPlay {
            game_id,
            event_id,
            period_descriptor_number,
            period_descriptor_type,
            period_descriptor_max_regulation_periods,
            time_in_period,
            time_remaining,
            situation_code,
            home_team_defending_side,
            type_code,
            type_desc_key,
            sort_order,
            details,
            endpoint,
            raw_json,
            last_updated: None,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct NhlPlay {
    pub game_id: i32,
    pub event_id: i32,
    pub period_descriptor_number: i32,
    pub period_descriptor_type: PeriodTypeJson,
    pub period_descriptor_max_regulation_periods: i32,
    pub time_in_period: PgInterval,
    pub time_remaining: PgInterval,
    pub situation_code: Option<i32>,
    pub home_team_defending_side: Option<DefendingSide>,
    pub type_code: i32,
    pub type_desc_key: String,
    pub sort_order: i32,
    pub details: Option<serde_json::Value>,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlPlay {}

impl_has_type_name!(NhlPlayJson);
impl_has_type_name!(NhlPlay);

make_deserialize_to_type!(deserialize_to_option_i32, Option<i32>);
