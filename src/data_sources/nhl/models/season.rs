use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use super::NhlDefaultContext;
use crate::{
    bind,
    common::{
        db::{CacheKey, DbEntity, StaticPgQuery, StaticPgQueryAs},
        models::traits::IntoDbStruct,
        serde_helpers::JsonExt,
    },
    data_sources::NhlSeasonKey,
    impl_has_type_name, impl_pk_debug, make_deserialize_to_type,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlSeasonJson {
    pub id: i32,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub all_star_game_in_use: bool,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub conferences_in_use: bool,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub divisions_in_use: bool,
    pub end_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub entry_draft_in_use: bool,
    pub formatted_season_id: String,
    pub minimum_playoff_minutes_for_goalie_stats_leaders: i32,
    pub minimum_regular_games_for_goalie_stats_leaders: i32,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub nhl_stanley_cup_owner: bool,
    pub number_of_games: i32,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub olympics_participation: bool,
    #[serde(deserialize_with = "deserialize_to_bool")]
    #[serde(rename = "pointForOTLossInUse")]
    pub point_for_ot_loss_in_use: bool,
    pub preseason_startdate: Option<chrono::NaiveDateTime>,
    pub regular_season_end_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub row_in_use: bool,
    pub season_ordinal: i32,
    pub start_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub supplemental_draft_in_use: bool,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub ties_in_use: bool,
    pub total_playoff_games: i32,
    pub total_regular_season_games: i32,
    #[serde(deserialize_with = "deserialize_to_bool")]
    pub wildcard_in_use: bool,
}
impl IntoDbStruct for NhlSeasonJson {
    type DbStruct = NhlSeason;
    type Context = NhlDefaultContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlSeasonJson {
            id,
            all_star_game_in_use,
            conferences_in_use,
            divisions_in_use,
            end_date,
            entry_draft_in_use,
            formatted_season_id,
            minimum_playoff_minutes_for_goalie_stats_leaders,
            minimum_regular_games_for_goalie_stats_leaders,
            nhl_stanley_cup_owner,
            number_of_games,
            olympics_participation,
            point_for_ot_loss_in_use,
            preseason_startdate,
            regular_season_end_date,
            row_in_use,
            season_ordinal,
            start_date,
            supplemental_draft_in_use,
            ties_in_use,
            total_playoff_games,
            total_regular_season_games,
            wildcard_in_use,
        } = self;
        let NhlDefaultContext { endpoint, raw_json } = context;
        NhlSeason {
            id,
            all_star_game_in_use,
            conferences_in_use,
            divisions_in_use,
            end_date,
            entry_draft_in_use,
            formatted_season_id,
            minimum_playoff_minutes_for_goalie_stats_leaders,
            minimum_regular_games_for_goalie_stats_leaders,
            nhl_stanley_cup_owner,
            number_of_games,
            olympics_participation,
            point_for_ot_loss_in_use,
            preseason_startdate,
            regular_season_end_date,
            row_in_use,
            season_ordinal,
            start_date,
            supplemental_draft_in_use,
            ties_in_use,
            total_playoff_games,
            total_regular_season_games,
            wildcard_in_use,
            endpoint,
            raw_json,
        }
    }
}

#[derive(FromRow, Clone)]
pub struct NhlSeason {
    pub id: i32,
    pub all_star_game_in_use: bool,
    pub conferences_in_use: bool,
    pub divisions_in_use: bool,
    pub end_date: chrono::NaiveDateTime,
    pub entry_draft_in_use: bool,
    pub formatted_season_id: String,
    pub minimum_playoff_minutes_for_goalie_stats_leaders: i32,
    pub minimum_regular_games_for_goalie_stats_leaders: i32,
    pub nhl_stanley_cup_owner: bool,
    pub number_of_games: i32,
    pub olympics_participation: bool,
    pub point_for_ot_loss_in_use: bool,
    pub preseason_startdate: Option<chrono::NaiveDateTime>,
    pub regular_season_end_date: chrono::NaiveDateTime,
    pub row_in_use: bool,
    pub season_ordinal: i32,
    pub start_date: chrono::NaiveDateTime,
    pub supplemental_draft_in_use: bool,
    pub ties_in_use: bool,
    pub total_playoff_games: i32,
    pub total_regular_season_games: i32,
    pub wildcard_in_use: bool,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
}

