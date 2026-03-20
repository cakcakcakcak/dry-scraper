use futures::stream::{self, StreamExt};

use super::{api::NhlApi, models::*, primary_key::NhlSeasonKey};

use crate::common::{
    app_context::AppContext,
    db::{
        all_foreign_keys_cached, find_missing_foreign_keys, group_cache_keys_by_table, DbContext,
        DbEntity, DbEntityVecExt, SqlxJobOrFlush,
    },
    errors::DSError,
    models::{
        partition_and_track_errors,
        traits::{HasTypeName, IntoDbStruct},
        DataSourceError, ItemParsedWithContext, ItemParsedWithContextVecExt,
    },
};

/// Ensure all foreign keys referenced by entities exist, fetching them if necessary.
///
/// First checks the cache for missing FKs. If any are missing, attempts to fetch them
/// from the API and upsert to the database. Only logs warnings if fetching fails.
///
/// Returns the number of entities fetched, or an error if fetching failed.
async fn ensure_and_fetch_foreign_keys<T>(
    entities: &[T],
    app_context: &AppContext,
    db_context: &DbContext,
    api: &NhlApi,
) -> Result<usize, DSError>
where
    T: DbEntity + HasTypeName,
{
    if all_foreign_keys_cached(entities, db_context) {
        return Ok(0);
    }

    let missing_fks = find_missing_foreign_keys(entities, db_context);
    let grouped = group_cache_keys_by_table(&missing_fks);

    // Fetch different FK types in parallel
    let fetch_futures: Vec<_> = grouped
        .into_iter()
        .map(|(table, cache_keys)| async move {
            match table.as_str() {
                "player" => {
                    tracing::info!("Fetching {} missing players", cache_keys.len());
                    let player_ids: Vec<i32> = cache_keys
                        .iter()
                        .filter_map(|ck| ck.id.parse::<i32>().ok())
                        .collect();

                    let results = api
                        .get_many_players(app_context, db_context, player_ids)
                        .await;
                    let players: Vec<NhlPlayer> = results
                        .into_iter()
                        .filter_map(|r| r.ok())
                        .map(|parsed| parsed.item.into_db_struct(parsed.context))
                        .collect();

                    if players.len() < cache_keys.len() {
                        tracing::warn!(
                            "Failed to fetch {} of {} missing players",
                            cache_keys.len() - players.len(),
                            cache_keys.len()
                        );
                    }

                    let count = players.len();
                    let _ = players.upsert_all(db_context).await;
                    count
                }
                "game" => {
                    tracing::info!("Fetching {} missing games", cache_keys.len());
                    let game_ids: Vec<i32> = cache_keys
                        .iter()
                        .filter_map(|ck| ck.id.parse::<i32>().ok())
                        .collect();

                    let results: Vec<_> = stream::iter(game_ids)
                        .map(|game_id| async move {
                            get_nhl_game(app_context, db_context, api, game_id).await
                        })
                        .buffer_unordered(app_context.config.db_concurrency_limit)
                        .collect()
                        .await;

                    let games: Vec<_> = results.into_iter().filter_map(|r| r.ok()).collect();
                    if games.len() < cache_keys.len() {
                        tracing::warn!(
                            "Failed to fetch {} of {} missing games",
                            cache_keys.len() - games.len(),
                            cache_keys.len()
                        );
                    }
                    games.len()
                }
                "team" => {
                    tracing::warn!(
                        "{} missing team FKs - teams should be fetched at startup",
                        cache_keys.len()
                    );
                    0
                }
                "season" => {
                    tracing::warn!(
                        "{} missing season FKs - seasons should be fetched at startup",
                        cache_keys.len()
                    );
                    0
                }
                "franchise" => {
                    tracing::warn!(
                        "{} missing franchise FKs - franchises should be fetched at startup",
                        cache_keys.len()
                    );
                    0
                }
                "api_cache" => {
                    // API cache entries are created during fetch, not fetched separately
                    tracing::debug!("Skipping {} api_cache FK entries", cache_keys.len());
                    0
                }
                "playoff_series" => {
                    tracing::warn!(
                        "{} missing playoff_series FKs - playoff series should be fetched before playoff series games",
                        cache_keys.len()
                    );
                    0
                }
                "playoff_bracket_series" => {
                    tracing::warn!(
                        "{} missing playoff_bracket_series FKs - playoff bracket series should be fetched before playoff series",
                        cache_keys.len()
                    );
                    0
                }
                _ => {
                    tracing::warn!(
                        "Unknown FK table type: {} ({} keys)",
                        table,
                        cache_keys.len()
                    );
                    0
                }
            }
        })
        .collect();

    let results = futures::future::join_all(fetch_futures).await;
    let total_fetched: usize = results.into_iter().sum();

    if total_fetched > 0 {
        tracing::info!("Fetched {} missing foreign key entities", total_fetched);
    }
    Ok(total_fetched)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_seasons(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlSeason>, DSError> {
    let season_results = nhl_api.get_list_seasons(db_context).await?;

    let (seasons, _) = partition_and_track_errors(
        season_results,
        db_context,
        "Parse errors during season fetch",
    );

    let db_seasons = seasons.into_db_structs();
    let _ = db_seasons.upsert_all(db_context).await;
    Ok(db_seasons)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_franchises(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlFranchise>, DSError> {
    let franchise_results = nhl_api.get_list_franchises(db_context).await?;

    let (franchises, _) = partition_and_track_errors(
        franchise_results,
        db_context,
        "Parse errors during franchise fetch",
    );

    let db_franchises = franchises.into_db_structs();
    let _ = db_franchises.upsert_all(db_context).await;
    Ok(db_franchises)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_teams(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlTeam>, DSError> {
    let team_results = nhl_api.get_list_teams(db_context).await?;

    let (teams, _) =
        partition_and_track_errors(team_results, db_context, "Parse errors during team fetch");

    let db_teams = teams.into_db_structs();
    let _ = db_teams.upsert_all(db_context).await;
    Ok(db_teams)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_shifts_in_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_id: i32,
) -> Result<Vec<NhlShift>, DSError> {
    let shift_results = nhl_api
        .get_list_shifts_for_game(db_context, game_id)
        .await?;

    let (shifts, _) = partition_and_track_errors(
        shift_results,
        db_context,
        &format!("Parse errors during shift fetch for game {game_id}"),
    );

    let db_shifts = shifts.into_db_structs();
    let _ = db_shifts.upsert_all(db_context).await;
    Ok(db_shifts)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_everything_in_season(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season_id: i32,
) -> Result<(), DSError> {
    let mut games =
        get_nhl_all_games_in_season(app_context, db_context, nhl_api, season_id).await?;

    let playoff_spinner = app_context
        .progress_reporter_mode
        .create_reporter(None, "Fetching playoff games...");

    let playoff_bracket_series =
        get_nhl_playoff_bracket_series(db_context, nhl_api, season_id).await?;

    let _ = db_context.sqlx_tx.send(SqlxJobOrFlush::Flush).await;

    for bracket_series in playoff_bracket_series {
        let series = get_nhl_playoff_series(
            app_context,
            db_context,
            nhl_api,
            bracket_series.season_id,
            &bracket_series.series_letter,
        )
        .await?;
        let mut playoff_games =
            get_nhl_games_in_playoff_series(app_context, db_context, nhl_api, &series.game_ids)
                .await?;
        games.append(&mut playoff_games);
    }
    playoff_spinner.finish();

    // Fetch ancillary data for all games, bounded by concurrency limit
    let ancillary_results: Vec<_> = stream::iter(games.iter())
        .map(|game| async move {
            tokio::try_join!(
                get_nhl_plays_in_game(app_context, db_context, nhl_api, game),
                get_nhl_roster_spots_in_game(app_context, db_context, nhl_api, game),
                get_nhl_shifts_in_game(db_context, nhl_api, game.id),
            )
        })
        .buffer_unordered(app_context.config.db_concurrency_limit)
        .collect()
        .await;

    for result in ancillary_results {
        result?;
    }

    Ok(())
}

/// Fetch multiple games, handle errors, ensure FKs, and upsert to database.
///
/// This is a helper function for batch game processing used by both regular season
/// and playoff game fetching. It handles the full pipeline: fetch → partition errors →
/// track errors → parse → FK check → upsert.
async fn fetch_and_upsert_games(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_ids: Vec<i32>,
    error_context: &str,
) -> Result<Vec<NhlGame>, DSError> {
    let json_results = nhl_api
        .get_many_games(app_context, db_context, game_ids)
        .await;

    let (ok_jsons, _) = partition_and_track_errors(
        json_results,
        db_context,
        &format!("Parse errors occurred while {error_context}"),
    );

    let games = ok_jsons.into_db_structs();
    _ = ensure_and_fetch_foreign_keys(&games, app_context, db_context, nhl_api).await?;
    let _ = games.upsert_all(db_context).await;

    Ok(games)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_all_games_in_season(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season_id: i32,
) -> Result<Vec<NhlGame>, DSError> {
    let season_key = NhlSeasonKey { id: season_id };
    let season = match NhlSeason::fetch_from_db_by_key(db_context, &season_key).await? {
        Some(s) => s,
        None => {
            return Err(DSError::DatabaseCustom(format!(
                "Season {} not found in database. Fetch seasons first.",
                season_id
            )))
        }
    };

    let number_of_games: i32 = season.total_regular_season_games;
    let season_id_str: String = season_id.to_string();

    let prefix: String = format!("{}02", &season_id_str[..4]);
    let game_ids: Vec<i32> = (1..=number_of_games)
        .map(|game_number| {
            let id_string: String = format!("{prefix}{game_number:04}");
            id_string
                .parse::<i32>()
                .expect("Game ID should always be valid i32")
        })
        .collect();

    tracing::info!(
        season_id = %season_id_str,
        game_count = number_of_games,
        "Fetching regular season games"
    );

    let games = fetch_and_upsert_games(
        app_context,
        db_context,
        nhl_api,
        game_ids,
        &format!("fetching regular season games for season {season_id_str}"),
    )
    .await?;

    tracing::info!(
        season_id = %season_id_str,
        upserted = games.len(),
        "Regular season games upserted"
    );

    Ok(games)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_game(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_id: i32,
) -> Result<NhlGame, DSError> {
    let json_result = nhl_api.get_game(app_context, db_context, game_id).await;

    // Handle parse errors
    let game_json = match json_result {
        Ok(item) => item,
        Err(e) => {
            tracing::warn!(
                game_id = game_id,
                "Parse error occurred while fetching game"
            );
            DataSourceError::track_error(e, db_context);
            return Err(DSError::ApiCustom(format!(
                "Failed to fetch or parse game {game_id}"
            )));
        }
    };

    let game: NhlGame = game_json.into_db_struct();

    // Check for missing foreign keys (teams, season) before upserting
    _ = ensure_and_fetch_foreign_keys(
        std::slice::from_ref(&game),
        app_context,
        db_context,
        nhl_api,
    )
    .await?;

    let _upsert_result = game.upsert(db_context).await;

    // Fetch ancillary game data (plays, roster spots, shifts)
    let (plays_res, roster_res, shifts_res) = tokio::join!(
        get_nhl_plays_in_game(app_context, db_context, nhl_api, &game),
        get_nhl_roster_spots_in_game(app_context, db_context, nhl_api, &game),
        get_nhl_shifts_in_game(db_context, nhl_api, game.id),
    );
    plays_res?;
    roster_res?;
    shifts_res?;

    Ok(game)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_roster_spots_in_game(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game: &NhlGame,
) -> Result<Vec<NhlRosterSpot>, DSError> {
    let _game_id: i32 = game.id;
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

    let roster_spots: Vec<NhlRosterSpot> = roster_spot_jsons_with_context.into_db_structs();

    _ = ensure_and_fetch_foreign_keys(&roster_spots, app_context, db_context, nhl_api).await?;

    let _ = roster_spots.upsert_all(db_context).await;
    tracing::debug!(
        game_id = game.id,
        upserted = roster_spots.len(),
        "Roster spots upserted"
    );

    Ok(roster_spots)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_plays_in_game(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game: &NhlGame,
) -> Result<Vec<NhlPlay>, DSError> {
    let _game_id: i32 = game.id;
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

    let plays: Vec<NhlPlay> = play_jsons_with_context.into_db_structs();

    _ = ensure_and_fetch_foreign_keys(&plays, app_context, db_context, nhl_api).await?;

    let _ = plays.upsert_all(db_context).await;
    tracing::debug!(game_id = game.id, upserted = plays.len(), "Plays upserted");

    Ok(plays)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_playoff_bracket_series(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season_id: i32,
) -> Result<Vec<NhlPlayoffBracketSeries>, DSError> {
    let year_id: i32 = season_id.to_string()[4..]
        .parse::<i32>()
        .map_err(DSError::Parse)?;

    let bracket_results = nhl_api
        .get_list_playoff_series_for_year(db_context, year_id)
        .await?;

    let (brackets, _) = partition_and_track_errors(
        bracket_results,
        db_context,
        &format!("Parse errors during playoff bracket fetch for season {season_id}"),
    );

    let db_brackets = brackets.into_db_structs();
    let _ = db_brackets.upsert_all(db_context).await;
    Ok(db_brackets)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_playoff_series(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season_id: i32,
    series_letter: &str,
) -> Result<NhlPlayoffSeries, DSError> {
    let series_json: ItemParsedWithContext<NhlPlayoffSeriesJson> = nhl_api
        .get_playoff_series(db_context, season_id, series_letter)
        .await?;

    let series: NhlPlayoffSeries = series_json.clone().into_db_struct();

    _ = ensure_and_fetch_foreign_keys(
        std::slice::from_ref(&series),
        app_context,
        db_context,
        nhl_api,
    )
    .await?;

    series.upsert(db_context).await?;

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
    let series_games: Vec<NhlPlayoffSeriesGame> = series_game_jsons.into_db_structs();

    _ = ensure_and_fetch_foreign_keys(&series_games, app_context, db_context, nhl_api).await?;

    let _ = series_games.upsert_all(db_context).await;
    tracing::debug!(
        season_id,
        series_letter,
        upserted = series_games.len(),
        "Playoff series games upserted"
    );

    Ok(series)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_games_in_playoff_series(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_ids: &[i32],
) -> Result<Vec<NhlGame>, DSError> {
    let game_ids_vec: Vec<i32> = game_ids.to_vec();
    let number_of_games: usize = game_ids_vec.len();

    tracing::info!(
        game_count = number_of_games,
        "Fetching playoff game play-by-play"
    );

    let games = fetch_and_upsert_games(
        app_context,
        db_context,
        nhl_api,
        game_ids_vec,
        "fetching playoff games",
    )
    .await?;

    tracing::info!(upserted = games.len(), "Playoff game play-by-play upserted");

    Ok(games)
}
