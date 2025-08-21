use std::fmt::Debug;

use futures::{future::join_all, stream, stream::StreamExt};
use sqlx::postgres::PgQueryResult;

use super::super::primary_key::*;
use super::{
    api::{NhlApi, NhlStatsApi},
    models::*,
};
use crate::{
    common::{
        api::HasEndpoint,
        db::{DbContext, DbEntity, PrimaryKey},
        errors::LPError,
        models::{
            ApiCache, ItemParsedWithContext,
            traits::{DbStruct, HasTypeName, IntoDbStruct},
        },
        util::filter_results,
    },
    config::CONFIG,
};

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
        + Debug,
    T::DbStruct: Debug + DbStruct + DbEntity + Clone + Send + Sync,
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
pub async fn _get_nhl_team(
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
pub async fn _get_nhl_player(
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
pub async fn _get_nhl_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    id: i32,
) -> Result<NhlGame, LPError> {
    tracing::debug!("Checking lp database for NhlGame with id `{id}`");
    match NhlGame::fetch_from_db_by_key(db_context, &NhlPrimaryKey::Game(NhlGameKey { id: id }))
        .await
    {
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

pub async fn _get_nhl_all_games_in_season_by_season_id(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    id: i32,
) -> Result<Vec<NhlGame>, LPError> {
    let season: NhlSeason = NhlSeason::fetch_from_db_by_key(
        db_context,
        &NhlPrimaryKey::Season(NhlSeasonKey { id: id }),
    )
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

    let games: Vec<NhlGame> = json_struct_vector_into_db_structs(ok_json_results);
    let game_count = games.len();
    tracing::info!(
        "Parsed {game_count}/{number_of_games} games from {season_id} NHL season into lp database structs."
    );

    tracing::info!(
        "Upserting {number_of_games} games from {season_id} NHL season into lp database."
    );
    let upsert_results = upsert_all(games.clone(), db_context, nhl_api).await;
    let ok_upsert_results = filter_results(upsert_results);
    let ok_upsert_count = ok_upsert_results.len();
    tracing::info!(
        "Successfully upserted {ok_upsert_count}/{number_of_games} games from {season_id} NHL season into lp database."
    );

    Ok(games)
}

#[tracing::instrument(skip(db_context, nhl_api, game))]
pub async fn get_nhl_roster_spots_in_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game: &NhlGame,
) -> Result<Vec<NhlRosterSpot>, LPError> {
    let game_json: NhlGameJson = serde_json::from_value(game.raw_json.clone())?;

    let roster_spot_jsons: Vec<NhlRosterSpotJson> = game_json.roster_spots;
    let roster_spot_jsons_with_context: Vec<ItemParsedWithContext<NhlRosterSpotJson>> =
        roster_spot_jsons
            .into_iter()
            .map(|json| {
                let raw_json: serde_json::Value = match serde_json::to_value(&json) {
                    Ok(val) => val,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to turn NhlRosterSpotJson back to a json value. Using json for entire game instead: {e}"
                        );
                        game.raw_json.clone()
                    }
                };
                ItemParsedWithContext {
                    item: json,
                    context: GameNhlContext {
                        game_id: game.id,
                        endpoint: game.endpoint.clone(),
                        raw_json,
                    },
                }
            })
            .collect();

    let roster_spots: Vec<NhlRosterSpot> =
        json_struct_vector_into_db_structs(roster_spot_jsons_with_context);
    tracing::info!(
        "Parsed {} roster spots from NHL game with id {} into lp database structs.",
        roster_spots.len(),
        game.id
    );

    tracing::info!(
        "Upserting {} roster spots from NHL game with id {} into lp database.",
        roster_spots.len(),
        game.id
    );
    let upsert_results = upsert_all(roster_spots.clone(), db_context, nhl_api).await;
    let ok_upsert_results = filter_results(upsert_results);
    let ok_upsert_count = ok_upsert_results.len();
    tracing::info!(
        "Upserted {}/{} roster spots from NHL game with id {} into lp database.",
        ok_upsert_count,
        roster_spots.len(),
        game.id
    );

    Ok(roster_spots)
}

