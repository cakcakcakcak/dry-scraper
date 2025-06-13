use async_trait::async_trait;
use reqwest::Response;
use serde_json;
use sqlx::Row;
use std::error::Error;
use tokio;

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pool = config::init_db().await?;

    Ok(())
}

pub struct NhlWebApi {
    client: reqwest::Client,
    base_url: String,
}

impl NhlWebApi {
    fn new() -> Self {
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

#[async_trait]
pub trait CacheableApi {
    fn client(&self) -> &reqwest::Client;

    async fn get_or_cache_endpoint(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        endpoint: &str,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        if let Some(row) = sqlx::query("SELECT raw_data FROM api_cache WHERE endpoint = $1")
            .bind(endpoint)
            .fetch_optional(pool)
            .await?
        {
            let raw_data: String = row.try_get("raw_data")?;
            let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;
            return Ok(raw_json);
        }
        let response = self.client().get(endpoint).send().await?;
        let json: serde_json::Value = response.json().await?;

        sqlx::query("INSERT INTO api_cache (endpoint, raw_data) VALUES ($1, $2) ON CONFLICT (endpoint) DO UPDATE SET raw_data = EXCLUDED.raw_data, last_updated = now()")
            .bind(endpoint)
            .bind(&json.to_string())
            .execute(pool)
            .await?;

        Ok(json) //placeholder value
    }
}
pub struct NhlStatsApi {
    client: reqwest::Client,
    base_url: String,
}
impl CacheableApi for NhlStatsApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl NhlStatsApi {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.nhle.com/stats/rest".to_string(),
        }
    }

    async fn get_nhl_seasons(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let endpoint = format!("{}/season", self.base_url);
        let raw_json = self.get_or_cache_endpoint(pool, &endpoint).await?;

        Ok(vec![])
    }
}
