use futures::future::join_all;
use futures::stream::{self, StreamExt};

use crate::api::api_common::HasEndpoint;
use crate::api::nhl::{NhlApi, NhlStatsApi, NhlWebApi};
use crate::config::CONFIG;
use crate::db::{DbContext, Persistable, PrimaryKey};
use crate::lp_error::LPError;
use crate::models::ItemParsedWithContext;
use crate::models::nhl::{
    DefaultNhlContext, NhlFranchise, NhlFranchiseJson, NhlGame, NhlGameJson, NhlGameKey, NhlPlayer,
    NhlPlayerJson, NhlRosterSpot, NhlRosterSpotJson, NhlSeason, NhlSeasonJson, NhlSeasonKey,
    NhlTeam, NhlTeamJson,
};
use crate::models::traits::{DbStruct, HasTypeName, IntoDbStruct};
use crate::util::filter_results;

use crate::with_progress_bar;

#[tracing::instrument(skip(db_context, nhl_api))]
async fn get_nhl_api_data_array<T>(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<T::DbStruct>, LPError>
where
    T: serde::de::DeserializeOwned
        + HasEndpoint<Api = NhlStatsApi>
        + IntoDbStruct<Context = DefaultNhlContext>
        + std::fmt::Debug
        + HasTypeName,
    T::DbStruct: std::fmt::Debug + DbStruct + Persistable + HasTypeName + Clone + Send + Sync,
{
    let data_array: Vec<ItemParsedWithContext<T>> =
        nhl_api.fetch_nhl_api_data_array::<T>(db_context).await?;

    let db_records: Vec<T::DbStruct> = data_array
        .into_iter()
        .map(|record: ItemParsedWithContext<T>| record.to_db_struct())
        .collect();
    let count = db_records.len();

    tracing::info!(
        "Upserting {count} `{}`s into lp database.",
        T::DbStruct::type_name()
    );
    join_all(db_records.iter().map(|record| record.upsert(db_context))).await;
    tracing::info!(
        "Upserted {count} `{}`s into lp database.",
        T::DbStruct::type_name()
    );

    Ok(db_records)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_seasons(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlSeason>, LPError> {
    let db_seasons = get_nhl_api_data_array::<NhlSeasonJson>(db_context, &nhl_api).await?;

    Ok(db_seasons)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_franchises(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlFranchise>, LPError> {
    let db_franchises = get_nhl_api_data_array::<NhlFranchiseJson>(&db_context, &nhl_api).await?;

    Ok(db_franchises)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_teams(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlTeam>, LPError> {
    let db_teams = get_nhl_api_data_array::<NhlTeamJson>(&db_context, &nhl_api).await?;

    Ok(db_teams)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_team(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    team_id: i32,
) -> Result<NhlTeam, LPError> {
    let team_json_with_context: ItemParsedWithContext<NhlTeamJson> =
        nhl_api.get_nhl_team(db_context, team_id).await?;
    let team = team_json_with_context.to_db_struct();

    tracing::debug!("Upserting team with id {team_id} into lp database.");
    team.upsert(db_context).await?;
    tracing::debug!("Upserted team with id {team_id} into lp database.");

    Ok(team)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_player(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    player_id: i32,
) -> Result<NhlPlayer, LPError> {
    let player_json_with_context: ItemParsedWithContext<NhlPlayerJson> = nhl_api
        .fetch_by_id::<NhlPlayerJson>(db_context, player_id)
        .await?;
    let player: NhlPlayer = player_json_with_context.to_db_struct();
    player.upsert(db_context).await?;
    Ok(player)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    id: i32,
) -> Result<NhlGame, LPError> {
    tracing::debug!("Checking lp database for NhlGame with id `{id}`");
    match NhlGame::fetch_from_db(db_context, &NhlGameKey { id }).await {
        Ok(Some(game)) => {
            return Ok(game);
        }
        Ok(None) => (),
        Err(e) => {
            tracing::warn!(error=%e, "Encountered an error while trying lp database.");
        }
    }
    let game_json_with_context: ItemParsedWithContext<NhlGameJson> =
        nhl_api.fetch_by_id::<NhlGameJson>(db_context, id).await?;
    let game: NhlGame = game_json_with_context.to_db_struct();
    game.upsert(db_context).await?;
    Ok(game)
}

pub async fn get_nhl_all_games_in_season_by_season_id(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    id: i32,
) -> Result<Vec<NhlGame>, LPError> {
    let season: NhlSeason = NhlSeason::fetch_from_db(db_context, &NhlSeasonKey { id })
        .await?
        .ok_or_else(|| {
            LPError::DatabaseCustom(format!(
                "Season {id} not found in database. Please fetch seasons first."
            ))
        })?;

    get_nhl_all_games_in_season(db_context, nhl_api, &season).await
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_all_games_in_season(
    db_context: &DbContext,
    nhl_api: &NhlApi,
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
                let id_string: String = format!("{prefix}{game_number:04}");
                async move {
                    let id: i32 = id_string.parse().map_err(|e| {
                        tracing::warn!("Failed to parse {id_string} into `i32`: {e:?}");
                        LPError::Parse(e)
                    })?;
                    nhl_api.fetch_by_id::<NhlGameJson>(db_context, id).await
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
        "Successfully parsed {game_count}/{number_of_games} games from {season_id} NHL season into lp database structs."
    );

    tracing::info!(
        "Upserting {number_of_games} games from {season_id} NHL season into lp database."
    );
    let upsert_results: Vec<Result<sqlx::postgres::PgQueryResult, LPError>> =
        join_all(games.iter().map(|game| game.upsert(db_context))).await;
    let ok_upsert_results = filter_results(upsert_results);
    let ok_upsert_count = ok_upsert_results.len();
    tracing::info!(
        "Successfully upserted {ok_upsert_count}/{number_of_games} games from {season_id} NHL season into lp database."
    );

    Ok(games)
}
