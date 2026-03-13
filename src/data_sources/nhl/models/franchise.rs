use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::{
    bind,
    common::{
        db::{CacheKey, DbEntity, StaticPgQuery, StaticPgQueryAs},
        models::traits::IntoDbStruct,
    },
    data_sources::NhlFranchiseKey,
    impl_has_type_name, impl_pk_debug,
};

use super::NhlDefaultContext;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlFranchiseJson {
    pub id: i32,
    pub full_name: String,
    pub team_common_name: String,
    pub team_place_name: String,
}
impl IntoDbStruct for NhlFranchiseJson {
    type DbStruct = NhlFranchise;
    type Context = NhlDefaultContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlFranchiseJson {
            id,
            full_name,
            team_common_name,
            team_place_name,
        } = self;
        let NhlDefaultContext { endpoint, raw_json } = context;
        NhlFranchise {
            id,
            full_name,
            team_common_name,
            team_place_name,
            endpoint,
            raw_json,
        }
    }
}

#[derive(Clone, FromRow)]
pub struct NhlFranchise {
    pub id: i32,
    pub full_name: String,
    pub team_common_name: String,
    pub team_place_name: String,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
}
#[async_trait]
impl DbEntity for NhlFranchise {
    type Pk = NhlFranchiseKey;

    fn pk(&self) -> Self::Pk {
        NhlFranchiseKey { id: self.id }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT id from nhl_franchise")
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        vec![CacheKey {
            source: "api_cache",
            table: "api_cache",
            id: self.endpoint.clone(),
        }]
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_franchise (
                                        id,
                                        full_name,
                                        team_common_name,
                                        team_place_name,
                                        raw_json,
                                        endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6)
                                    ON CONFLICT (id) DO UPDATE SET
                                        full_name = EXCLUDED.full_name,
                                        team_common_name = EXCLUDED.team_common_name,
                                        team_place_name = EXCLUDED.team_place_name,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#,
            ),
            self.id,
            self.full_name,
            self.team_common_name,
            self.team_place_name,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlFranchiseJson);
impl_has_type_name!(NhlFranchise);
impl_pk_debug!(NhlFranchise);