#[tracing::instrument(skip(db_context, nhl_api, game))]
pub async fn get_nhl_plays_in_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game: &NhlGame,
) -> Result<Vec<NhlPlay>, LPError> {
    let game_json: NhlGameJson = serde_json::from_value(game.raw_json.clone())?;

    let play_jsons: Vec<NhlPlayJson> = game_json.plays;
    let play_jsons_with_context: Vec<ItemParsedWithContext<NhlPlayJson>> = play_jsons
            .into_iter()
            .map(|json| {
                let raw_json: serde_json::Value = match serde_json::to_value(&json) {
                    Ok(val) => val,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to turn NhlPlayJson back to a json value. Using json for entire game instead: {e}"
                        );
                        game.raw_json.clone()
                    }
                };
                ItemParsedWithContext {
                    item: json,
                    context: GameNhlContext {
                        game_id: game.id,
                        endpoint: game.endpoint.clone(),
                        raw_json,
                    },
                }
            })
            .collect();

    let plays: Vec<NhlPlay> = json_struct_vector_into_db_structs(play_jsons_with_context);
    tracing::info!(
        "Parsed {} plays from NHL game with id {} into lp database structs.",
        plays.len(),
        game.id
    );

    tracing::info!(
        "Upserting {} plays from NHL game with id {} into lp database.",
        plays.len(),
        game.id
    );
    let upsert_results = upsert_all(plays.clone(), db_context, nhl_api).await;
    let ok_upsert_results = filter_results(upsert_results);
    let ok_upsert_count = ok_upsert_results.len();
    tracing::info!(
        "Upserted {}/{} plays from NHL game with id {} into lp database.",
        ok_upsert_count,
        plays.len(),
        game.id
    );

    Ok(plays)
}

#[tracing::instrument(skip(db_context, nhl_api, game))]
pub async fn get_nhl_shifts_in_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game: &NhlGame,
) -> Result<Vec<NhlShift>, LPError> {
    let shift_array: Vec<ItemParsedWithContext<NhlShiftJson>> =
        nhl_api.get_nhl_shifts_in_game(db_context, game.id).await?;
    let shifts: Vec<NhlShift> = json_struct_vector_into_db_structs(shift_array);

    tracing::info!(
        "Parsed {} shifts from NHL game with id {} into lp database structs.",
        shifts.len(),
        game.id
    );

    tracing::info!(
        "Upserting {} shifts from NHL game with id {} into lp database.",
        shifts.len(),
        game.id
    );
    let upsert_results = upsert_all(shifts.clone(), db_context, nhl_api).await;
    let ok_upsert_results = filter_results(upsert_results);
    let ok_upsert_count = ok_upsert_results.len();
    tracing::info!(
        "Upserted {}/{} plays from NHL game with id {} into lp database.",
        ok_upsert_count,
        shifts.len(),
        game.id
    );

    Ok(shifts)
}

pub fn json_struct_vector_into_db_structs<J>(
    json_structs: Vec<ItemParsedWithContext<J>>,
) -> Vec<J::DbStruct>
where
    J: IntoDbStruct,
    J::DbStruct: DbStruct,
{
    with_progress_bar!(json_structs.len(), |pb| {
        json_structs
            .into_iter()
            .map(|game_json| game_json.to_db_struct())
            .inspect(|_| pb.inc(1))
            .collect()
    })
}

#[tracing::instrument(skip(items, db_context, api))]
pub async fn upsert_all<T: DbEntity + DbStruct + HasTypeName>(
    items: Vec<T>,
    db_context: &DbContext,
    api: &<<T as DbEntity>::Pk as PrimaryKey>::Api,
) -> Vec<Result<Option<PgQueryResult>, LPError>> {
    join_all(
        items
            .iter()
            .map(|game| game.fix_relationships_and_upsert(db_context, api)),
    )
    .await
}

pub async fn warm_nhl_key_cache(db_context: &DbContext) -> Result<(), LPError> {
    tracing::info!("Warming NHL database key cache.");
    ApiCache::warm_key_cache(db_context).await?;
    NhlSeason::warm_key_cache(db_context).await?;
    NhlFranchise::warm_key_cache(db_context).await?;
    NhlTeam::warm_key_cache(db_context).await?;
    NhlPlayer::warm_key_cache(db_context).await?;
    NhlGame::warm_key_cache(db_context).await?;
    NhlRosterSpot::warm_key_cache(db_context).await?;
    NhlPlay::warm_key_cache(db_context).await?;
    NhlPlayoffSeries::warm_key_cache(db_context).await?;
    tracing::info!("Warmed NHL database key cache.");
    Ok(())
}
