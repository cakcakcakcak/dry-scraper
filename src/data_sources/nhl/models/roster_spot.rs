use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use super::super::primary_key::*;
use super::{LocalizedNameJson, NhlGameContext};
use crate::impl_pk_debug;
use crate::{
    bind,
    common::{
        db::{DbContext, DbEntity, RelationshipIntegrity, StaticPgQuery, StaticPgQueryAs},
        errors::LPError,
        models::traits::{DbStruct, IntoDbStruct},
    },
    impl_has_type_name, verify_fk,
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
    type Context = NhlGameContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlRosterSpotJson {
            player_id,
            team_id,
            first_name,
            last_name,
            sweater_number,
            position_code,
            headshot,
        } = self;
        let NhlGameContext {
            endpoint,
            game_id,
            raw_json,
        } = context;

        NhlRosterSpot {
            game_id,
            player_id,
            team_id,
            first_name: first_name.best_str(),
            last_name: last_name.best_str(),
            sweater_number,
            position_code,
            headshot,
            endpoint,
            raw_json,
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

    fn foreign_keys(&self) -> Vec<Self::Pk> {
        vec![
            Self::Pk::api_cache(&self.endpoint),
            Self::Pk::game(self.game_id),
            Self::Pk::player(self.player_id),
            Self::Pk::team(self.team_id),
        ]
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
