use async_trait::async_trait;
use sqlx::Row;
use tracing::instrument;

use crate::lp_error;

use crate::reqwest_with_retries;
use crate::sqlx_operation_with_retries;

#[async_trait]
pub trait CacheableApi: std::fmt::Debug {
    fn client(&self) -> &reqwest::Client;

    #[instrument(skip(pool))]
    async fn get_or_cache_endpoint(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        endpoint: &str,
    ) -> Result<String, lp_error::LPError> {
        // query our api_cache for the endpoint we seek
        tracing::info!("Querying api_cache for the endpoint we seek...");
        match sqlx_operation_with_retries!(
            sqlx::query("SELECT raw_data from api_cache WHERE endpoint = $1")
                .bind(&endpoint)
                .fetch_optional(pool)
                .await
        )
        .await?
        {
            Some(row) => {
                // if present, get the contents of the raw_data column and return it
                match row.try_get::<String, _>("raw_data") {
                    Ok(raw_data) => {
                        tracing::info!("Record found for endpoint. Retrieving raw_data...");
                        return Ok(raw_data);
                    }
                    Err(e) => {
                        tracing::warn!(
                            endpoint = %&endpoint,
                            error = %e,
                            "Cached record for endpoint is unusable. Attempting to refresh from API..."
                        );
                        // fall through to fetch from API
                    }
                }
            }
            None => {
                // otherwise fetch the raw_data, store it in our cache, and return it
                tracing::info!("Cached record not found for endpoint. Querying API...");
                // fall through to fetch from api
            }
        }
        // fetch from api, insert into cache, and return
        let response: reqwest::Response =
            reqwest_with_retries!(self.client().get(endpoint).send().await)
                .await
                .map_err(|e| {
                    // network error encountered
                    lp_error::LPError::Api(e)
                })?
                .error_for_status()
                .map_err(|e| {
                    // http status code not 2xx
                    lp_error::LPError::Api(e)
                })?;

        tracing::info!("Response received. Parsing and inserting into cache...");
        let raw_data = response
            .text()
            .await
            .map_err(|e| lp_error::LPError::Api(e))?;
        sqlx_operation_with_retries!(
            sqlx::query(r#"INSERT INTO api_cache (endpoint, raw_data) VALUES ($1, $2)
                ON CONFLICT (endpoint) DO UPDATE SET raw_data = EXCLUDED.raw_data, last_updated = now()"#)
                .bind(&endpoint)
                .bind(&raw_data)
                .execute(pool)
                .await
        ).await.map_err(|e| lp_error::LPError::Database(e))?;
        Ok(raw_data)
    }
}
