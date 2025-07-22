use std::str::FromStr;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::db::DbPool;
use crate::db::persistable::Persistable;
use crate::lp_error::LPError;
use crate::models::traits::{DbStruct, IntoDbStruct};
use crate::serde_helpers::deserialize_to_bool;

use crate::impl_has_type_name;
use crate::sqlx_operation_with_retries;

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
    type U = NhlSeason;

    fn to_db_struct(self) -> Self::U {
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
            endpoint: String::new(),
            raw_json: serde_json::Value::Null,
            last_updated: None,
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
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl std::fmt::Debug for NhlSeason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only print the base_url, not the whole client struct
        f.debug_struct("NhlSeason")
            .field("id", &self.id)
            .field(
                "total_regular_season_games",
                &self.total_regular_season_games,
            )
            .finish()
    }
}
impl DbStruct for NhlSeason {
    fn fill_context(&mut self, endpoint: String, raw_data: String) -> Result<(), LPError> {
        self.endpoint = endpoint;

        let raw_json = serde_json::Value::from_str(&raw_data)?;
        self.raw_json = raw_json;
        Ok(())
    }
}

#[async_trait]
impl Persistable for NhlSeason {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM nhl_season WHERE id=$1"#)
                .bind(id)
                .fetch_optional(pool)
                .await
        )
        .await
        .map_err(LPError::from)
    }

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(r#"INSERT INTO nhl_season (
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
                                    "#)
                    .bind(&self.id)
                    .bind(&self.all_star_game_in_use)
                    .bind(&self.conferences_in_use)
                    .bind(&self.divisions_in_use)
                    .bind(&self.end_date)
                    .bind(&self.entry_draft_in_use)
                    .bind(&self.formatted_season_id)
                    .bind(&self.minimum_playoff_minutes_for_goalie_stats_leaders)
                    .bind(&self.minimum_regular_games_for_goalie_stats_leaders)
                    .bind(&self.nhl_stanley_cup_owner)
                    .bind(&self.number_of_games)
                    .bind(&self.olympics_participation)
                    .bind(&self.point_for_ot_loss_in_use)
                    .bind(&self.preseason_startdate)
                    .bind(&self.regular_season_end_date)
                    .bind(&self.row_in_use)
                    .bind(&self.season_ordinal)
                    .bind(&self.start_date)
                    .bind(&self.supplemental_draft_in_use)
                    .bind(&self.ties_in_use)
                    .bind(&self.total_playoff_games)
                    .bind(&self.total_regular_season_games)
                    .bind(&self.wildcard_in_use)
                    .bind(&self.raw_json)
                    .bind(&self.endpoint)
    }
}

impl_has_type_name!(NhlSeasonJson);
impl_has_type_name!(NhlSeason);