#[async_trait]
impl DbEntity for NhlSeason {
    type Pk = NhlSeasonKey;

    fn pk(&self) -> Self::Pk {
        NhlSeasonKey { id: self.id }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT id from nhl_season")
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        vec![CacheKey {
            source: "nhl",
            table: "api_cache",
            id: self.endpoint.clone(),
        }]
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_season (
                                        id,
                                        all_star_game_in_use,
                                        conferences_in_use,
                                        divisions_in_use,
                                        end_date,
                                        entry_draft_in_use,
                                        formatted_season_id,
                                        minimum_playoff_minutes_for_goalie_stats_leaders,
                                        minimum_regular_games_for_goalie_stats_leaders,
                                        nhl_stanley_cup_owner,
                                        number_of_games,
                                        olympics_participation,
                                        point_for_ot_loss_in_use,
                                        preseason_startdate,
                                        regular_season_end_date,
                                        row_in_use,
                                        season_ordinal,
                                        start_date,
                                        supplemental_draft_in_use,
                                        ties_in_use,
                                        total_playoff_games,
                                        total_regular_season_games,
                                        wildcard_in_use,
                                        raw_json,
                                        endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,
                                        $14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
                                    ON CONFLICT (id) DO UPDATE SET
                                        all_star_game_in_use = EXCLUDED.all_star_game_in_use,
                                        conferences_in_use = EXCLUDED.conferences_in_use,
                                        divisions_in_use = EXCLUDED.divisions_in_use,
                                        end_date = EXCLUDED.end_date,
                                        entry_draft_in_use = EXCLUDED.entry_draft_in_use,
                                        formatted_season_id = EXCLUDED.formatted_season_id,
                                        minimum_playoff_minutes_for_goalie_stats_leaders = EXCLUDED.minimum_playoff_minutes_for_goalie_stats_leaders,
                                        minimum_regular_games_for_goalie_stats_leaders = EXCLUDED.minimum_regular_games_for_goalie_stats_leaders,
                                        nhl_stanley_cup_owner = EXCLUDED.nhl_stanley_cup_owner,
                                        number_of_games = EXCLUDED.number_of_games,
                                        olympics_participation = EXCLUDED.olympics_participation,
                                        point_for_ot_loss_in_use = EXCLUDED.point_for_ot_loss_in_use,
                                        preseason_startdate = EXCLUDED.preseason_startdate,
                                        regular_season_end_date = EXCLUDED.regular_season_end_date,
                                        row_in_use = EXCLUDED.row_in_use,
                                        season_ordinal = EXCLUDED.season_ordinal,
                                        start_date = EXCLUDED.start_date,
                                        supplemental_draft_in_use = EXCLUDED.supplemental_draft_in_use,
                                        ties_in_use = EXCLUDED.ties_in_use,
                                        total_playoff_games = EXCLUDED.total_playoff_games,
                                        total_regular_season_games = EXCLUDED.total_regular_season_games,
                                        wildcard_in_use = EXCLUDED.wildcard_in_use,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#
            ),
            self.id,
            self.all_star_game_in_use,
            self.conferences_in_use,
            self.divisions_in_use,
            self.end_date,
            self.entry_draft_in_use,
            self.formatted_season_id,
            self.minimum_playoff_minutes_for_goalie_stats_leaders,
            self.minimum_regular_games_for_goalie_stats_leaders,
            self.nhl_stanley_cup_owner,
            self.number_of_games,
            self.olympics_participation,
            self.point_for_ot_loss_in_use,
            self.preseason_startdate,
            self.regular_season_end_date,
            self.row_in_use,
            self.season_ordinal,
            self.start_date,
            self.supplemental_draft_in_use,
            self.ties_in_use,
            self.total_playoff_games,
            self.total_regular_season_games,
            self.wildcard_in_use,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlSeasonJson);
impl_has_type_name!(NhlSeason);
impl_pk_debug!(NhlSeason);
make_deserialize_to_type!(deserialize_to_bool, bool);
