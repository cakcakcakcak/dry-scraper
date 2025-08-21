use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use super::super::primary_key::*;
use super::{
    DefaultNhlContext, DefendingSide, GameNhlContext, GameType, LocalizedNameJson,
    NhlRosterSpotJson, NhlSeason, NhlTeam, PeriodDescriptorJson, PeriodTypeJson,
};
use crate::impl_pk_debug;
use crate::{
    bind,
    common::{
        db::{
            DbContext, DbEntity, PrimaryKey, RelationshipIntegrity, StaticPgQuery, StaticPgQueryAs,
        },
        errors::LPError,
        models::{
            ApiCache, ApiCacheKey,
            traits::{DbStruct, IntoDbStruct},
        },
        serde_helpers::JsonExt,
    },
    impl_has_type_name, make_deserialize_key_to_type, make_deserialize_to_type,
    sqlx_operation_with_retries, verify_fk,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftDetailsJson {
    pub year: Option<i32>,
    pub team_abbrev: Option<String>,
    pub round: Option<i32>,
    pub pick_in_round: Option<i32>,
    pub overall_pick: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayerJson {
    #[serde(rename = "playerId")]
    pub id: i32,
    pub first_name: LocalizedNameJson,
    pub last_name: LocalizedNameJson,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub is_active: bool,
    pub current_team_id: Option<i32>,
    pub current_team_abbrev: Option<String>,
    pub full_team_name: Option<LocalizedNameJson>,
    pub team_common_name: Option<LocalizedNameJson>,
    pub team_place_name_with_preposition: Option<LocalizedNameJson>,
    pub team_logo: Option<String>,
    pub sweater_number: Option<i32>,
    pub position: String,
    pub headshot: String,
    pub hero_image: String,
    pub height_in_inches: i32,
    pub height_in_centimeters: i32,
    pub weight_in_pounds: i32,
    pub weight_in_kilograms: i32,
    pub birth_date: chrono::NaiveDate,
    pub birth_city: LocalizedNameJson,
    pub birth_state_province: Option<LocalizedNameJson>,
    pub birth_country: String,
    pub shoots_catches: String,
    pub draft_details: Option<DraftDetailsJson>,
    pub player_slug: String,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub in_top100_all_time: bool,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub in_hhof: bool,
}
impl IntoDbStruct for NhlPlayerJson {
    type DbStruct = NhlPlayer;
    type Context = DefaultNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
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
        let DefaultNhlContext { endpoint, raw_json } = context;
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
        let full_team_name = full_team_name.map(|name| name.default);
        let team_common_name = team_common_name.map(|name| name.default);
        let team_place_name_with_preposition =
            team_place_name_with_preposition.map(|name| name.default);
        let birth_city = birth_city.default;
        let birth_state_province = birth_state_province.map(|name| name.default);
        NhlPlayer {
            id,
            first_name: first_name.default,
            last_name: last_name.default,
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
            raw_json,
            last_updated: None,
        }
    }
}

#[derive(Clone, FromRow)]
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
    pub team_logo: Option<String>,
    pub sweater_number: Option<i32>,
    pub position: String,
    pub headshot: String,
    pub hero_image: String,
    pub height_in_inches: i32,
    pub height_in_centimeters: i32,
    pub weight_in_pounds: i32,
    pub weight_in_kilograms: i32,
    pub birth_date: chrono::NaiveDate,
    pub birth_city: String,
    pub birth_state_province: Option<String>,
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
    type IntoDbStruct = NhlPlayerJson;
}
#[async_trait]
impl DbEntity for NhlPlayer {
    type Pk = NhlPrimaryKey;

    fn pk(&self) -> Self::Pk {
        Self::Pk::Player(NhlPlayerKey { id: self.id })
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT 'nhl_player' AS table_name, id from nhl_player")
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        if let Some(current_team_id) = self.current_team_id {
            verify_fk!(missing, db_context, Self::Pk::team(current_team_id))
        }
        verify_fk!(missing, db_context, Self::Pk::api_cache(&self.endpoint));

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_player (
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
            ),
            self.id,
            self.first_name,
            self.last_name,
            self.is_active,
            self.current_team_id,
            self.current_team_abbrev,
            self.full_team_name,
            self.team_common_name,
            self.team_place_name_with_preposition,
            self.team_logo,
            self.sweater_number,
            self.position,
            self.headshot,
            self.hero_image,
            self.height_in_inches,
            self.height_in_centimeters,
            self.weight_in_pounds,
            self.weight_in_kilograms,
            self.birth_date,
            self.birth_city,
            self.birth_state_province,
            self.birth_country,
            self.shoots_catches,
            self.draft_year,
            self.draft_team_abbreviation,
            self.draft_round,
            self.draft_pick_in_round,
            self.draft_overall_pick,
            self.player_slug,
            self.in_top100_all_time,
            self.in_hhof,
            self.endpoint,
            self.raw_json,
        )
    }
}

impl_has_type_name!(NhlPlayerJson);
impl_has_type_name!(NhlPlayer);
impl_pk_debug!(NhlPlayer);

make_deserialize_key_to_type!(deserialize_default_to_string, "default", String);
make_deserialize_to_type!(deserialize_to_bool, bool);
make_deserialize_key_to_type!(
    deserialize_default_to_option_string,
    "default",
    Option<String>
);
