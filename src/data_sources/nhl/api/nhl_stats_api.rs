use crate::common::{
    api::{
        api_common::{ApiContext, HasEndpoint},
        cacheable_api::CacheableApi,
    },
    db::DbContext,
    errors::LPError,
    models::{
        ItemParsedWithContext,
        traits::{HasTypeName, IntoDbStruct},
    },
    util::filter_results,
};

use super::super::models::{
    DefaultNhlContext, NhlApiDataArrayResponse, NhlFranchiseJson, NhlSeasonJson, NhlTeamJson,
};

#[derive(Clone)]
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
    type Api = NhlStatsApi;
    type Params = ();

    fn endpoint(api: &Self::Api, _params: Self::Params) -> String {
        format!("{}/season", api.base_url())
    }
}
#[derive(Debug, Default)]
pub struct NhlTeamParams {
    pub team_id: Option<i32>,
}
impl HasEndpoint for NhlTeamJson {
    type Api = NhlStatsApi;
    type Params = NhlTeamParams;

    fn endpoint(api: &Self::Api, params: Self::Params) -> String {
        match params.team_id {
            Some(team_id) => format!("{}/team/id/{team_id}", api.base_url()),
            None => format!("{}/team", api.base_url()),
        }
    }
}
impl HasEndpoint for NhlFranchiseJson {
    type Api = NhlStatsApi;
    type Params = ();

    fn endpoint(api: &Self::Api, _params: Self::Params) -> String {
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

    #[tracing::instrument(skip(db_context))]
    pub async fn fetch_nhl_api_data_array<T>(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<ItemParsedWithContext<T>>, LPError>
    where
        T: serde::de::DeserializeOwned
            + HasEndpoint<Api = NhlStatsApi>
            + HasTypeName
            + std::fmt::Debug
            + IntoDbStruct<Context = DefaultNhlContext>,
    {
        let endpoint: String = T::endpoint(self, T::Params::default());
        let raw_data: String = self.get_or_cache_endpoint(&db_context, &endpoint).await?;

        let data_array_response: NhlApiDataArrayResponse = match serde_json::from_str(&raw_data) {
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

        let results: Vec<Result<ItemParsedWithContext<T>, LPError>> =
            data_array_response.map_json_array_to_json_structs(&endpoint);

        Ok(filter_results::<ItemParsedWithContext<T>>(results))
    }

    #[tracing::instrument(skip(db_context))]
    pub async fn get_nhl_team(
        &self,
        db_context: &DbContext,
        team_id: i32,
    ) -> Result<ItemParsedWithContext<NhlTeamJson>, LPError> {
        let endpoint: String = NhlTeamJson::endpoint(
            self,
            NhlTeamParams {
                team_id: Some(team_id),
            },
        );
        let raw_data: String = self.get_or_cache_endpoint(&db_context, &endpoint).await?;

        let data_array_response: NhlApiDataArrayResponse = match serde_json::from_str(&raw_data) {
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

        let results: Vec<Result<ItemParsedWithContext<NhlTeamJson>, LPError>> =
            data_array_response.map_json_array_to_json_structs(&endpoint);
        let mut filtered_results: Vec<ItemParsedWithContext<NhlTeamJson>> =
            filter_results::<ItemParsedWithContext<NhlTeamJson>>(results);

        let team_json: ItemParsedWithContext<NhlTeamJson> = filtered_results
            .pop()
            .ok_or_else(|| LPError::ApiCustom(format!("NHL team with ID {team_id} not found.")))?;

        Ok(team_json)
    }
}
