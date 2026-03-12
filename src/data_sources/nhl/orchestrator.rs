use futures::stream::{self, StreamExt};
use sqlx::postgres::PgQueryResult;

use super::{api::NhlApi, models::*};

use crate::{
    common::{
        app_context::AppContext,
        db::{
            all_foreign_keys_cached, find_missing_foreign_keys, CacheKey, DbContext, DbEntity,
            DbEntityVecExt,
        },
        errors::DSError,
        models::{
            traits::{DbStruct, HasTypeName, IntoDbStruct},
            ApiCache, DataSourceError, ItemParsedWithContext, ItemParsedWithContextVecExt,
        },
    },
    CONFIG,
};

/// Ensure all foreign keys referenced by entities exist in the database.
///
/// This checks the key cache for missing FKs and logs warnings if any are found.
/// It does NOT fetch missing entities - the caller must ensure proper ordering
/// (e.g., fetch teams before games that reference them).
///
/// Returns the list of missing foreign keys for diagnostic purposes.
async fn ensure_foreign_keys_exist<T>(entities: &[T], db_context: &DbContext) -> Vec<CacheKey>
where
    T: DbEntity + HasTypeName,
{
    if all_foreign_keys_cached(entities, db_context) {
        return Vec::new();
    }

    let missing_fks = find_missing_foreign_keys(entities, db_context);
    let type_name = T::type_name();

    tracing::warn!(
        "{} `{}` entities reference {} missing foreign keys. Some upserts may fail if FKs don't exist.",
        entities.len(),
        type_name,
        missing_fks.len()
    );

    if tracing::enabled!(tracing::Level::DEBUG) {
        let sample_size = missing_fks.len().min(10);
        tracing::debug!(
            "Missing FK sample ({}/{}): {:#?}",
            sample_size,
            missing_fks.len(),
            &missing_fks.iter().take(sample_size).collect::<Vec<_>>()
        );
    }

    missing_fks
}

