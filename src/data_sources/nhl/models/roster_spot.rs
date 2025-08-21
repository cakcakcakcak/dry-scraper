use async_trait::async_trait;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use super::super::primary_key::*;
use super::{GameNhlContext, LocalizedNameJson};
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
        let GameNhlContext {
            endpoint,
            game_id,
            raw_json,
        } = context;

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

#[derive(FromRow, Clone)]
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
impl DbStruct for NhlRosterSpot {
    type IntoDbStruct = NhlRosterSpotJson;
}

#[async_trait]
impl DbEntity for NhlRosterSpot {
    type Pk = NhlPrimaryKey;

    fn pk(&self) -> Self::Pk {
        Self::Pk::RosterSpot(NhlRosterSpotKey {
            game_id: self.game_id,
            player_id: self.player_id,
        })
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>(
            "SELECT 'nhl_roster_spot' AS table_name, game_id, player_id from nhl_roster_spot",
        )
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        verify_fk!(missing, db_context, Self::Pk::game(self.game_id));
        verify_fk!(missing, db_context, Self::Pk::player(self.player_id));
        verify_fk!(missing, db_context, Self::Pk::team(self.team_id));
        verify_fk!(missing, db_context, Self::Pk::api_cache(&self.endpoint));

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_roster_spot (
                                        game_id,
                                        player_id,
                                        team_id,
                                        first_name,
                                        last_name,
                                        sweater_number,
                                        position_code,
                                        headshot,
                                        raw_json,
                                        endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                                    ON CONFLICT (game_id, player_id) DO UPDATE SET 
                                        game_id = EXCLUDED.game_id,
                                        player_id = EXCLUDED.player_id,
                                        team_id = EXCLUDED.team_id,
                                        first_name = EXCLUDED.first_name,
                                        last_name = EXCLUDED.last_name,
                                        sweater_number = EXCLUDED.sweater_number,
                                        position_code = EXCLUDED.position_code,
                                        headshot = EXCLUDED.headshot,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#,
            ),
            self.game_id,
            self.player_id,
            self.team_id,
            self.first_name,
            self.last_name,
            self.sweater_number,
            self.position_code,
            self.headshot,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlRosterSpotJson);
impl_has_type_name!(NhlRosterSpot);
impl_pk_debug!(NhlRosterSpot);
