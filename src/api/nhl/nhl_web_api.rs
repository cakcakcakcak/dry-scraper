use crate::api::api_common::{ApiContext, FromId, HasEndpoint};
use crate::api::cacheable_api::CacheableApi;
use crate::db::DbContext;
use crate::lp_error::LPError;
use crate::models::ItemParsedWithContext;
use crate::models::nhl::{DefaultNhlContext, NhlGameJson, NhlPlayerJson};
use crate::models::traits::{HasTypeName, IntoDbStruct};

#[derive(Clone)]
pub struct NhlWebApi {
    pub client: reqwest::Client,
    pub base_url: String,
}
impl NhlWebApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api-web.nhle.com/v1".to_string(),
        }
    }
}
impl std::fmt::Debug for NhlWebApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // don't print the Client struct or the base_url
        f.debug_struct("NhlWebApi").finish()
    }
}
impl CacheableApi for NhlWebApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl ApiContext for NhlWebApi {
    fn base_url(&self) -> &str {
        &self.base_url
    }
}
#[derive(Debug, Default)]
pub struct NhlPlayerParams {
    pub player_id: i32,
}
impl FromId for NhlPlayerParams {
    fn from_id(player_id: i32) -> Self {
        NhlPlayerParams { player_id }
    }
}
impl HasEndpoint for NhlPlayerJson {
    type Api = NhlWebApi;
    type Params = NhlPlayerParams;

    fn endpoint(api: &Self::Api, params: Self::Params) -> String {
        format!("{}/player/{}/landing", api.base_url(), params.player_id)
    }
}
#[derive(Debug, Default)]
pub struct NhlGameParams {
    pub game_id: i32,
}
impl FromId for NhlGameParams {
    fn from_id(game_id: i32) -> Self {
        NhlGameParams { game_id }
    }
}
impl HasEndpoint for NhlGameJson {
    type Api = NhlWebApi;
    type Params = NhlGameParams;

    fn endpoint(api: &Self::Api, params: Self::Params) -> String {
        format!(
            "{}/gamecenter/{}/play-by-play",
            api.base_url(),
            params.game_id
        )
    }
}
impl NhlWebApi {
    #[tracing::instrument(skip(db_context))]
    pub async fn fetch_from_id<T>(
        &self,
        db_context: &DbContext,
        id: i32,
    ) -> Result<ItemParsedWithContext<T>, LPError>
    where
        T: serde::de::DeserializeOwned
            + HasEndpoint<Api = NhlWebApi>
            + IntoDbStruct<Context = DefaultNhlContext>
            + HasTypeName,
        T::Params: FromId,
    {
        let endpoint: String = T::endpoint(self, T::Params::from_id(id));
        let raw_data: String = self.get_or_cache_endpoint(&db_context, &endpoint).await?;
        let raw_json: serde_json::Value = match serde_json::from_str(&raw_data) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    endpoint,
                    "Failed to parse `raw_data` into `serde_json::Value`: {e}"
                );
                tracing::debug!(raw_data);
                return Err(LPError::Serde(e));
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
                tracing::debug!(raw_data);
                return Err(LPError::Serde(e));
            }
        };

        Ok(ItemParsedWithContext {
            item,
            context: DefaultNhlContext { endpoint, raw_json },
        })
    }
}
