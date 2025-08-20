use std::fmt::Debug;

use async_trait::async_trait;

use super::{NhlStatsApi, NhlWebApi};
use crate::{
    common::{
        api::{CacheableApi, FromId, HasEndpoint},
        db::DbContext,
        errors::LPError,
        models::{ItemParsedWithContext, traits::IntoDbStruct},
    },
    data_sources::nhl::models::{DefaultNhlContext, NhlTeamJson},
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

    pub async fn fetch_nhl_api_data_array<T>(
        &self,
        db_context: &DbContext,
    ) -> Result<Vec<ItemParsedWithContext<T>>, LPError>
    where
        T: HasEndpoint<Api = NhlStatsApi> + Debug + IntoDbStruct<Context = DefaultNhlContext>,
    {
        self.nhl_stats_api
            .fetch_nhl_api_data_array::<T>(db_context)
            .await
    }

    pub async fn get_nhl_team(
        &self,
        db_context: &DbContext,
        team_id: i32,
    ) -> Result<ItemParsedWithContext<NhlTeamJson>, LPError> {
        self.nhl_stats_api.get_nhl_team(db_context, team_id).await
    }

    pub async fn fetch_by_id<T>(
        &self,
        db_context: &DbContext,
        id: i32,
    ) -> Result<ItemParsedWithContext<T>, LPError>
    where
        T: serde::de::DeserializeOwned
            + HasEndpoint<Api = NhlWebApi>
            + IntoDbStruct<Context = DefaultNhlContext>,
        T::Params: FromId,
    {
        self.nhl_web_api.fetch_by_id::<T>(db_context, id).await
    }
}
