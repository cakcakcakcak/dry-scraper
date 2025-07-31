use crate::api::api_common::HasEndpoint;
use crate::db::DbContext;
use crate::lp_error::LPError;
use crate::models::ItemParsedWithContext;
use crate::models::nhl::{DefaultNhlContext, NhlTeamJson};
use crate::models::traits::{HasTypeName, IntoDbStruct};

use super::super::FromId;
use super::{NhlStatsApi, NhlWebApi};

pub struct NhlApi {
    nhl_stats_api: NhlStatsApi,
    nhl_web_api: NhlWebApi,
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
        T: serde::de::DeserializeOwned
            + HasEndpoint<Api = NhlStatsApi>
            + HasTypeName
            + std::fmt::Debug
            + IntoDbStruct<Context = DefaultNhlContext>,
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
            + IntoDbStruct<Context = DefaultNhlContext>
            + HasTypeName,
        T::Params: FromId,
    {
        self.nhl_web_api.fetch_by_id::<T>(db_context, id).await
    }
}
