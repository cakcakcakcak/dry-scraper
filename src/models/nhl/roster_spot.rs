use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;
use sqlx::postgres::types::PgInterval;

use crate::models::nhl::GameNhlContext;
use crate::models::nhl::LocalizedNameJson;
use crate::models::traits::{DbStruct, IntoDbStruct};
use crate::serde_helpers::deserialize_mmss_to_pginterval;

use crate::impl_has_type_name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlRosterSpotJson {
    pub player_id: i32,
    pub team_id: i32,
    pub first_name: LocalizedNameJson,
    pub last_name: LocalizedNameJson,
    pub sweater_number: i32,
    pub position_code: String,
    pub headshot: String,
}
impl IntoDbStruct for NhlRosterSpotJson {
    type DbStruct = NhlRosterSpot;
    type Context = GameNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlRosterSpotJson {
            player_id,
            team_id,
            first_name,
            last_name,
            sweater_number,
            position_code,
            headshot,
        } = self;
        let GameNhlContext { endpoint, game_id, raw_json } = context;

        NhlRosterSpot {
            game_id,
            player_id,
            team_id,
            first_name: first_name.default,
            last_name: last_name.default,
            sweater_number,
            position_code,
            headshot,
            endpoint,
            raw_json,
            last_updated: None,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct NhlRosterSpot {
    pub game_id: i32,
    pub player_id: i32,
    pub team_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub sweater_number: i32,
    pub position_code: String,
    pub headshot: String,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlRosterSpot {}

impl_has_type_name!(NhlRosterSpotJson);
impl_has_type_name!(NhlRosterSpot);
