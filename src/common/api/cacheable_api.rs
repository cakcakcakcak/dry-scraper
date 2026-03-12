use std::fmt::Debug;

use async_trait::async_trait;
use sqlx::Row;
use tracing::instrument;

use crate::{
    common::{
        db::{DbContext, DbEntity},
        errors::DSError,
        models::ApiCache,
    },
    reqwest_with_retries, sqlx_operation_with_retries,
};

#[async_trait]
pub trait CacheableApi: Debug {
    fn client(&self) -> &reqwest::Client;

    #[instrument(skip(db_context))]
    async fn fetch_endpoint_cached(
        &self,
        db_context: &DbContext,
        endpoint: &str,
    ) -> Result<String, DSError> {
        tracing::debug!(endpoint, "Checking API cache");
        match sqlx_operation_with_retries!(
            sqlx::query("SELECT raw_data from api_cache WHERE endpoint = $1")
                .bind(endpoint)
                .fetch_optional(&db_context.pool)
                .await
        )
        .await?
        {
            Some(row) => match row.try_get::<String, _>("raw_data") {
                Ok(raw_data) => {
                    tracing::debug!(endpoint, "Cache hit");
                    return Ok(raw_data);
                }
                Err(e) => {
                    tracing::warn!(
                        endpoint,
                        error = %e,
                        "Cached record unusable, refreshing from API"
                    );
                }
            },
            None => {
                tracing::debug!(endpoint, "Cache miss, fetching from API");
            }
        }
        // Not in cache or unusable, fetch from API
        let response = reqwest_with_retries!(self.client().get(endpoint).send().await)
            .await
            .map_err(|e| {
                tracing::error!(endpoint, error = %e, "Failed to fetch from API");
                DSError::Api(e)
            })?
            .error_for_status()
            .map_err(|e| {
                tracing::error!(endpoint, error = %e, status = ?e.status(), "Non-2xx response from API");
                DSError::Api(e)
            })?;

        tracing::debug!(endpoint, "Parsing response and caching");
        let raw_data = response.text().await.map_err(|e| {
            tracing::error!(endpoint, error = %e, "Failed to parse response body");
            DSError::Api(e)
        })?;
        let cache_record = ApiCache {
            endpoint: endpoint.to_string(),
            raw_data,
            last_updated: None,
        };

        cache_record.upsert(db_context).await?;
        Ok(cache_record.raw_data)
    }
}

#[derive(Clone, Debug)]
pub struct SimpleApi {
    pub client: reqwest::Client,
}
#[async_trait]
impl CacheableApi for SimpleApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
