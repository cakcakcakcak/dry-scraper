use std::fmt::Debug;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::common::{
    api::cacheable_api::CacheableApi,
    db::DbContext,
    errors::DSError,
    models::{traits::IntoDbStruct, ItemParsedWithContext},
    rate_limiter::RateLimiter,
};

use super::super::models::NhlDefaultContext;

#[derive(Clone)]
pub struct NhlWebApi {
    pub client: reqwest::Client,
    pub base_url: String,
    rate_limiter: RateLimiter,
}
impl Debug for NhlWebApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // don't print the Client struct or the base_url
        f.debug_struct("NhlWebApi").finish()
    }
}
#[async_trait]
impl CacheableApi for NhlWebApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
    fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}
impl NhlWebApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api-web.nhle.com/v1".to_string(),
            rate_limiter: RateLimiter::new(1),
        }
    }

    pub fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    pub async fn fetch_and_parse<T>(
        &self,
        endpoint: &str,
        db_context: &DbContext,
    ) -> Result<ItemParsedWithContext<T>, DSError>
    where
        T: DeserializeOwned + Debug + IntoDbStruct<Context = NhlDefaultContext>,
    {
        let raw_data: String = self.fetch_endpoint_cached(db_context, endpoint).await?;
        let raw_json: serde_json::Value = match serde_json::from_str(&raw_data) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    endpoint,
                    "Failed to parse `raw_data` into `serde_json::Value`: {e}"
                );
                tracing::info!(raw_data);
                return Err(DSError::Serde(e));
            }
        };
        let item: T = match serde_json::from_str(&raw_data) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    endpoint,
                    "Failed to parse `raw_data` into `{}`: {e}",
                    T::type_name()
                );
                tracing::info!(raw_data);
                return Err(DSError::Serde(e));
            }
        };

        Ok(ItemParsedWithContext {
            item,
            context: NhlDefaultContext {
                raw_json,
                endpoint: endpoint.to_string(),
            },
        })
    }
}
