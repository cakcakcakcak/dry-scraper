use crate::api::api_common::{ApiContext, FromId, HasEndpoint};
use crate::api::cacheable_api::CacheableApi;
use crate::db::DbPool;
use crate::lp_error::LPError;
use crate::models::item_parsed_with_context::ItemParsedWithContext;
use crate::models::nhl::nhl_game::NhlGameJson;
use crate::models::nhl::nhl_player::NhlPlayerJson;

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
    type Params = NhlPlayerParams;

    fn endpoint<A: ApiContext>(api: &A, params: Self::Params) -> String {
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
    type Params = NhlGameParams;

    fn endpoint<A: ApiContext>(api: &A, params: Self::Params) -> String {
        format!(
            "{}/gamecenter/{}/play-by-play",
            api.base_url(),
            params.game_id
        )
    }
}
impl NhlWebApi {
    #[tracing::instrument(skip(pool))]
    pub async fn fetch_from_id<T>(
        &self,
        pool: &DbPool,
        id: i32,
    ) -> Result<ItemParsedWithContext<T>, LPError>
    where
        T: serde::de::DeserializeOwned + HasEndpoint,
        T::Params: FromId,
    {
        let endpoint: String = T::endpoint(self, T::Params::from_id(id));
        let raw_data: String = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let json_value: serde_json::Value = serde_json::from_str(&raw_data)?;

        let parsed: Result<T, LPError> =
            serde_json::from_value(json_value.clone()).map_err(LPError::from);
        match parsed {
            Ok(item) => Ok(ItemParsedWithContext {
                raw_data,
                item,
                endpoint: endpoint.to_string(),
            }),
            Err(e) => Err(e),
        }
    }
}
