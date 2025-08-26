use std::fmt::Debug;

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use serde::de::DeserializeOwned;

use crate::{
    CONFIG,
    common::{
        api::cacheable_api::CacheableApi,
        db::DbContext,
        errors::LPError,
        models::{ItemParsedWithContext, traits::IntoDbStruct},
    },
    data_sources::models::NhlSeasonContext,
    with_progress_bar,
};

use super::super::models::{
    NhlDefaultContext, NhlGameJson, NhlPlayerJson, NhlPlayoffBracketJson,
    NhlPlayoffBracketSeriesJson, NhlPlayoffSeriesJson,
};

#[derive(Clone)]
pub struct NhlWebApi {
    pub client: reqwest::Client,
    pub base_url: String,
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
}
impl NhlWebApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api-web.nhle.com/v1".to_string(),
        }
    }

    pub fn players(&self) -> PlayerResource<'_> {
        PlayerResource { api: self }
    }

    pub fn games(&self) -> GameResource<'_> {
        GameResource { api: self }
    }

    pub fn playoff_bracket(&self) -> PlayoffBracketResource<'_> {
        PlayoffBracketResource { api: self }
    }

    pub fn playoff_series(&self) -> PlayoffSeriesResource<'_> {
        PlayoffSeriesResource { api: self }
    }

    async fn fetch_and_parse<T>(
        &self,
        endpoint: &str,
        db_context: &DbContext,
    ) -> Result<ItemParsedWithContext<T>, LPError>
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
            context: NhlDefaultContext {
                raw_json,
                endpoint: endpoint.to_string(),
            },
        })
    }
}

pub struct PlayerResource<'a> {
    api: &'a NhlWebApi,
}
impl<'a> PlayerResource<'a> {
    pub async fn get(
        &self,
        db_context: &DbContext,
        player_id: i32,
    ) -> Result<ItemParsedWithContext<NhlPlayerJson>, LPError> {
        let endpoint = format!("{}/player/{}/landing", self.api.base_url, player_id);
        self.api
            .fetch_and_parse::<NhlPlayerJson>(&endpoint, db_context)
            .await
    }

    pub async fn _get_many(
        &self,
        db_context: &DbContext,
        player_ids: Vec<i32>,
    ) -> Vec<Result<ItemParsedWithContext<NhlPlayerJson>, LPError>> {
        with_progress_bar!(player_ids.len(), |pb| {
            stream::iter(player_ids)
                .map(|player_id| self.get(db_context, player_id))
                .buffer_unordered(CONFIG.upsert_concurrency)
                .collect()
                .await
        })
    }
}

pub struct GameResource<'a> {
    api: &'a NhlWebApi,
}
impl<'a> GameResource<'a> {
    pub async fn get(
        &self,
        db_context: &DbContext,
        game_id: i32,
    ) -> Result<ItemParsedWithContext<NhlGameJson>, LPError> {
        let endpoint: String = format!("{}/gamecenter/{}/play-by-play", self.api.base_url, game_id);
        self.api
            .fetch_and_parse::<NhlGameJson>(&endpoint, db_context)
            .await
    }

    pub async fn get_many(
        &self,
        db_context: &DbContext,
        game_ids: Vec<i32>,
    ) -> Vec<Result<ItemParsedWithContext<NhlGameJson>, LPError>> {
        with_progress_bar!(game_ids.len(), |pb| {
            stream::iter(game_ids)
                .map(|game_id| self.get(db_context, game_id))
                .buffer_unordered(CONFIG.upsert_concurrency)
                .inspect(|_| pb.inc(1))
                .collect()
                .await
        })
    }
}

pub struct PlayoffBracketResource<'a> {
    api: &'a NhlWebApi,
}
impl<'a> PlayoffBracketResource<'a> {
    pub async fn list_playoff_series_for_year(
        &self,
        db_context: &DbContext,
        year_id: i32,
    ) -> Result<Vec<ItemParsedWithContext<NhlPlayoffBracketSeriesJson>>, LPError> {
        let endpoint: String = format!("{}/playoff-bracket/{}", self.api.base_url, year_id);

        let raw_data = self
            .api
            .fetch_endpoint_cached(db_context, &endpoint)
            .await?;
        let bracket: NhlPlayoffBracketJson = serde_json::from_str(&raw_data).map_err(|e| {
            tracing::warn!(
                endpoint,
                "Failed to parse into `NhlPlayoffBracketJson`: {e}"
            );
            tracing::debug!(raw_data);
            LPError::Serde(e)
        })?;

        let season_id: i32 = format!("{}{}", year_id - 1, year_id)
            .parse::<i32>()
            .unwrap();
        bracket
            .series
            .into_iter()
            .map(|series| {
                let raw_json: serde_json::Value = serde_json::to_value(series.clone()).unwrap();
                Ok(ItemParsedWithContext {
                    item: series,
                    context: NhlSeasonContext {
                        season_id,
                        endpoint: endpoint.clone(),
                        raw_json,
                    },
                })
            })
            .collect()
    }
}

pub struct PlayoffSeriesResource<'a> {
    api: &'a NhlWebApi,
}
impl<'a> PlayoffSeriesResource<'a> {
    pub async fn get(
        &self,
        db_context: &DbContext,
        season_id: i32,
        series_letter: &str,
    ) -> Result<ItemParsedWithContext<NhlPlayoffSeriesJson>, LPError> {
        let endpoint: String = format!(
            "{}/schedule/playoff-series/{}/{}",
            self.api.base_url, season_id, series_letter
        );
        self.api
            .fetch_and_parse::<NhlPlayoffSeriesJson>(&endpoint, db_context)
            .await
    }
}
