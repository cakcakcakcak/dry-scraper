use futures::stream::{self, StreamExt};

use crate::api::api_common::HasEndpoint;
use crate::api::nhl::nhl_stats_api::NhlStatsApi;
use crate::api::nhl::nhl_web_api::NhlWebApi;
use crate::config::CONFIG;
use crate::db::DbPool;
use crate::db::persistable::Persistable;
use crate::lp_error::LPError;
use crate::models::item_parsed_with_context::ItemParsedWithContext;
use crate::models::nhl::nhl_franchise::{NhlFranchise, NhlFranchiseJson};
use crate::models::nhl::nhl_game::{NhlGame, NhlGameJson};
use crate::models::nhl::nhl_player::{NhlPlayer, NhlPlayerJson};
use crate::models::nhl::nhl_season::{NhlSeason, NhlSeasonJson};
use crate::models::nhl::nhl_team::{NhlTeam, NhlTeamJson};
use crate::models::traits::{DbStruct, HasTypeName, IntoDbStruct};
use crate::util::filter_and_log_results;

#[tracing::instrument(skip(pool, nhl_stats_api))]
async fn get_nhl_api_data_array<T>(
    pool: &DbPool,
    nhl_stats_api: &NhlStatsApi,
) -> Result<Vec<T::U>, LPError>
where
    T: serde::de::DeserializeOwned + HasEndpoint + IntoDbStruct,
    T::U: std::fmt::Debug + DbStruct + Persistable + HasTypeName + Clone + Send + Sync,
{
    let data_array: Vec<ItemParsedWithContext<T>> =
        nhl_stats_api.fetch_nhl_api_data_array::<T>(&pool).await?;

    let db_records: Vec<T::U> = data_array
        .into_iter()
        .map(|record: ItemParsedWithContext<T>| record.to_db_struct())
        .collect();
    let count = db_records.len();

    // VERIFY_RELATIONSHIPS
    // CORRECT ANY MISSING PIECES
    tracing::info!(
        "Upserting {count} `{}`s into lp database.",
        T::U::type_name()
    );
    T::U::upsert_all(db_records.clone(), pool).await?;
    tracing::info!(
        "Upserted {count} `{}`s into lp database. Now returning them.",
        T::U::type_name()
    );

    Ok(db_records)
}

#[tracing::instrument(skip(pool, nhl_stats_api))]
pub async fn get_nhl_seasons(
    pool: &DbPool,
    nhl_stats_api: &NhlStatsApi,
) -> Result<Vec<NhlSeason>, LPError> {
    let db_seasons = get_nhl_api_data_array::<NhlSeasonJson>(&pool, &nhl_stats_api).await?;

    Ok(db_seasons)
}

#[tracing::instrument(skip(pool, nhl_stats_api))]
pub async fn get_nhl_franchises(
    pool: &DbPool,
    nhl_stats_api: &NhlStatsApi,
) -> Result<Vec<NhlFranchise>, LPError> {
    let db_franchises = get_nhl_api_data_array::<NhlFranchiseJson>(&pool, &nhl_stats_api).await?;

    Ok(db_franchises)
}

#[tracing::instrument(skip(pool, nhl_stats_api))]
pub async fn get_nhl_teams(
    pool: &DbPool,
    nhl_stats_api: &NhlStatsApi,
) -> Result<Vec<NhlTeam>, LPError> {
    let db_teams = get_nhl_api_data_array::<NhlTeamJson>(&pool, &nhl_stats_api).await?;

    Ok(db_teams)
}

#[tracing::instrument(skip(pool, nhl_stats_api))]
pub async fn get_nhl_team(
    pool: &DbPool,
    nhl_stats_api: &NhlStatsApi,
    team_id: i32,
) -> Result<NhlTeam, LPError> {
    let team_json_with_context: ItemParsedWithContext<NhlTeamJson> =
        nhl_stats_api.get_nhl_team(pool, team_id).await?;
    let team = team_json_with_context.to_db_struct();

    tracing::debug!("Upserting team with id {team_id} into lp database. Now returning it.");
    team.upsert(pool).await?;
    tracing::debug!("Upserted team with id {team_id} into lp database. Now returning it.");

    Ok(team)
}

#[tracing::instrument(skip(pool, nhl_web_api))]
pub async fn get_nhl_player(
    pool: &DbPool,
    nhl_web_api: &NhlWebApi,
    player_id: i32,
) -> Result<NhlPlayer, LPError> {
    let player_json_with_context: ItemParsedWithContext<NhlPlayerJson> = nhl_web_api
        .fetch_from_id::<NhlPlayerJson>(pool, player_id)
        .await?;
    let player: NhlPlayer = player_json_with_context.to_db_struct();
    player.upsert(pool).await?;
    Ok(player)
}

#[tracing::instrument(skip(pool, nhl_web_api))]
pub async fn get_nhl_game(
    pool: &DbPool,
    nhl_web_api: &NhlWebApi,
    game_id: i32,
) -> Result<NhlGame, LPError> {
    let game_json_with_context: ItemParsedWithContext<NhlGameJson> = nhl_web_api
        .fetch_from_id::<NhlGameJson>(pool, game_id)
        .await?;
    let game: NhlGame = game_json_with_context.to_db_struct();
    game.upsert(pool).await?;
    Ok(game)
}

#[tracing::instrument(skip(pool, nhl_web_api))]
pub async fn get_nhl_all_games_in_season(
    pool: &DbPool,
    nhl_web_api: &NhlWebApi,
    season: i32,
) -> Result<Vec<NhlGame>, LPError> {
    let number_of_games: Option<i32> =
        sqlx::query_scalar("SELECT total_regular_season_games FROM nhl_season WHERE id=$1")
            .bind(season)
            .fetch_optional(pool)
            .await?;
    let number_of_games = number_of_games.ok_or_else(|| {
        LPError::DatabaseCustom(format!("{season} season not found in lp database."))
    })?;

    let prefix: String = format!("{}02", season.to_string()[..4].to_string());

    let fetches = stream::iter(1..=number_of_games).map(|game_number| {
        let pool: sqlx::Pool<sqlx::Postgres> = pool.clone();
        let nhl_web_api: &NhlWebApi = nhl_web_api;
        let id_string: String = format!("{prefix}{game_number:04}");
        async move {
            let id: i32 = id_string.parse().map_err(|e| LPError::Parse(e))?;
            nhl_web_api.fetch_from_id::<NhlGameJson>(&pool, id).await
        }
    });
    let json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, LPError>> = fetches
        .buffer_unordered(CONFIG.upsert_concurrency)
        .collect()
        .await;

    let ok_json_results: Vec<ItemParsedWithContext<NhlGameJson>> =
        filter_and_log_results(json_results);
    let games: Vec<NhlGame> = ok_json_results
        .into_iter()
        .map(|game_json| game_json.to_db_struct())
        .collect();

    tracing::info!("Upserting {number_of_games} games from {season} NHL season into lp database.");
    let successes = NhlGame::upsert_all(games.clone(), pool).await?;
    tracing::info!(
        "Successfull upserted {successes}/{number_of_games} games from {season} NHL season into lp database. Now returning."
    );

    Ok(games)
}

// pub async fn upsert_all_with_logging<T>(items: &Vec<T>, pool: &DbPool)
// where
//     T: std::fmt::Debug + DbStruct + Persistable + Sync,
// {
//     stream::iter(items)
//         .map(|item| item.upsert(pool))
//         .buffer_unordered(CONFIG.upsert_concurrency)
//         .for_each(|result| async {
//             if let Err(e) = result {
//                 tracing::warn!("{e}");
//             }
//         })
//         .await;
// }
