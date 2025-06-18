use async_trait::async_trait;
use sqlx::Row;

use crate::lp_error;
use crate::models;
use crate::util;

use crate::reqwest_with_retries;
use crate::sqlx_operation_with_retries;

#[async_trait]
pub trait CacheableApi {
    fn client(&self) -> &reqwest::Client;

    async fn get_or_cache_endpoint(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        endpoint: &str,
    ) -> Result<String, lp_error::LPError> {
        // query our api_cache for the endpoint we seek
        match sqlx_operation_with_retries!(
            sqlx::query("SELECT raw_data from api_cache WHERE endpoint = $1")
                .bind(endpoint)
                .fetch_optional(pool)
                .await
        )? {
            Some(row) => {
                // if present, get the contents of the raw_data column and return it
                let raw_data: String = row.try_get("raw_data").map_err(|e| match e {
                    sqlx::Error::ColumnNotFound(_) => lp_error::LPError::DatabaseCustom(format!(
                        "Column not found when retrieving cached value for endpoint '{}': {}",
                        endpoint, e
                    )),
                    sqlx::Error::ColumnDecode { .. } => lp_error::LPError::DatabaseCustom(format!(
                        "Unable to decode column when retrieving cached value for endpoint '{}': {}",
                        endpoint, e
                    )),
                    _ => lp_error::LPError::DatabaseCustom(format!(
                        "Cache retrieval failed for endpoint '{}': {}",
                        endpoint, e
                    )),
                })?;
                Ok(raw_data)
            }
            None => {
                // otherwise fetch the raw_data, store it in our cache, and return it
                let response: reqwest::Response =
                    reqwest_with_retries!(self.client().get(endpoint).send().await)?;

                let raw_data = response.text().await?;
                sqlx_operation_with_retries!(
                    sqlx::query(r#"INSERT INTO api_cache (endpoint, raw_data) VALUES ($1, $2)
                        ON CONFLICT (endpoint) DO UPDATE SET raw_data = EXCLUDED.raw_data, last_updated = now()"#)
                        .bind(endpoint)
                        .bind(&raw_data)
                        .execute(pool)
                        .await
                )?;
                Ok(raw_data)
            }
        }
    }
}

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
impl CacheableApi for NhlWebApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

pub struct NhlStatsApi {
    pub client: reqwest::Client,
    pub base_url: String,
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
    ) -> Result<Vec<models::NhlSeason>, lp_error::LPError> {
        // query nhl_season database to see if the desired data is already present
        let seasons: Vec<models::NhlSeason> =
            sqlx::query_as::<sqlx::Postgres, models::NhlSeason>("SELECT * FROM nhl_season")
                .fetch_all(pool)
                .await?;
        if !seasons.is_empty() {
            return Ok(seasons);
        }

        // construct endpoint url
        let endpoint = format!("{}/season", self.base_url);

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
            let mut season: models::NhlSeason = serde_json::from_value(item.clone())?;
            season.raw_json = Some(item.clone());
            season.api_cache_endpoint = Some(endpoint.clone());
            seasons.push(season);
        }

        // build the upsert futures, run them concurrently, and propagate any errors
        let upserts = seasons.iter().map(|season| season.upsert(pool));
        util::run_futures_concurrently(upserts).await?;

        Ok(seasons)
    }
}
