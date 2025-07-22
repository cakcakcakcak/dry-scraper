use crate::api::api_common::{ApiContext, HasEndpoint};
use crate::api::cacheable_api::CacheableApi;
use crate::db::DbPool;
use crate::lp_error::LPError;
use crate::models::item_parsed_with_context::ItemParsedWithContext;
use crate::models::nhl::nhl_franchise::NhlFranchiseJson;
use crate::models::nhl::nhl_model_common::NhlApiDataArrayResponse;
use crate::models::nhl::nhl_season::NhlSeasonJson;
use crate::models::nhl::nhl_team::NhlTeamJson;
use crate::util::filter_and_log_results;

pub struct NhlStatsApi {
    pub client: reqwest::Client,
    pub base_url: String,
}
impl std::fmt::Debug for NhlStatsApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
impl ApiContext for NhlStatsApi {
    fn base_url(&self) -> &str {
        &self.base_url
    }
}
impl HasEndpoint for NhlSeasonJson {
    type Params = ();

    fn endpoint<A: ApiContext>(api: &A, _params: Self::Params) -> String {
        format!("{}/season", api.base_url())
    }
}
#[derive(Debug, Default)]
pub struct NhlTeamParams {
    pub team_id: Option<i32>,
}
impl HasEndpoint for NhlTeamJson {
    type Params = NhlTeamParams;

    fn endpoint<A: ApiContext>(api: &A, _params: Self::Params) -> String {
        format!("{}/team", api.base_url())
    }
}
impl HasEndpoint for NhlFranchiseJson {
    type Params = ();

    fn endpoint<A: ApiContext>(api: &A, _params: Self::Params) -> String {
        format!("{}/franchise", api.base_url())
    }
}

impl NhlStatsApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.nhle.com/stats/rest/en".to_string(),
        }
    }
    pub fn nhl_team_endpoint(&self, team_id: i32) -> String {
        format!("{}/team/id/{team_id}", self.base_url)
    }

    #[tracing::instrument(skip(pool))]
    pub async fn fetch_nhl_api_data_array<T>(
        &self,
        pool: &DbPool,
    ) -> Result<Vec<ItemParsedWithContext<T>>, LPError>
    where
        T: serde::de::DeserializeOwned + HasEndpoint + std::fmt::Debug,
    {
        let endpoint: String = T::endpoint(self, T::Params::default());
        let raw_data: String = self.get_or_cache_endpoint(pool, &endpoint).await?;

        let json_value: serde_json::Value = serde_json::from_str(&raw_data)?;
        let data_array_response: NhlApiDataArrayResponse =
            serde_json::from_value(json_value.clone()).map_err(|e| LPError::Serde(e))?;

        let results: Vec<Result<ItemParsedWithContext<T>, LPError>> =
            data_array_response.map_json_array_to_json_structs(&endpoint);

        Ok(filter_and_log_results::<ItemParsedWithContext<T>>(results))
    }

    #[tracing::instrument(skip(pool))]
    pub async fn get_nhl_team(
        &self,
        pool: &DbPool,
        team_id: i32,
    ) -> Result<ItemParsedWithContext<NhlTeamJson>, LPError> {
        let endpoint: String = self.nhl_team_endpoint(team_id);
        let raw_data: String = self.get_or_cache_endpoint(pool, &endpoint).await?;

        let json_value: serde_json::Value = serde_json::from_str(&raw_data)?;
        let data_array_response: NhlApiDataArrayResponse =
            serde_json::from_value(json_value.clone()).map_err(|e| LPError::Serde(e))?;

        let results: Vec<Result<ItemParsedWithContext<NhlTeamJson>, LPError>> =
            data_array_response.map_json_array_to_json_structs(&endpoint);
        let mut filtered_results: Vec<ItemParsedWithContext<NhlTeamJson>> =
            filter_and_log_results::<ItemParsedWithContext<NhlTeamJson>>(results);

        let team_json: ItemParsedWithContext<NhlTeamJson> = filtered_results
            .pop()
            .ok_or_else(|| LPError::ApiCustom(format!("NHL team with ID {team_id} not found.")))?;

        Ok(team_json)
    }
}
