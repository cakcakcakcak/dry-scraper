//! NHL data orchestration layer.
//!
//! ## Foreign key dependency management
//!
//! This module fetches and upserts NHL entities in the correct dependency order.
//! Each `get_nhl_*` function assumes its dependencies are already cached.
//!
//! **Dependency graph (fetch in this order):**
//! 1. Franchises (no dependencies)
//! 2. Seasons (no dependencies)
//! 3. Teams (depends on: franchises)
//! 4. Players (depends on: teams - optional current_team_id)
//! 5. Games (depends on: teams, seasons)
//! 6. Playoff bracket series (depends on: seasons, teams)
//! 7. Playoff series (depends on: seasons, teams, playoff bracket series)
//! 8. Playoff series games (depends on: playoff series)
//! 9. Roster spots (depends on: games, teams, players)
//! 10. Plays (depends on: games)
//! 11. Shifts (depends on: games, teams, players)
//!
//! **Usage pattern:**
//! ```rust,ignore
//! // At startup, warm the key cache from existing DB records
//! warm_nhl_key_cache(&app_context, &db_context).await?;
//!
//! // Then fetch in dependency order (each function is idempotent)
//! get_nhl_franchises(&app_context, &db_context, &nhl_api).await?;
//! get_nhl_teams(&app_context, &db_context, &nhl_api).await?;
//! get_nhl_games(&app_context, &db_context, &nhl_api).await?;
//! ```
//!
//! All `get_nhl_*` functions check the key cache before upserting, so calling
//! them multiple times is safe and cheap.

use futures::stream::{self, StreamExt};
use sqlx::postgres::PgQueryResult;

use super::{api::NhlApi, models::*};

use crate::{
    common::{
        app_context::AppContext,
        db::{DbContext, DbEntity, DbEntityVecExt},
        errors::DSError,
        models::{
            traits::{DbStruct, HasTypeName, IntoDbStruct},
            ApiCache, ItemParsedWithContext, ItemParsedWithContextVecExt,
        },
        util::track_and_filter_errors,
    },
    CONFIG,
};

