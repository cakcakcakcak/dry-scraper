use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use super::super::primary_key::*;
use super::DefaultNhlContext;
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
    impl_has_type_name, impl_pk_debug, make_deserialize_key_to_type, make_deserialize_to_type,
    sqlx_operation_with_retries, verify_fk,
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
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlTeam {
    type IntoDbStruct = NhlTeamJson;
}
#[async_trait]
impl DbEntity for NhlTeam {
    type Pk = NhlPrimaryKey;

    fn pk(&self) -> Self::Pk {
        Self::Pk::Team(NhlTeamKey { id: self.id })
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT 'nhl_team' AS table_name, id from nhl_team")
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        if let Some(franchise_id) = self.franchise_id {
            verify_fk!(missing, db_context, Self::Pk::franchise(franchise_id));
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
