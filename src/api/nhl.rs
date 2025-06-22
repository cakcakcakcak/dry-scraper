//! Provides API clients and related functionality for interacting with NHL web and stats APIs.
//!
//! This module defines two main API client structs:
//! - [`NhlWebApi`]: A client for the NHL web API (`https://api-web.nhle.com/`).
//! - [`NhlStatsApi`]: A client for the NHL stats API (`https://api.nhle.com/stats/rest/en`).
//!
//! Both clients implement the [`CacheableApi`] trait, allowing for endpoint caching and reuse of the underlying HTTP client.
//!
//! # Structs
//! - [`NhlWebApi`]: Handles requests to the NHL web API. Only exposes the `base_url` in debug output.
//! - [`NhlStatsApi`]: Handles requests to the NHL stats API. Provides a method to fetch and cache NHL season data, upserting it into a database if not already present. Only exposes the `base_url` in debug output.
//!
//! # Methods
//! - [`NhlWebApi::new`]: Constructs a new `NhlWebApi` client.
//! - [`NhlStatsApi::new`]: Constructs a new `NhlStatsApi` client.
//! - [`NhlStatsApi::get_nhl_seasons`]: Asynchronously retrieves NHL season data, first checking the database cache, then fetching from the API if necessary, and upserts the results into the database.
//!
//! # Dependencies
//! - Uses `reqwest` for HTTP requests.
//! - Uses `sqlx` for database operations.
//! - Uses `serde_json` for JSON parsing.
//! - Relies on custom error handling via `lp_error::LPError`.
//!
//! # Example
//! ```rust
//! let stats_api = NhlStatsApi::new();
//! let seasons = stats_api.get_nhl_seasons(&pool).await?;
//! ```

use futures::future::join_all;

use crate::lp_error;

use crate::api::cacheable::CacheableApi;
use crate::models::nhl_season::NhlSeason;
use crate::sqlx_operation_with_retries;

pub struct NhlWebApi {
    pub client: reqwest::Client,
    pub base_url: String,
}
impl NhlWebApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api-web.nhle.com/".to_string(),
        }
    }
}
impl std::fmt::Debug for NhlWebApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only print the base_url, not the whole client struct
        f.debug_struct("NhlWebApi")
            .field("base_url", &self.base_url)
            .finish()
    }
}
impl CacheableApi for NhlWebApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

pub struct NhlStatsApi {
    pub client: reqwest::Client,
    pub base_url: String,
}
impl std::fmt::Debug for NhlStatsApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only print the base_url, not the whole client struct
        f.debug_struct("NhlStatsApi")
            .field("base_url", &self.base_url)
            .finish()
    }
}
impl CacheableApi for NhlStatsApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl NhlStatsApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.nhle.com/stats/rest/en".to_string(),
        }
    }

    pub async fn get_nhl_seasons(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Vec<NhlSeason>, lp_error::LPError> {
        // query nhl_season database to see if the desired data is already present
        let seasons: Vec<NhlSeason> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlSeason>("SELECT * FROM nhl_season")
                .fetch_all(pool)
                .await
        )
        .await?;
        if !seasons.is_empty() {
            tracing::info!(
                "Returning {} cached NHL seasons from database.",
                seasons.len()
            );
            return Ok(seasons);
        }

        // construct endpoint url
        let endpoint = format!("{}/season", self.base_url);
        tracing::info!(endpoint = %endpoint, "No cached seasons found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let data = raw_json
            .get("data")
            .ok_or_else(|| {
                lp_error::LPError::ApiCustom(format!(
                    "No 'data' field found in API response for endpoint {}",
                    &endpoint
                ))
            })?
            .clone();

        // parse data field into an array of nhl seasons
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;
        let mut seasons = Vec::with_capacity(data_array.len());

        // for each item in the seasons array, serde the json object into the NhlSeason struct
        // and add the raw_json and api_cache_endpoint fields that are not present in the api response
        for item in data_array {
            let mut season: NhlSeason = serde_json::from_value(item.clone())?;
            season.raw_json = Some(item.clone());
            season.api_cache_endpoint = Some(endpoint.clone());
            seasons.push(season);
        }

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting seasons into database...");
        let upserts = seasons.iter().map(|season| season.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed seasons
        for (season, result) in seasons.iter().zip(upsert_results.iter()) {
            if let Err(e) = result {
                tracing::warn!(
                    season_id = season.id,
                    error = ?e,
                    "Failed to upsert NHL season"
                );
            }
        }

        tracing::info!(
            "Upserted {} NHL seasons into database. Now returning them.",
            seasons.len()
        );

        Ok(seasons)
    }
}
