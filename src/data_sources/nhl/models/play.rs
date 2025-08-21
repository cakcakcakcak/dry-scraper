use async_trait::async_trait;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;
use sqlx::postgres::types::PgInterval;

use super::super::{NhlPlayKey, NhlPrimaryKey};
use super::{DefendingSide, GameNhlContext, PeriodDescriptorJson, PeriodTypeJson};
use crate::impl_pk_debug;
use crate::{
    bind,
    common::{
        db::{DbContext, DbEntity, RelationshipIntegrity, StaticPgQuery, StaticPgQueryAs},
        errors::LPError,
        models::traits::{DbStruct, IntoDbStruct},
    },
    impl_has_type_name, make_deserialize_to_type, verify_fk,
};

use crate::common::serde_helpers::{JsonExt, parse_mmss_to_pginterval};

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

#[derive(Clone, FromRow)]
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
impl DbStruct for NhlPlay {
    type IntoDbStruct = NhlPlayJson;
}
#[async_trait]
impl DbEntity for NhlPlay {
    type Pk = NhlPrimaryKey;

    fn pk(&self) -> Self::Pk {
        Self::Pk::Play(NhlPlayKey {
            game_id: self.game_id,
            sort_order: self.sort_order,
        })
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>(
            "SELECT 'nhl_play' AS table_name, game_id, sort_order from nhl_play",
        )
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        verify_fk!(missing, db_context, Self::Pk::api_cache(&self.endpoint));
        verify_fk!(missing, db_context, Self::Pk::game(self.game_id));

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_play (
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
                                            raw_json,
                                            endpoint
                                        ) VALUES (
                                            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                            $11,$12,$13,$14,$15)
                                        ON CONFLICT (game_id, sort_order) DO UPDATE SET
                                            event_id = EXCLUDED.event_id,
                                            period_descriptor_number = EXCLUDED.period_descriptor_number,
                                            period_descriptor_type = EXCLUDED.period_descriptor_type,
                                            period_descriptor_max_regulation_periods = EXCLUDED.period_descriptor_max_regulation_periods,
                                            time_in_period = EXCLUDED.time_in_period,
                                            time_remaining = EXCLUDED.time_remaining,
                                            situation_code = EXCLUDED.situation_code,
                                            home_team_defending_side = EXCLUDED.home_team_defending_side,
                                            type_code = EXCLUDED.type_code,
                                            type_desc_key = EXCLUDED.type_desc_key,
                                            sort_order = EXCLUDED.sort_order,
                                            details = EXCLUDED.details,
                                            raw_json = EXCLUDED.raw_json,
                                            endpoint = EXCLUDED.endpoint,
                                            last_updated = now()
                                        "#,
            ),
            self.game_id,
            self.event_id,
            self.period_descriptor_number,
            self.period_descriptor_type,
            self.period_descriptor_max_regulation_periods,
            self.time_in_period,
            self.time_remaining,
            self.situation_code,
            self.home_team_defending_side,
            self.type_code,
            self.type_desc_key,
            self.sort_order,
            self.details,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlPlayJson);
impl_has_type_name!(NhlPlay);
impl_pk_debug!(NhlPlay);

make_deserialize_to_type!(deserialize_to_option_i32, Option<i32>);
