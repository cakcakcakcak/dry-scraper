use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::LPError;
use crate::db::{DbContext, Persistable, PrimaryKey, RelationshipIntegrity, StaticPgQuery};
use crate::models::nhl::{DefaultNhlContext, NhlFranchise, NhlFranchiseKey};
use crate::models::traits::{DbStruct, IntoDbStruct};
use crate::models::{ApiCache, ApiCacheKey};

use crate::bind;
use crate::impl_has_type_name;
use crate::verify_fk;

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
    type Context = DefaultNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlTeamJson {
            id,
            franchise_id,
            full_name,
            league_id,
            raw_tricode,
            tricode,
        } = self;
        let DefaultNhlContext { endpoint, raw_json } = context;
        NhlTeam {
            id,
            franchise_id,
            full_name,
            league_id,
            raw_tricode,
            tricode,
            endpoint,
            raw_json,
            last_updated: None,
        }
    }
}
#[derive(Debug, FromRow, Clone)]
pub struct NhlTeam {
    pub id: i32,
    pub franchise_id: Option<i32>,
    pub full_name: String,
    pub league_id: i32,
    pub raw_tricode: String,
    pub tricode: String,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlTeam {
    type IntoDbStruct = NhlTeamJson;
}
#[async_trait]
impl Persistable for NhlTeam {
    type Pk = NhlTeamKey;

    fn id(&self) -> Self::Pk {
        Self::Pk { id: self.id }
    }

    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity, LPError> {
        let mut missing: Vec<Box<dyn PrimaryKey>> = vec![];

        if let Some(franchise_id) = self.franchise_id {
            verify_fk!(
                missing,
                db_context,
                NhlFranchise,
                NhlFranchiseKey { id: franchise_id }
            );
        }
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

#[derive(Debug)]
pub struct NhlTeamKey {
    pub id: i32,
}
impl PrimaryKey for NhlTeamKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_team WHERE id=$1").bind(self.id)
    }
}

impl_has_type_name!(NhlTeamJson);
impl_has_type_name!(NhlTeam);
