use async_trait::async_trait;
use sqlx::Row;
use tracing::instrument;

use crate::db::DbPool;
use crate::lp_error;
use crate::models::api_cache::ApiCache;

use crate::reqwest_with_retries;
use crate::sqlx_operation_with_retries;

#[async_trait]
pub trait CacheableApi: std::fmt::Debug {
    fn client(&self) -> &reqwest::Client;

    #[instrument(skip(pool))]
    async fn get_or_cache_endpoint(
        &self,
        pool: &DbPool,
        endpoint: &str,
    ) -> Result<String, lp_error::LPError> {
        // query our api_cache for the endpoint we seek
        tracing::debug!("Querying api_cache for the endpoint we seek...");
        match sqlx_operation_with_retries!(
            sqlx::query("SELECT raw_data from api_cache WHERE endpoint = $1")
                .bind(&endpoint)
                .fetch_optional(pool)
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
                    lp_error::LPError::Api(e)
                })?
                .error_for_status()
                .map_err(|e| {
                    tracing::warn!(
                        error = %e,
                        "HTTP response code not 2xx."
                    );
                    lp_error::LPError::Api(e)
                })?;

        tracing::debug!("Response received. Parsing and inserting into cache...");
        let raw_data = response.text().await.map_err(|e| {
            tracing::warn!(
                error = %e,
                "Failed to parse response into text."
            );
            lp_error::LPError::Api(e)
        })?;
        let cache_record = ApiCache {
            endpoint: endpoint.to_string(),
            raw_data: raw_data,
            last_updated: None,
        };

        tracing::debug!("Upserting cache record with endpoint {endpoint} into lp database.");
        cache_record.upsert(&pool).await?;
        Ok(cache_record.raw_data)
    }
}
