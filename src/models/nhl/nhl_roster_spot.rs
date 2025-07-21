use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;
use sqlx::postgres::types::PgInterval;

use crate::models::nhl::nhl_model_common::LocalizedNameJson;
use crate::serde_helpers::deserialize_mmss_to_pginterval;

use crate::impl_has_type_name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlRosterSpotJson {
    player_id: i32,
    team_id: i32,
    first_name: LocalizedNameJson,
    last_name: LocalizedNameJson,
    sweater_number: i32,
    position_code: String,
    headshot: String,
}

impl_has_type_name!(NhlRosterSpotJson);
