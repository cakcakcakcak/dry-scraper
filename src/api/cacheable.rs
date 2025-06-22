//! A trait for APIs whose responses can be cached in a database.
//!
//! Implementors must provide access to a `reqwest::Client`.
//!
//! # Methods
//!
//! - `client(&self) -> &reqwest::Client`: Returns a reference to the HTTP client.
//! - `get_or_cache_endpoint(&self, pool: &sqlx::Pool<sqlx::Postgres>, endpoint: &str) -> Result<String, lp_error::LPError>`:
//!     Attempts to retrieve the cached response for the given endpoint from the database.
//!     If not present or unusable, fetches the data from the API, stores it in the cache, and returns it.
//!
//! # Caching Logic
//!
//! 1. Checks the `api_cache` table for a cached response for the given endpoint.
//! 2. If found and valid, returns the cached data.
//! 3. If not found or invalid, fetches the data from the API, updates the cache, and returns the new data.
//!
//! # Errors
//!
//! Returns an `LPError` if any database or network operation fails.

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
        tracing::info!(endpoint = %endpoint, "Querying api_cache for the endpoint we seek...");
        match sqlx_operation_with_retries!(
            sqlx::query("SELECT raw_data from api_cache WHERE endpoint = $1")
                .bind(endpoint)
                .fetch_optional(pool)
                .await
        )
        .await?
        {
            Some(row) => {
                // if present, get the contents of the raw_data column and return it
                match row.try_get::<String, _>("raw_data") {
                    Ok(raw_data) => {
                        tracing::info!(endpoint = %endpoint, "Record found for endpoint. Retrieving raw_data...");
                        return Ok(raw_data);
                    }
                    Err(e) => {
                        tracing::warn!(
                            endpoint = %endpoint,
                            error = %e,
                            "Cached record for endpoint is unusable. Attempting to refresh from API..."
                        );
                        // fall through to fetch from API
                    }
                }
            }
            None => {
                // otherwise fetch the raw_data, store it in our cache, and return it
                tracing::info!(endpoint = %endpoint, "Cached record not found for endpoint. Querying API...");
                // fall through to fetch from api
            }
        }
        // fetch from api, insert into cache, and return
        let response: reqwest::Response =
            reqwest_with_retries!(self.client().get(endpoint).send().await)
                .await
                .map_err(|e| {
                    lp_error::LPError::ApiCustom(format!(
                        "Failed to fetch from API for endpoint {}:\n\t{}",
                        endpoint, e
                    ))
                })?;

        tracing::info!(endpoint = %endpoint, "Response received. Parsing and inserting into cache...");
        let raw_data = response.text().await.map_err(|e| {
            lp_error::LPError::ApiCustom(format!(
                "Failed to read response body for endpoint {}:\n\t{}",
                endpoint, e
            ))
        })?;
        sqlx_operation_with_retries!(
            sqlx::query(r#"INSERT INTO api_cache (endpoint, raw_data) VALUES ($1, $2)
                ON CONFLICT (endpoint) DO UPDATE SET raw_data = EXCLUDED.raw_data, last_updated = now()"#)
                .bind(endpoint)
                .bind(&raw_data)
                .execute(pool)
                .await
        ).await.map_err(|e| {
            lp_error::LPError::ApiCustom(format!(
                "Failed to insert/update cache for endpoint {}:\n\t{}",
                endpoint, e))
        })?;
        Ok(raw_data)
    }
}
