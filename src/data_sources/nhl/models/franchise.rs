use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::{
    bind,
    common::{
        db::{DbContext, DbEntity, PrimaryKey, RelationshipIntegrity, StaticPgQuery},
        errors::LPError,
        models::{
            ApiCache, ApiCacheKey,
            traits::{DbStruct, IntoDbStruct},
        },
    },
    impl_has_type_name, verify_fk,
};

use super::{DefaultNhlContext, NhlFranchiseKey, NhlPrimaryKey};

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
    type Context = DefaultNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlFranchiseJson {
            id,
            full_name,
            team_common_name,
            team_place_name,
        } = self;
        let DefaultNhlContext { endpoint, raw_json } = context;
        NhlFranchise {
            id,
            full_name,
            team_common_name,
            team_place_name,
            endpoint,
            raw_json,
            last_updated: None,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct NhlFranchise {
    pub id: i32,
    pub full_name: String,
    pub team_common_name: String,
    pub team_place_name: String,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlFranchise {
    type IntoDbStruct = NhlFranchiseJson;

    fn create_context_struct(&self) -> <<Self as DbStruct>::IntoDbStruct as IntoDbStruct>::Context {
        DefaultNhlContext {
            endpoint: self.endpoint.clone(),
            raw_json: self.raw_json.clone(),
        }
    }
}
#[async_trait]
impl DbEntity for NhlFranchise {
    type Pk = NhlPrimaryKey;

    fn id(&self) -> Self::Pk {
        Self::Pk::Franchise(NhlFranchiseKey { id: self.id })
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        verify_fk!(missing, db_context, Self::Pk::api_cache(&self.endpoint));

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }

    fn create_upsert_query(&self) -> StaticPgQuery {
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
