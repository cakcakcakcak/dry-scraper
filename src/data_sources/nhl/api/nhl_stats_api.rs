use std::fmt::Debug;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::common::{
    api::cacheable_api::CacheableApi,
    db::DbContext,
    errors::LPError,
    models::{ItemParsedWithContext, traits::IntoDbStruct},
    util::track_and_filter_errors,
};

use super::super::models::{
    NhlApiDataArrayResponse, NhlDefaultContext, NhlFranchiseJson, NhlSeasonJson, NhlShiftJson,
    NhlTeamJson,
};

#[derive(Clone)]
pub struct NhlStatsApi {
    pub client: reqwest::Client,
    pub base_url: String,
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
}
impl NhlStatsApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.nhle.com/stats/rest/en".to_string(),
        }
    }

    pub fn seasons(&self) -> SeasonResource<'_> {
        SeasonResource { api: self }
    }

    pub fn teams(&self) -> TeamResource<'_> {
        TeamResource { api: self }
    }

    pub fn franchises(&self) -> FranchiseResource<'_> {
        FranchiseResource { api: self }
    }

    pub fn shifts(&self) -> ShiftResource<'_> {
        ShiftResource { api: self }
    }

    async fn fetch_and_parse<T>(
        &self,
        endpoint: &str,
        db_context: &DbContext,
    ) -> Result<Vec<ItemParsedWithContext<T>>, LPError>
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
                tracing::debug!(raw_data);
                LPError::Serde(e)
            })?;

        let results = data_array_response.map_json_array_to_json_structs(endpoint);
        Ok(track_and_filter_errors(results, db_context).await)
    }
}

pub struct SeasonResource<'a> {
    api: &'a NhlStatsApi,
}
impl<'a> SeasonResource<'a> {
    pub async fn list(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<ItemParsedWithContext<NhlSeasonJson>>, LPError> {
        let endpoint = format!("{}/season", self.api.base_url);
        self.api
            .fetch_and_parse(endpoint.as_str(), db_context)
            .await
    }
}

pub struct TeamResource<'a> {
    api: &'a NhlStatsApi,
}
impl<'a> TeamResource<'a> {
    pub async fn list(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<ItemParsedWithContext<NhlTeamJson>>, LPError> {
        let endpoint = format!("{}/team", self.api.base_url);
        self.api
            .fetch_and_parse(endpoint.as_str(), db_context)
            .await
    }

    pub async fn get(
        &self,
        db_context: &DbContext,
        team_id: i32,
    ) -> Result<ItemParsedWithContext<NhlTeamJson>, LPError> {
        let endpoint = format!("{}/team/id/{team_id}", self.api.base_url);
        let mut results = self
            .api
            .fetch_and_parse(endpoint.as_str(), db_context)
            .await?;

        results
            .pop()
            .ok_or_else(|| LPError::ApiCustom(format!("NHL team with id {team_id} not found.")))
    }
}

pub struct FranchiseResource<'a> {
    api: &'a NhlStatsApi,
}
impl<'a> FranchiseResource<'a> {
    pub async fn list(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<ItemParsedWithContext<NhlFranchiseJson>>, LPError> {
        let endpoint = format!("{}/franchise", self.api.base_url);
        self.api
            .fetch_and_parse(endpoint.as_str(), db_context)
            .await
    }
}

pub struct ShiftResource<'a> {
    api: &'a NhlStatsApi,
}
impl<'a> ShiftResource<'a> {
    pub async fn list_shifts_for_game(
        &self,
        db_context: &DbContext,
        game_id: i32,
    ) -> Result<Vec<ItemParsedWithContext<NhlShiftJson>>, LPError> {
        let endpoint = format!(
            "{}/shiftcharts?cayenneExp=gameId={game_id}",
            self.api.base_url,
        );
        self.api
            .fetch_and_parse(endpoint.as_str(), db_context)
            .await
    }
}
