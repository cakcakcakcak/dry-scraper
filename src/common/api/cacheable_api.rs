use std::fmt::Debug;

use async_trait::async_trait;
use sqlx::Row;
use tracing::instrument;

use crate::{
    common::{
        db::{DbContext, DbEntity},
        errors::LPError,
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
    ) -> Result<String, LPError> {
        tracing::debug!("Querying api_cache for the endpoint we seek...");
        match sqlx_operation_with_retries!(
            sqlx::query("SELECT raw_data from api_cache WHERE endpoint = $1")
                .bind(&endpoint)
                .fetch_optional(&db_context.pool)
                .await
        )
        .await?
        {
            Some(row) => match row.try_get::<String, _>("raw_data") {
                Ok(raw_data) => {
                    tracing::debug!("Record found for endpoint. Retrieving raw_data...");
                    return Ok(raw_data);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Cached record for endpoint is unusable. Attempting to refresh from API..."
                    );
                }
            },
            None => {
                tracing::debug!("Cached record not found for endpoint. Querying API...");
            }
        }
        let response: reqwest::Response =
            reqwest_with_retries!(self.client().get(endpoint).send().await)
                .await
                .map_err(|e| {
                    tracing::warn!(
                        error = %e,
                        "Error encountered while fetching from API."
                    );
                    LPError::Api(e)
                })?
                .error_for_status()
                .map_err(|e| {
                    tracing::warn!(
                        error = %e,
                        "HTTP response code not 2xx."
                    );
                    LPError::Api(e)
                })?;

        tracing::debug!("Response received. Parsing and inserting into cache...");
        let raw_data = response.text().await.map_err(|e| {
            tracing::warn!(
                error = %e,
                "Failed to parse response into text."
            );
            LPError::Api(e)
        })?;
        let cache_record = ApiCache {
            endpoint: endpoint.to_string(),
            raw_data: raw_data,
            last_updated: None,
        };

        tracing::debug!("Upserting cache record with endpoint {endpoint} into lp database.");
        cache_record.upsert(&db_context).await?;
        Ok(cache_record.raw_data)
    }
}