pub async fn get_resource<J, D, Fut>(
    app_context: &AppContext,
    db_context: &DbContext,
    fetch_fn: Fut,
) -> Result<Vec<D>, DSError>
where
    J: IntoDbStruct<DbStruct = D>,
    D: DbStruct + DbEntity + HasTypeName,
    Fut: std::future::Future<
        Output = Result<Vec<Result<ItemParsedWithContext<J>, DSError>>, DSError>,
    >,
{
    let j_name: &'static str = J::type_name();
    let d_name: &'static str = D::type_name();

    tracing::debug!(type_name = j_name, "Fetching from NHL API");
    let json_struct_results: Vec<Result<ItemParsedWithContext<J>, DSError>> = fetch_fn.await?;
    let total_count = json_struct_results.len();

    // Partition successes and failures
    let mut json_structs = Vec::new();
    let mut parse_errors = Vec::new();
    for result in json_struct_results {
        match result {
            Ok(item) => json_structs.push(item),
            Err(e) => parse_errors.push(e),
        }
    }

    let json_struct_count = json_structs.len();
    if !parse_errors.is_empty() {
        tracing::warn!(
            type_name = j_name,
            successful = json_struct_count,
            failed = parse_errors.len(),
            total = total_count,
            "Parse errors occurred during fetch"
        );
    }

    // Log parse errors (fire-and-forget)
    if !parse_errors.is_empty() {
        for error in parse_errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let db_structs: Vec<D> = json_structs.into_db_structs(
        app_context,
        &format!("Parsing `{j_name}`s into `{d_name}`s."),
    );
    let db_struct_count: usize = db_structs.len();

    // Check for missing foreign keys before upserting
    ensure_foreign_keys_exist(&db_structs, db_context).await;

    let upsert_results: Vec<Option<PgQueryResult>> =
        db_structs.upsert_all(app_context, db_context).await;
    let ok_upsert_count: usize = upsert_results.len();
    tracing::debug!(
        type_name = d_name,
        upserted = ok_upsert_count,
        total = db_struct_count,
        "Upsert complete"
    );

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
            id_string
                .parse::<i32>()
                .expect("Game ID should always be valid i32")
        })
        .collect();

    tracing::info!(
        season_id = %season_id,
        game_count = number_of_games,
        "Fetching regular season games"
    );
    let json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, DSError>> = nhl_api
        .get_many_games(app_context, db_context, game_ids)
        .await;

    // Partition successes and failures
    let mut ok_json_results = Vec::new();
    let mut parse_errors = Vec::new();
    for result in json_results {
        match result {
            Ok(item) => ok_json_results.push(item),
            Err(e) => parse_errors.push(e),
        }
    }

    let ok_json_result_count = ok_json_results.len();
    if !parse_errors.is_empty() {
        tracing::warn!(
            season_id = %season_id,
            successful = ok_json_result_count,
            failed = parse_errors.len(),
            total = number_of_games,
            "Parse errors occurred while fetching games"
        );
    }

    // Log parse errors (fire-and-forget)
    if !parse_errors.is_empty() {
        for error in parse_errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let games: Vec<NhlGame> = ok_json_results.into_db_structs(
        app_context,
        &format!("Parsing `NhlGameJson`s from {season_id} season."),
    );

    // Check for missing foreign keys (teams, season) before upserting
    ensure_foreign_keys_exist(&games, db_context).await;

    let upsert_results: Vec<Option<PgQueryResult>> =
        games.upsert_all(app_context, db_context).await;
    let ok_upsert_count: usize = upsert_results.len();
    tracing::info!(
        season_id = %season_id,
        upserted = ok_upsert_count,
        "Regular season games upserted"
    );

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

    // Check for missing foreign keys (game, players, teams) before upserting
    ensure_foreign_keys_exist(&roster_spots, db_context).await;

    let upsert_results = roster_spots.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::debug!(
        game_id = game.id,
        upserted = ok_upsert_count,
        total = roster_spot_count,
        "Roster spots upserted"
    );

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

    // Check for missing foreign keys (game) before upserting
    ensure_foreign_keys_exist(&plays, db_context).await;

    let upsert_results = plays.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::debug!(
        game_id = game.id,
        upserted = ok_upsert_count,
        total = play_count,
        "Plays upserted"
    );

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
    let year_id: i32 = season_id.to_string()[4..]
        .parse::<i32>()
        .map_err(DSError::Parse)?;
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

    // Check for missing foreign keys (season, teams, bracket series) before upserting
    ensure_foreign_keys_exist(std::slice::from_ref(&series), db_context).await;

    // Upsert the playoff series now that FKs are checked
    let _upsert_result = series.upsert(db_context).await?;

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

    // Check for missing foreign keys (season, teams, series, bracket series) before upserting
    ensure_foreign_keys_exist(&series_games, db_context).await;

    let upsert_results = series_games.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::debug!(
        season_id,
        series_letter,
        upserted = ok_upsert_count,
        "Playoff series games upserted"
    );

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

    tracing::info!(
        season_id,
        series_letter,
        game_count = number_of_games,
        "Fetching playoff game play-by-play"
    );
    let game_json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, DSError>> = nhl_api
        .get_many_games(app_context, db_context, game_ids)
        .await;

    // Partition successes and failures
    let mut game_jsons = Vec::new();
    let mut parse_errors = Vec::new();
    for result in game_json_results {
        match result {
            Ok(item) => game_jsons.push(item),
            Err(e) => parse_errors.push(e),
        }
    }

    let ok_game_json_count = game_jsons.len();
    if !parse_errors.is_empty() {
        tracing::warn!(
            season_id,
            series_letter,
            successful = ok_game_json_count,
            failed = parse_errors.len(),
            total = number_of_games,
            "Parse errors occurred while fetching playoff games"
        );
    }

    // Log parse errors (fire-and-forget)
    if !parse_errors.is_empty() {
        for error in parse_errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }
    let games: Vec<NhlGame> = game_jsons.into_db_structs(
        app_context,
        &format!(
        "Parsing `NhlPlayoffSeriesGameJson`s from Series {series_letter} from {season_id} season."
    ),
    );

    // Check for missing foreign keys (season, teams) before upserting
    ensure_foreign_keys_exist(&games, db_context).await;

    let upsert_results = games.upsert_all(app_context, db_context).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!(
        season_id,
        series_letter,
        upserted = ok_upsert_count,
        "Playoff game play-by-play upserted"
    );

    Ok(games)
}

pub async fn warm_nhl_key_cache(
    _app_context: &AppContext,
    db_context: &DbContext,
) -> Result<(), DSError> {
    let db_context = &db_context.clone();

    tracing::debug!("Warming NHL database key cache");
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
    tracing::debug!("NHL key cache warmed");
    Ok(())
}
