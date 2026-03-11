use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use super::super::primary_key::*;
use super::NhlDefaultContext;
use crate::{
    bind,
    common::{
        db::{CacheKey, DbEntity, StaticPgQuery, StaticPgQueryAs},
        models::traits::{DbStruct, IntoDbStruct},
    },
    impl_has_type_name, impl_pk_debug,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlTeamJson {
    pub id: i32,
    pub franchise_id: Option<i32>,
    pub full_name: String,
    pub league_id: i32,
    pub raw_tricode: String,
    #[serde(rename = "triCode")]
    pub tricode: String,
}
impl IntoDbStruct for NhlTeamJson {
    type DbStruct = NhlTeam;
    type Context = NhlDefaultContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlTeamJson {
            id,
            franchise_id,
            full_name,
            league_id,
            raw_tricode,
            tricode,
        } = self;
        let NhlDefaultContext { endpoint, raw_json } = context;
        NhlTeam {
            id,
            franchise_id,
            full_name,
            league_id,
            raw_tricode,
            tricode,
            endpoint,
            raw_json,
        }
    }
}
#[derive(FromRow, Clone)]
pub struct NhlTeam {
    pub id: i32,
    pub franchise_id: Option<i32>,
    pub full_name: String,
    pub league_id: i32,
    pub raw_tricode: String,
    pub tricode: String,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
}
impl DbStruct for NhlTeam {
    type IntoDbStruct = NhlTeamJson;
}
#[async_trait]
impl DbEntity for NhlTeam {
    type Pk = NhlTeamKey;

    fn pk(&self) -> Self::Pk {
        NhlTeamKey { id: self.id }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT id from nhl_team")
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        let mut keys = vec![CacheKey {
            source: "nhl",
            table: "api_cache",
            id: self.endpoint.clone(),
        }];

        if let Some(franchise_id) = self.franchise_id {
            keys.push(CacheKey {
                source: "nhl",
                table: "team",
                id: franchise_id.to_string(),
            });
        }

        keys
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_team (
                                        id,
                                        franchise_id,
                                        full_name,
                                        league_id,
                                        raw_tricode,
                                        tricode,
                                        raw_json,
                                        endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8)
                                    ON CONFLICT (id) DO UPDATE SET
                                        franchise_id = EXCLUDED.franchise_id,
                                        full_name = EXCLUDED.full_name,
                                        league_id = EXCLUDED.league_id,
                                        raw_tricode = EXCLUDED.raw_tricode,
                                        tricode = EXCLUDED.tricode,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#,
            ),
            self.id,
            self.franchise_id,
            self.full_name,
            self.league_id,
            self.raw_tricode,
            self.tricode,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlTeamJson);
impl_has_type_name!(NhlTeam);
impl_pk_debug!(NhlTeam);
