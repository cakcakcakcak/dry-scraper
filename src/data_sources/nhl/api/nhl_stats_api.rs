use std::fmt::Debug;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::common::{
    api::cacheable_api::CacheableApi,
    db::DbContext,
    errors::DSError,
    models::{traits::IntoDbStruct, ItemParsedWithContext},
    rate_limiter::{RateLimiter, RateLimiterConfig},
};

use super::super::models::{NhlApiDataArrayResponse, NhlDefaultContext};

#[derive(Clone)]
pub struct NhlStatsApi {
    pub client: reqwest::Client,
    pub base_url: String,
    rate_limiter: RateLimiter,
}
impl Debug for NhlStatsApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NhlStatsApi")
            .field("base_url", &self.base_url)
            .finish()
    }
}
#[async_trait]
impl CacheableApi for NhlStatsApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
    fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}
impl NhlStatsApi {
    pub fn new(rate_limiter_config: RateLimiterConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.nhle.com/stats/rest/en".to_string(),
            rate_limiter: RateLimiter::new(rate_limiter_config),
        }
    }

    pub fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    pub async fn fetch_and_parse<T>(
        &self,
        endpoint: &str,
        db_context: &DbContext,
    ) -> Result<Vec<Result<ItemParsedWithContext<T>, DSError>>, DSError>
    where
        T: DeserializeOwned + Debug + IntoDbStruct<Context = NhlDefaultContext>,
    {
        let raw_data = self.fetch_endpoint_cached(db_context, endpoint).await?;

        let data_array_response: NhlApiDataArrayResponse = serde_json::from_str(&raw_data)
            .map_err(|e| {
                tracing::warn!(
                    endpoint,
                    "Failed to parse into `NhlApiDataArrayResponse`: {e}"
                );
                tracing::info!(raw_data);
                DSError::Serde(e)
            })?;

        let results = data_array_response.map_json_array_to_json_structs(endpoint);
        Ok(results)
    }
}