pub async fn get_resource<J, D, Fut>(
    app_context: &AppContext,
    db_context: &DbContext,
    fetch_fn: Fut,
) -> Result<Vec<D>, DSError>
where
    J: IntoDbStruct<DbStruct = D>,
    D: DbStruct + DbEntity + HasTypeName,
    Fut: std::future::Future<Output = Result<Vec<ItemParsedWithContext<J>>, DSError>>,
{
    let j_name: &'static str = J::type_name();
    let d_name: &'static str = D::type_name();

    tracing::debug!("Fetching all available `{j_name}`s from NHL API.");
    let json_structs: Vec<ItemParsedWithContext<J>> = fetch_fn.await?;
    let json_struct_count: usize = json_structs.len();
    tracing::debug!("Successfully fetched {json_struct_count} `{j_name}`s from NHL API",);

    let db_structs: Vec<D> = json_structs.into_db_structs(
        app_context,
        &format!("Parsing `{j_name}`s into `{d_name}`s."),
    );
    let db_struct_count: usize = db_structs.len();
    tracing::debug!("Parsed {db_struct_count}/{json_struct_count} `{j_name}`s into `{d_name}`s.",);

    tracing::debug!("Upserting {db_struct_count} `{d_name}`s into lp database.",);
    let upsert_results: Vec<Option<PgQueryResult>> =
        db_structs.upsert_all(app_context, db_context).await;
    let ok_upsert_count: usize = upsert_results.len();
    tracing::debug!("Upserted {ok_upsert_count}/{db_struct_count} `{d_name}`s into lp database.");

    Ok(db_structs)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_seasons(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlSeason>, DSError> {
    get_resource(app_context, db_context, nhl_api.list_seasons(db_context)).await
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_franchises(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlFranchise>, DSError> {
    get_resource(app_context, db_context, nhl_api.list_franchises(db_context)).await
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_teams(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlTeam>, DSError> {
    get_resource(app_context, db_context, nhl_api.list_teams(db_context)).await
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_shifts_in_game(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_id: i32,
) -> Result<Vec<NhlShift>, DSError> {
    get_resource(
        app_context,
        db_context,
        nhl_api.list_shifts_for_game(db_context, game_id),
    )
    .await
}

#[tracing::instrument(skip(app_context, db_context, nhl_api, season))]
pub async fn get_nhl_everything_in_season(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season: &NhlSeason,
) -> Result<(), DSError> {
    let mut games: Vec<NhlGame> =
        get_nhl_all_games_in_season(app_context, db_context, nhl_api, season).await?;

    let playoff_bracket_series: Vec<NhlPlayoffBracketSeries> =
        get_nhl_playoff_bracket_series(app_context, db_context, nhl_api, season).await?;

    for bracket_series in playoff_bracket_series {
        let series: NhlPlayoffSeries =
            get_nhl_playoff_series(app_context, db_context, nhl_api, &bracket_series).await?;
        let mut playoff_games: Vec<NhlGame> =
            get_nhl_games_in_playoff_series(app_context, db_context, nhl_api, &series).await?;
        games.append(&mut playoff_games);
    }

    for game in games {
        let (plays_res, roster_res, shifts_res) = tokio::join!(
            get_nhl_plays_in_game(app_context, db_context, &game),
            get_nhl_roster_spots_in_game(app_context, db_context, &game),
            get_nhl_shifts_in_game(app_context, db_context, nhl_api, game.id),
        );
        plays_res?;
        roster_res?;
        shifts_res?;
    }

    Ok(())
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_all_games_in_season(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season: &NhlSeason,
) -> Result<Vec<NhlGame>, DSError> {
    let number_of_games: i32 = season.total_regular_season_games;
    let season_id: String = season.id.to_string();

    let prefix: String = format!("{}02", &season_id[..4]);
    let game_ids: Vec<i32> = (1..=number_of_games)
        .map(|game_number| {
            let id_string: String = format!("{prefix}{game_number:04}");
            id_string.parse::<i32>().unwrap()
        })
        .collect();

    tracing::info!("Fetching {number_of_games} `NhlGameJson`s from NHL API or cache.");
    let json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, DSError>> = nhl_api
        .get_many_games(app_context, db_context, game_ids)
        .await;
    let ok_json_results: Vec<ItemParsedWithContext<NhlGameJson>> =
        track_and_filter_errors(json_results, db_context).await;
    let ok_json_result_count = ok_json_results.len();
    tracing::info!(
        "Successfully fetched {ok_json_result_count}/{number_of_games} games from NHL API or cache."
    );

    let games: Vec<NhlGame> = ok_json_results.into_db_structs(
        app_context,
        &format!("Parsing `NhlGameJson`s from {season_id} season."),
    );
    let game_count = games.len();
    tracing::info!("Parsed {game_count}/{number_of_games} games into lp database structs.");

    tracing::info!("Upserting {number_of_games} games  into lp database.");
    let upsert_results: Vec<Option<PgQueryResult>> =
        games.upsert_all(app_context, db_context).await;
    let ok_upsert_count: usize = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{number_of_games} games into lp database.");

    Ok(games)
}

#[tracing::instrument(skip(app_context, db_context))]
pub async fn get_nhl_roster_spots_in_game(
    app_context: &AppContext,
    db_context: &DbContext,
    game: &NhlGame,
) -> Result<Vec<NhlRosterSpot>, DSError> {
    let game_id: i32 = game.id;
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
                            "Failed to turn `NhlRosterSpotJson` back to a json value. Using json for entire game instead: {e}"
                        );
                        game.raw_json.clone()
                    }
                };
                ItemParsedWithContext {
                    item: json,
                    context: NhlGameContext {
                        game_id: game.id,
                        endpoint: game.endpoint.clone(),
                        raw_json,
                    },
                }
            })
            .collect();

    let roster_spots: Vec<NhlRosterSpot> = roster_spot_jsons_with_context.into_db_structs(
        app_context,
        &format!("Parsing `NhlRosterSpotJson`s from {game_id}."),
    );
    let roster_spot_count = roster_spots.len();
    tracing::info!("Parsed {roster_spot_count} roster spots into lp database structs.");

    tracing::info!("Upserting {roster_spot_count} roster spots lp database.",);
    let upsert_results = roster_spots.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{roster_spot_count} roster spots into lp database.",);

    Ok(roster_spots)
}

#[tracing::instrument(skip(app_context, db_context))]
pub async fn get_nhl_plays_in_game(
    app_context: &AppContext,
    db_context: &DbContext,
    game: &NhlGame,
) -> Result<Vec<NhlPlay>, DSError> {
    let game_id: i32 = game.id;
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
                    context: NhlGameContext {
                        game_id: game.id,
                        endpoint: game.endpoint.clone(),
                        raw_json,
                    },
                }
            })
            .collect();

    let plays: Vec<NhlPlay> = play_jsons_with_context.into_db_structs(
        app_context,
        &format!("Parsing `NhlPlayJson`s from {game_id}."),
    );
    let play_count = plays.len();
    tracing::info!("Parsed {play_count} plays into lp database structs.",);

    tracing::info!("Upserting {play_count} plays into lp database.",);
    let upsert_results = plays.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{play_count} plays into lp database.",);

    Ok(plays)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_playoff_bracket_series(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season: &NhlSeason,
) -> Result<Vec<NhlPlayoffBracketSeries>, DSError> {
    let season_id: i32 = season.id;
    let year_id: i32 = season_id.to_string()[4..].parse::<i32>().unwrap();
    get_resource(
        app_context,
        db_context,
        nhl_api.list_playoff_series_for_year(db_context, year_id),
    )
    .await
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_playoff_series(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    bracket_series: &NhlPlayoffBracketSeries,
) -> Result<NhlPlayoffSeries, DSError> {
    let season_id: i32 = bracket_series.season_id;
    let series_letter: &str = &bracket_series.series_letter;
    let series_json: ItemParsedWithContext<NhlPlayoffSeriesJson> = nhl_api
        .get_playoff_series(db_context, season_id, series_letter)
        .await?;

    let series: NhlPlayoffSeries = series_json.clone().into_db_struct();
    tracing::info!("Parsed playoff series into lp database struct.");

    tracing::debug!(
        "playoff series upsert deferred to orchestrator (use resolve_foreign_keys in step 1.4c)"
    );

    let series_game_jsons: Vec<ItemParsedWithContext<NhlPlayoffSeriesGameJson>> = series_json
        .item
        .games
        .iter()
        .map(|game_json| {
                let raw_json: serde_json::Value = match serde_json::to_value(game_json) {
                    Ok(val) => val,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to turn `NhlPlayoffSeriesGameJson` back to a json value. Using json for entire series instead: {e}"
                        );
                        series.raw_json.clone()
                    }
                };
                ItemParsedWithContext::<NhlPlayoffSeriesGameJson> {
            item: game_json.clone(),
            context: NhlPlayoffSeriesContext {
                series_letter: series_letter.to_string(),
                raw_json,
                endpoint: series.endpoint.clone(),
            },
        }})
        .collect();
    let series_games: Vec<NhlPlayoffSeriesGame> = series_game_jsons.into_db_structs(
        app_context,
        &format!(
        "Parsing `NhlPlayoffSeriesGameJson`s from Series {series_letter} from {season_id} season."
    ),
    );
    let series_game_count = series_games.len();
    tracing::info!("Parsed {series_game_count} games into lp database structs.",);

    tracing::info!("Upserting {series_game_count} games  into lp database.",);
    let upsert_results = series_games.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{series_game_count} games into lp database.",);

    Ok(series)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_games_in_playoff_series(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    series: &NhlPlayoffSeries,
) -> Result<Vec<NhlGame>, DSError> {
    let game_ids: Vec<i32> = series.game_ids.to_vec();
    let number_of_games: usize = game_ids.len();
    let series_letter: &str = &series.series_letter;
    let season_id: i32 = series.season_id;

    tracing::info!("Fetching {number_of_games} game play-by-play reports from NHL API or cache.");
    let game_json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, DSError>> = nhl_api
        .get_many_games(app_context, db_context, game_ids)
        .await;
    let game_jsons: Vec<ItemParsedWithContext<NhlGameJson>> =
        track_and_filter_errors(game_json_results, db_context).await;
    let ok_game_json_count: usize = game_jsons.len();
    tracing::info!(
        "Fetched {ok_game_json_count}/{number_of_games} game play-by-play reports from NHL API or cache."
    );
    let games: Vec<NhlGame> = game_jsons.into_db_structs(
        app_context,
        &format!(
        "Parsing `NhlPlayoffSeriesGameJson`s from Series {series_letter} from {season_id} season."
    ),
    );
    let ok_game_count: usize = games.len();
    tracing::info!(
        "Parsed {ok_game_count}/{number_of_games} game play-by-play reports lp database structs."
    );

    tracing::info!("Upserting {ok_game_count} game play-by-play reports into lp database.");
    let upsert_results = games.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!(
        "Upserted {ok_upsert_count}/{number_of_games} game play-by-play reports into lp database."
    );

    Ok(games)
}

pub async fn warm_nhl_key_cache(
    _app_context: &AppContext,
    db_context: &DbContext,
) -> Result<(), DSError> {
    let db_context = &db_context.clone();

    tracing::info!("Warming NHL database key cache.");
    let cache_warmers = vec![
        ApiCache::warm_key_cache(db_context),
        NhlSeason::warm_key_cache(db_context),
        NhlFranchise::warm_key_cache(db_context),
        NhlTeam::warm_key_cache(db_context),
        NhlPlayer::warm_key_cache(db_context),
        NhlGame::warm_key_cache(db_context),
        NhlRosterSpot::warm_key_cache(db_context),
        NhlPlay::warm_key_cache(db_context),
        NhlShift::warm_key_cache(db_context),
        NhlPlayoffBracketSeries::warm_key_cache(db_context),
        NhlPlayoffSeries::warm_key_cache(db_context),
        NhlPlayoffSeriesGame::warm_key_cache(db_context),
    ];
    stream::iter(cache_warmers)
        .map(|fut| fut)
        .buffer_unordered(CONFIG.db_concurrency_limit)
        .collect::<Vec<_>>()
        .await;
    tracing::info!("Warmed NHL database key cache.");
    Ok(())
}
