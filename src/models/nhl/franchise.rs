use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::LPError;
use crate::db::{DbContext, Persistable, PrimaryKey, RelationshipIntegrity, StaticPgQuery};
use crate::models::nhl::DefaultNhlContext;
use crate::models::traits::{DbStruct, IntoDbStruct};
use crate::models::{ApiCache, ApiCacheKey};

use crate::bind;
use crate::impl_has_type_name;
use crate::verify_fk;

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
}
#[async_trait]
impl Persistable for NhlFranchise {
    type Pk = NhlFranchiseKey;

    fn id(&self) -> Self::Pk {
        Self::Pk { id: self.id }
    }

    #[tracing::instrument(skip(db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity, LPError> {
        let mut missing: Vec<Box<dyn PrimaryKey>> = vec![];

        verify_fk!(
            missing,
            db_context,
            ApiCache,
            ApiCacheKey {
                endpoint: self.endpoint.clone()
            }
        );

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

#[derive(Debug)]
pub struct NhlFranchiseKey {
    pub id: i32,
}
impl PrimaryKey for NhlFranchiseKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_franchise WHERE id=$1"#).bind(self.id)
    }
}

impl_has_type_name!(NhlFranchiseJson);
impl_has_type_name!(NhlFranchise);
