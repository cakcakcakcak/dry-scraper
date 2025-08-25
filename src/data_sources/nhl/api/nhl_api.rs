use std::fmt::Debug;

use async_trait::async_trait;

use super::{
    nhl_stats_api::{FranchiseResource, NhlStatsApi, SeasonResource, ShiftResource, TeamResource},
    nhl_web_api::{GameResource, NhlWebApi, PlayerResource},
};
use crate::{
    common::api::CacheableApi,
    data_sources::api::nhl_web_api::{PlayoffBracketResource, PlayoffSeriesResource},
};

#[derive(Clone, Debug)]
pub struct NhlApi {
    nhl_stats_api: NhlStatsApi,
    nhl_web_api: NhlWebApi,
}
#[async_trait]
impl CacheableApi for NhlApi {
    fn client(&self) -> &reqwest::Client {
        &self.nhl_stats_api.client
    }
}
impl NhlApi {
    pub fn new() -> Self {
        Self {
            nhl_stats_api: NhlStatsApi::new(),
            nhl_web_api: NhlWebApi::new(),
        }
    }

    pub fn players(&self) -> PlayerResource<'_> {
        self.nhl_web_api.players()
    }

    pub fn games(&self) -> GameResource<'_> {
        self.nhl_web_api.games()
    }

    pub fn playoff_bracket(&self) -> PlayoffBracketResource<'_> {
        self.nhl_web_api.playoff_bracket()
    }

    pub fn playoff_series(&self) -> PlayoffSeriesResource<'_> {
        self.nhl_web_api.playoff_series()
    }

    pub fn seasons(&self) -> SeasonResource<'_> {
        self.nhl_stats_api.seasons()
    }

    pub fn teams(&self) -> TeamResource<'_> {
        self.nhl_stats_api.teams()
    }

    pub fn franchises(&self) -> FranchiseResource<'_> {
        self.nhl_stats_api.franchises()
    }

    pub fn shifts(&self) -> ShiftResource<'_> {
        self.nhl_stats_api.shifts()
    }
}
