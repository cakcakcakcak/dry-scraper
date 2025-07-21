use std::str::FromStr;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::db::DbPool;
use crate::db::persistable::Persistable;
use crate::lp_error::LPError;
use crate::models::traits::{DbStruct, IntoDbStruct};
use crate::serde_helpers::JsonExt;
use crate::serde_helpers::{deserialize_default_to_string, deserialize_to_bool};

use crate::impl_has_type_name;
use crate::make_deserialize_key_to_type;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftDetails {
    pub year: Option<i32>,
    pub team_abbrev: Option<String>,
    pub round: Option<i32>,
    pub pick_in_round: Option<i32>,
    pub overall_pick: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayerJson {
    #[serde(rename = "playerId")]
    pub id: i32,
    #[serde(deserialize_with = "deserialize_default_to_string")]
    pub first_name: String,
    #[serde(deserialize_with = "deserialize_default_to_string")]
    pub last_name: String,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub is_active: bool,
    pub current_team_id: Option<i32>,
    pub current_team_abbrev: Option<String>,
    #[serde(deserialize_with = "deserialize_default_to_option_string")]
    pub full_team_name: Option<String>,
    #[serde(deserialize_with = "deserialize_default_to_option_string")]
    pub team_common_name: Option<String>,
    #[serde(deserialize_with = "deserialize_default_to_option_string")]
    pub team_place_name_with_preposition: Option<String>,
    pub team_logo: String,
    pub sweater_number: i32,
    pub position: String,
    pub headshot: String,
    pub hero_image: String,
    pub height_in_inches: i32,
    pub height_in_centimeters: i32,
    pub weight_in_pounds: i32,
    pub weight_in_kilograms: i32,
    pub birth_date: chrono::NaiveDate,
    #[serde(deserialize_with = "deserialize_default_to_string")]
    pub birth_city: String,
    #[serde(deserialize_with = "deserialize_default_to_string")]
    pub birth_state_province: String,
    pub birth_country: String,
    pub shoots_catches: String,
    pub draft_details: Option<DraftDetails>,
    pub player_slug: String,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub in_top100_all_time: bool,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub in_hhof: bool,
}
impl IntoDbStruct for NhlPlayerJson {
    type U = NhlPlayer;

    fn to_db_struct(self) -> Self::U {
        let NhlPlayerJson {
            id,
            first_name,
            last_name,
            is_active,
            current_team_id,
            current_team_abbrev,
            full_team_name,
            team_common_name,
            team_place_name_with_preposition,
            team_logo,
            sweater_number,
            position,
            headshot,
            hero_image,
            height_in_inches,
            height_in_centimeters,
            weight_in_pounds,
            weight_in_kilograms,
            birth_date,
            birth_city,
            birth_state_province,
            birth_country,
            shoots_catches,
            draft_details,
            player_slug,
            in_top100_all_time,
            in_hhof,
        } = self;
        let (
            draft_year,
            draft_team_abbreviation,
            draft_round,
            draft_pick_in_round,
            draft_overall_pick,
        ) = match draft_details {
            Some(d) => (
                d.year,
                d.team_abbrev,
                d.round,
                d.pick_in_round,
                d.overall_pick,
            ),
            None => (None, None, None, None, None),
        };
        NhlPlayer {
            id,
            first_name,
            last_name,
            is_active,
            current_team_id,
            current_team_abbrev,
            full_team_name,
            team_common_name,
            team_place_name_with_preposition,
            team_logo,
            sweater_number,
            position,
            headshot,
            hero_image,
            height_in_inches,
            height_in_centimeters,
            weight_in_pounds,
            weight_in_kilograms,
            birth_date,
            birth_city,
            birth_state_province,
            birth_country,
            shoots_catches,
            draft_year,
            draft_team_abbreviation,
            draft_round,
            draft_pick_in_round,
            draft_overall_pick,
            player_slug,
            in_top100_all_time,
            in_hhof,
            endpoint: String::new(),
            raw_json: serde_json::Value::Null,
            last_updated: None,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct NhlPlayer {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
    pub current_team_id: Option<i32>,
    pub current_team_abbrev: Option<String>,
    pub full_team_name: Option<String>,
    pub team_common_name: Option<String>,
    pub team_place_name_with_preposition: Option<String>,
    pub team_logo: String,
    pub sweater_number: i32,
    pub position: String,
    pub headshot: String,
    pub hero_image: String,
    pub height_in_inches: i32,
    pub height_in_centimeters: i32,
    pub weight_in_pounds: i32,
    pub weight_in_kilograms: i32,
    pub birth_date: chrono::NaiveDate,
    pub birth_city: String,
    pub birth_state_province: String,
    pub birth_country: String,
    pub shoots_catches: String,
    pub draft_year: Option<i32>,
    pub draft_team_abbreviation: Option<String>,
    pub draft_round: Option<i32>,
    pub draft_pick_in_round: Option<i32>,
    pub draft_overall_pick: Option<i32>,
    pub player_slug: String,
    pub in_top100_all_time: bool,
    pub in_hhof: bool,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlPlayer {
    fn fill_context(&mut self, endpoint: String, raw_data: String) -> Result<(), LPError> {
        self.endpoint = endpoint;

        let raw_json = serde_json::Value::from_str(&raw_data)?;
        self.raw_json = raw_json;
        Ok(())
    }
}
#[async_trait]
impl Persistable for NhlPlayer {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM nhl_player WHERE id=$1"#)
                .bind(id)
                .fetch_optional(pool)
                .await
        )
        .await
        .map_err(LPError::from)
    }

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(r#"INSERT INTO nhl_player (
                                        id,
                                        first_name,
                                        last_name,
                                        is_active,
                                        current_team_id,
                                        current_team_abbrev,
                                        full_team_name,
                                        team_common_name,
                                        team_place_name_with_preposition,
                                        team_logo,
                                        sweater_number,
                                        position,
                                        headshot,
                                        hero_image,
                                        height_in_inches,
                                        height_in_centimeters,
                                        weight_in_pounds,
                                        weight_in_kilograms,
                                        birth_date,
                                        birth_city,
                                        birth_state_province,
                                        birth_country,
                                        shoots_catches,
                                        draft_year,
                                        draft_team_abbreviation,
                                        draft_round,
                                        draft_pick_in_round,
                                        draft_overall_pick,
                                        player_slug,
                                        in_top100_all_time,
                                        in_hhof,
                                        endpoint,
                                        raw_json
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                        $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                                        $31,$32,$33)
                                    ON CONFLICT (id) DO UPDATE SET
                                        first_name = EXCLUDED.first_name,
                                        last_name = EXCLUDED.last_name,
                                        is_active = EXCLUDED.is_active,
                                        current_team_id = EXCLUDED.current_team_id,
                                        current_team_abbrev = EXCLUDED.current_team_abbrev,
                                        full_team_name = EXCLUDED.full_team_name,
                                        team_common_name = EXCLUDED.team_common_name,
                                        team_place_name_with_preposition = EXCLUDED.team_place_name_with_preposition,
                                        team_logo = EXCLUDED.team_logo,
                                        sweater_number = EXCLUDED.sweater_number,
                                        position = EXCLUDED.position,
                                        headshot = EXCLUDED.headshot,
                                        hero_image = EXCLUDED.hero_image,
                                        height_in_inches = EXCLUDED.height_in_inches,
                                        height_in_centimeters = EXCLUDED.height_in_centimeters,
                                        weight_in_pounds = EXCLUDED.weight_in_pounds,
                                        weight_in_kilograms = EXCLUDED.weight_in_kilograms,
                                        birth_date = EXCLUDED.birth_date,
                                        birth_city = EXCLUDED.birth_city,
                                        birth_state_province = EXCLUDED.birth_state_province,
                                        birth_country = EXCLUDED.birth_country,
                                        shoots_catches = EXCLUDED.shoots_catches,
                                        draft_year = EXCLUDED.draft_year,
                                        draft_team_abbreviation = EXCLUDED.draft_team_abbreviation,
                                        draft_round = EXCLUDED.draft_round,
                                        draft_pick_in_round = EXCLUDED.draft_pick_in_round,
                                        draft_overall_pick = EXCLUDED.draft_overall_pick,
                                        player_slug = EXCLUDED.player_slug,
                                        in_top100_all_time = EXCLUDED.in_top100_all_time,
                                        in_hhof = EXCLUDED.in_hhof,
                                        endpoint = EXCLUDED.endpoint,
                                        raw_json = EXCLUDED.raw_json,
                                        last_updated = now()
                                    "#
            )
            .bind(&self.id)
            .bind(&self.first_name)
            .bind(&self.last_name)
            .bind(&self.is_active)
            .bind(&self.current_team_id)
            .bind(&self.current_team_abbrev)
            .bind(&self.full_team_name)
            .bind(&self.team_common_name)
            .bind(&self.team_place_name_with_preposition)
            .bind(&self.team_logo)
            .bind(&self.sweater_number)
            .bind(&self.position)
            .bind(&self.headshot)
            .bind(&self.hero_image)
            .bind(&self.height_in_inches)
            .bind(&self.height_in_centimeters)
            .bind(&self.weight_in_pounds)
            .bind(&self.weight_in_kilograms)
            .bind(&self.birth_date)
            .bind(&self.birth_city)
            .bind(&self.birth_state_province)
            .bind(&self.birth_country)
            .bind(&self.shoots_catches)
            .bind(&self.draft_year)
            .bind(&self.draft_team_abbreviation)
            .bind(&self.draft_round)
            .bind(&self.draft_pick_in_round)
            .bind(&self.draft_overall_pick)
            .bind(&self.player_slug)
            .bind(&self.in_top100_all_time)
            .bind(&self.in_hhof)
            .bind(&self.endpoint)
            .bind(&self.raw_json)
    }
}

make_deserialize_key_to_type!(
    deserialize_default_to_option_string,
    "default",
    Option<String>
);

impl_has_type_name!(NhlPlayerJson);
impl_has_type_name!(NhlPlayer);
