use async_trait::async_trait;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::LPError;
use crate::db::{DbContext, Persistable, PrimaryKey, StaticPgQuery};
use crate::models::nhl::GameNhlContext;
use crate::models::nhl::LocalizedNameJson;
use crate::models::traits::{DbStruct, IntoDbStruct};

use crate::bind;
use crate::impl_has_type_name;
use crate::sqlx_operation_with_retries;

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

#[derive(Debug, FromRow, Clone)]
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
impl Persistable for NhlRosterSpot {
    type Id = PrimaryKey;

    fn id(&self) -> Self::Id {
        PrimaryKey::NhlRosterSpot {
            game_id: self.game_id,
            player_id: self.player_id,
        }
    }

    #[tracing::instrument(skip(db_context))]
    async fn try_db(db_context: &DbContext, id: Self::Id) -> Result<Option<Self>, LPError> {
        match id {
            PrimaryKey::NhlRosterSpot { game_id, player_id } => sqlx_operation_with_retries!(
                sqlx::query_as::<_, Self>(
                    r#"SELECT * FROM nhl_roster_spot WHERE game_id=$1 AND player_id=$2"#
                )
                .bind(game_id.clone())
                .bind(player_id.clone())
                .fetch_optional(&db_context.pool)
                .await
            )
            .await
            .map_err(LPError::from),
            _ => Err(LPError::DatabaseCustom("Wrong ID variant".to_string())),
        }
    }

    fn create_upsert_query(&self) -> StaticPgQuery {
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
