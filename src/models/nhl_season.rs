use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::serde_helpers::{int_to_bool, number_to_string};

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlSeason {
    #[serde(deserialize_with = "number_to_string")]
    pub id: String,
    #[serde(deserialize_with = "int_to_bool")]
    pub all_star_game_in_use: bool,
    #[serde(deserialize_with = "int_to_bool")]
    pub conferences_in_use: bool,
    #[serde(deserialize_with = "int_to_bool")]
    pub divisions_in_use: bool,
    pub end_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "int_to_bool")]
    pub entry_draft_in_use: bool,
    pub formatted_season_id: String,
    pub minimum_playoff_minutes_for_goalie_stats_leaders: i32,
    pub minimum_regular_games_for_goalie_stats_leaders: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub nhl_stanley_cup_owner: bool,
    pub number_of_games: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub olympics_participation: bool,
    #[serde(deserialize_with = "int_to_bool")]
    #[serde(rename = "pointForOTLossInUse")]
    pub point_for_ot_loss_in_use: bool,
    pub preseason_startdate: Option<chrono::NaiveDateTime>,
    pub regular_season_end_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "int_to_bool")]
    pub row_in_use: bool,
    pub season_ordinal: i32,
    pub start_date: chrono::NaiveDateTime,
    #[serde(deserialize_with = "int_to_bool")]
    pub supplemental_draft_in_use: bool,
    #[serde(deserialize_with = "int_to_bool")]
    pub ties_in_use: bool,
    pub total_playoff_games: i32,
    pub total_regular_season_games: i32,
    #[serde(deserialize_with = "int_to_bool")]
    pub wildcard_in_use: bool,
    pub api_cache_endpoint: Option<String>,
    pub raw_json: Option<serde_json::Value>,
    pub last_updated: Option<chrono::NaiveDateTime>,
}

impl NhlSeason {
    pub async fn upsert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
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
                                    api_cache_endpoint
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
                                    api_cache_endpoint = EXCLUDED.api_cache_endpoint,
                                    last_updated = now()
                                "#)
                .bind(&self.id)
                .bind(self.all_star_game_in_use)
                .bind(self.conferences_in_use)
                .bind(self.divisions_in_use)
                .bind(self.end_date)
                .bind(self.entry_draft_in_use)
                .bind(&self.formatted_season_id)
                .bind(self.minimum_playoff_minutes_for_goalie_stats_leaders)
                .bind(self.minimum_regular_games_for_goalie_stats_leaders)
                .bind(self.nhl_stanley_cup_owner)
                .bind(self.number_of_games)
                .bind(self.olympics_participation)
                .bind(self.point_for_ot_loss_in_use)
                .bind(&self.preseason_startdate)
                .bind(self.regular_season_end_date)
                .bind(self.row_in_use)
                .bind(self.season_ordinal)
                .bind(self.start_date)
                .bind(self.supplemental_draft_in_use)
                .bind(self.ties_in_use)
                .bind(self.total_playoff_games)
                .bind(self.total_regular_season_games)
                .bind(self.wildcard_in_use)
                .bind(&self.raw_json)
                .bind(&self.api_cache_endpoint)
                .execute(pool)
                .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_nhl_season_deserialization() {
        let j = json!({"id":19531954,
            "allStarGameInUse":1,
            "conferencesInUse":0,
            "divisionsInUse":0,
            "endDate":"1954-04-16T20:00:00",
            "entryDraftInUse":0,
            "formattedSeasonId":"1953-54",
            "minimumPlayoffMinutesForGoalieStatsLeaders":60,
            "minimumRegularGamesForGoalieStatsLeaders":21,
            "nhlStanleyCupOwner":1,
            "numberOfGames":70,
            "olympicsParticipation":0,
            "pointForOTLossInUse":0,
            "preseasonStartdate":null,
            "regularSeasonEndDate":"1954-03-21T20:00:00",
            "rowInUse":0,
            "seasonOrdinal":37,
            "startDate":"1953-10-08T20:00:00",
            "supplementalDraftInUse":0,
            "tiesInUse":1,
            "totalPlayoffGames":16,
            "totalRegularSeasonGames":210,
            "wildcardInUse":0});
        let season: NhlSeason = serde_json::from_value(j).unwrap();
        assert_eq!(season.id, "19531954");
        assert!(season.all_star_game_in_use);
        assert!(!season.olympics_participation);
    }
}
