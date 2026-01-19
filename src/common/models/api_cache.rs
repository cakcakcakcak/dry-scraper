use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    any_primary_key::AnyPrimaryKey,
    bind,
    common::{
        api::{CacheableApi, cacheable_api::SimpleApi},
        db::{DbContext, DbEntity, PrimaryKey, StaticPgQuery, StaticPgQueryAs},
        errors::DSError,
    },
    impl_has_type_name, impl_pk_debug, sqlx_operation_with_retries,
};

#[derive(Clone, Serialize, Deserialize, FromRow)]
pub struct ApiCache {
    pub endpoint: String,
    pub raw_data: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
#[async_trait]
impl DbEntity for ApiCache {
    type Pk = ApiCacheKey;

    fn pk(&self) -> Self::Pk {
        Self::Pk {
            endpoint: self.endpoint.clone(),
        }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT endpoint from api_cache")
    }

    #[tracing::instrument(skip(db_context))]
    async fn fetch_from_db_by_key(
        db_context: &DbContext,
        id: &Self::Pk,
    ) -> Result<Option<Self>, DSError> {
        match sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM api_cache WHERE endpoint=$1"#)
                .bind(&id.endpoint.clone())
                .fetch_optional(&db_context.pool)
                .await
        )
        .await
        {
            Ok(Some(record)) => {
                tracing::info!(
                    "Record found in lp table api_cache for endpoint {}",
                    id.endpoint
                );
                Ok(Some(record))
            }
            Ok(None) => {
                tracing::info!(
                    "Record NOT found in lp table api_cache for endpoint {}",
                    id.endpoint
                );
                Ok(None)
            }
            Err(e) => {
                tracing::warn!(
                    "Error encountered while querying api_cache for endpoint {}",
                    id.endpoint
                );
                Err(DSError::Database(e))
            }
        }
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO api_cache (endpoint, raw_data)
                                    VALUES ($1,$2)
                                    ON CONFLICT (endpoint) DO UPDATE SET 
                                        endpoint = EXCLUDED.endpoint,
                                        raw_data = EXCLUDED.raw_data,
                                        last_updated = now()
                                    "#,
            ),
            self.endpoint,
            self.raw_data,
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, FromRow)]
pub struct ApiCacheKey {
    pub endpoint: String,
}
#[async_trait]
impl PrimaryKey for ApiCacheKey {
    type Api = SimpleApi;

    fn any_pk(&self) -> AnyPrimaryKey {
        AnyPrimaryKey::ApiCache(self.clone())
    }

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM api_cache WHERE endpoint=$1").bind(self.endpoint.clone())
    }

    async fn upsert_from_api(
        &self,
        db_context: &DbContext,
        api: &SimpleApi,
    ) -> Result<(), DSError> {
        api.fetch_endpoint_cached(db_context, &self.endpoint)
            .await?;
        Ok(())
    }

    async fn verify_by_key(self, db_context: &DbContext) -> Result<Option<Self>, DSError> {
        ApiCache::verify_by_key(db_context, self).await
    }
}

impl_has_type_name!(ApiCache);
impl_pk_debug!(ApiCache);
