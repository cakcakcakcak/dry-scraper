use futures::stream::{self, StreamExt};

use crate::api::api_common::HasEndpoint;
use crate::api::nhl::nhl_stats_api::NhlStatsApi;
use crate::api::nhl::nhl_web_api::NhlWebApi;
use crate::config::CONFIG;
use crate::db::{DbPool, Persistable};
use crate::lp_error::LPError;
use crate::models::ItemParsedWithContext;
use crate::models::nhl::{
    NhlFranchise, NhlFranchiseJson, NhlGame, NhlGameJson, NhlPlayer, NhlPlayerJson, NhlSeason,
    NhlSeasonJson, NhlTeam, NhlTeamJson, DefaultNhlContext
};
use crate::models::traits::{DbStruct, HasTypeName, IntoDbStruct};
use crate::util::filter_results;

use crate::with_progress_bar;

#[tracing::instrument(skip(pool, nhl_stats_api))]
async fn get_nhl_api_data_array<T>(
    pool: &DbPool,
    nhl_stats_api: &NhlStatsApi,
) -> Result<Vec<T::DbStruct>, LPError>
where
    T: serde::de::DeserializeOwned
        + HasEndpoint
        + IntoDbStruct<Context = DefaultNhlContext>
        + std::fmt::Debug
        + HasTypeName,
    T::DbStruct: std::fmt::Debug + DbStruct + Persistable + HasTypeName + Clone + Send + Sync,
{
    let data_array: Vec<ItemParsedWithContext<T>> =
        nhl_stats_api.fetch_nhl_api_data_array::<T>(&pool).await?;

    let db_records: Vec<T::DbStruct> = data_array
        .into_iter()
        .map(|record: ItemParsedWithContext<T>| record.to_db_struct())
        .collect();
    let count = db_records.len();

    // VERIFY_RELATIONSHIPS
    // CORRECT ANY MISSING PIECES
    tracing::info!(
        "Upserting {count} `{}`s into lp database.",
        T::DbStruct::type_name()
    );
    T::DbStruct::upsert_all(db_records.clone(), pool).await?;
    tracing::info!(
        "Upserted {count} `{}`s into lp database.",
        T::DbStruct::type_name()
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

    tracing::debug!("Upserting team with id {team_id} into lp database.");
    team.upsert(pool).await?;
    tracing::debug!("Upserted team with id {team_id} into lp database.");

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

pub async fn get_nhl_all_games_in_season_by_id(
    pool: &DbPool,
    nhl_web_api: &NhlWebApi,
    season_id: i32,
) -> Result<Vec<NhlGame>, LPError> {
    let season = NhlSeason::try_db(pool, season_id).await?.ok_or_else(|| {
        LPError::DatabaseCustom(format!(
            "Season {season_id} not found in database. Please fetch seasons first."
        ))
    })?;

    get_nhl_all_games_in_season(pool, nhl_web_api, &season).await
}

#[tracing::instrument(skip(pool, nhl_web_api))]
pub async fn get_nhl_all_games_in_season(
    pool: &DbPool,
    nhl_web_api: &NhlWebApi,
    season: &NhlSeason,
) -> Result<Vec<NhlGame>, LPError> {
    let number_of_games: i32 = season.total_regular_season_games;
    let season_id: String = season.id.to_string();

    let prefix: String = format!("{}02", season_id[..4].to_string());

    tracing::info!(
        "Fetching {number_of_games} games from {season_id} NHL season from API or cache."
    );
    let json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, LPError>> =
        with_progress_bar!(number_of_games, |pb| {
            let fetches = stream::iter(1..=number_of_games).map(|game_number| {
                let pool: sqlx::Pool<sqlx::Postgres> = pool.clone();
                let nhl_web_api: &NhlWebApi = nhl_web_api;
                let id_string: String = format!("{prefix}{game_number:04}");
                async move {
                    let id: i32 = id_string.parse().map_err(|e| {
                        tracing::warn!("Failed to parse {id_string} into `i32`: {e:?}");
                        LPError::Parse(e)
                    })?;
                    nhl_web_api.fetch_from_id::<NhlGameJson>(&pool, id).await
                }
            });
            fetches
                .buffer_unordered(CONFIG.upsert_concurrency)
                .inspect(|_| pb.inc(1))
                .collect()
                .await
        });
    let ok_json_results: Vec<ItemParsedWithContext<NhlGameJson>> = filter_results(json_results);
    let ok_json_result_count = ok_json_results.len();
    tracing::info!(
        "Successfully fetched {ok_json_result_count}/{number_of_games} games from {season_id} NHL season."
    );

    tracing::info!(
        "Parsing {ok_json_result_count} games from {season_id} NHL season into lp database structs."
    );
    let games: Vec<NhlGame> = with_progress_bar!(ok_json_result_count, |pb| {
        ok_json_results
            .into_iter()
            .map(|game_json| game_json.to_db_struct())
            .inspect(|_| pb.inc(1))
            .collect()
    });
    let game_count = games.len();
    tracing::info!(
        "Successfully parsed {game_count}/{number_of_games} games from {season_id} NHL season into lp database."
    );

    tracing::info!(
        "Upserting {number_of_games} games from {season_id} NHL season into lp database."
    );
    let successes = NhlGame::upsert_all(games.clone(), pool).await?;
    tracing::info!(
        "Successfully upserted {successes}/{number_of_games} games from {season_id} NHL season into lp database."
    );

    Ok(games)
}
