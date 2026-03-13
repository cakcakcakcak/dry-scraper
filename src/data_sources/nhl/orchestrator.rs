use sqlx::postgres::PgQueryResult;

use super::{api::NhlApi, models::*, primary_key::NhlSeasonKey};

use crate::common::{
    app_context::AppContext,
    db::{
        all_foreign_keys_cached, find_missing_foreign_keys, CacheKey, DbContext, DbEntity,
        DbEntityVecExt,
    },
    errors::DSError,
    models::{
        traits::HasTypeName, DataSourceError, ItemParsedWithContext, ItemParsedWithContextVecExt,
    },
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

    // Group missing FKs by table for better reporting
    let mut fk_by_table = std::collections::HashMap::new();
    for fk in &missing_fks {
        fk_by_table
            .entry(&fk.table)
            .or_insert_with(Vec::new)
            .push(&fk.id);
    }

    let mut table_summary = String::new();
    for (table, ids) in &fk_by_table {
        if !table_summary.is_empty() {
            table_summary.push_str(", ");
        }
        table_summary.push_str(&format!("{}: {}", table, ids.len()));
    }

    tracing::warn!(
        "{} `{}` entities reference {} missing foreign keys ({}). Some upserts may fail if FKs don't exist.",
        entities.len(),
        type_name,
        missing_fks.len(),
        table_summary
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

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_seasons(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlSeason>, DSError> {
    let season_results = nhl_api.list_seasons(db_context).await?;

    let mut seasons = Vec::new();
    let mut errors = Vec::new();
    for result in season_results {
        match result {
            Ok(item) => seasons.push(item),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        tracing::warn!(
            successful = seasons.len(),
            failed = errors.len(),
            "Parse errors during season fetch"
        );
        for error in errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let db_seasons = seasons.into_db_structs(app_context, "Parsing seasons");
    ensure_foreign_keys_exist(&db_seasons, db_context).await;

    db_seasons.upsert_all(app_context, db_context).await;
    Ok(db_seasons)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_franchises(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlFranchise>, DSError> {
    let franchise_results = nhl_api.list_franchises(db_context).await?;

    let mut franchises = Vec::new();
    let mut errors = Vec::new();
    for result in franchise_results {
        match result {
            Ok(item) => franchises.push(item),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        tracing::warn!(
            successful = franchises.len(),
            failed = errors.len(),
            "Parse errors during franchise fetch"
        );
        for error in errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let db_franchises = franchises.into_db_structs(app_context, "Parsing franchises");
    ensure_foreign_keys_exist(&db_franchises, db_context).await;

    db_franchises.upsert_all(app_context, db_context).await;
    Ok(db_franchises)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_teams(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlTeam>, DSError> {
    let team_results = nhl_api.list_teams(db_context).await?;

    let mut teams = Vec::new();
    let mut errors = Vec::new();
    for result in team_results {
        match result {
            Ok(item) => teams.push(item),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        tracing::warn!(
            successful = teams.len(),
            failed = errors.len(),
            "Parse errors during team fetch"
        );
        for error in errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let db_teams = teams.into_db_structs(app_context, "Parsing teams");
    ensure_foreign_keys_exist(&db_teams, db_context).await;

    db_teams.upsert_all(app_context, db_context).await;
    Ok(db_teams)
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_shifts_in_game(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_id: i32,
) -> Result<Vec<NhlShift>, DSError> {
    let shift_results = nhl_api.list_shifts_for_game(db_context, game_id).await?;

    let mut shifts = Vec::new();
    let mut errors = Vec::new();
    for result in shift_results {
        match result {
            Ok(item) => shifts.push(item),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        tracing::warn!(
            game_id,
            successful = shifts.len(),
            failed = errors.len(),
            "Parse errors during shift fetch"
        );
        for error in errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let db_shifts =
        shifts.into_db_structs(app_context, &format!("Parsing shifts for game {game_id}"));
    ensure_foreign_keys_exist(&db_shifts, db_context).await;

    db_shifts.upsert_all(app_context, db_context).await;
    Ok(db_shifts)
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

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_season_games(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season_id: i32,
) -> Result<Vec<NhlGame>, DSError> {
    tracing::info!(season_id = season_id, "Fetching season games");

    // First, ensure the season exists in the database
    let season_key = NhlSeasonKey { id: season_id };
    let season = match NhlSeason::fetch_from_db_by_key(db_context, &season_key).await? {
        Some(s) => s,
        None => {
            tracing::warn!(season_id = season_id, "Season not found, fetching from API");
            // Fetch all seasons to ensure it exists
            get_nhl_seasons(app_context, db_context, nhl_api).await?;

            // Try again
            match NhlSeason::fetch_from_db_by_key(db_context, &season_key).await? {
                Some(s) => s,
                None => {
                    return Err(DSError::DatabaseCustom(format!(
                        "Season {} not found in database after fetch",
                        season_id
                    )))
                }
            }
        }
    };

    // Fetch all games in the season
    get_nhl_all_games_in_season(app_context, db_context, nhl_api, &season).await
}

#[tracing::instrument(skip(app_context, db_context, nhl_api))]
pub async fn get_nhl_game(
    app_context: &AppContext,
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_id: i32,
) -> Result<NhlGame, DSError> {
    tracing::info!(game_id = game_id, "Fetching single game");

    let json_result = nhl_api.get_game(db_context, game_id).await;

    // Handle parse errors
    let game_json = match json_result {
        Ok(item) => item,
        Err(e) => {
            tracing::warn!(
                game_id = game_id,
                "Parse error occurred while fetching game"
            );
            DataSourceError::track_error(e, db_context).await;
            return Err(DSError::ApiCustom(format!(
                "Failed to fetch or parse game {game_id}"
            )));
        }
    };

    let game: NhlGame = game_json.into_db_struct();

    // Check for missing foreign keys (teams, season) before upserting
    ensure_foreign_keys_exist(std::slice::from_ref(&game), db_context).await;

    let _upsert_result = game.upsert(db_context).await;
    tracing::info!(game_id = game_id, "Game upserted");

    // Fetch ancillary game data (plays, roster spots, shifts)
    let (plays_res, roster_res, shifts_res) = tokio::join!(
        get_nhl_plays_in_game(app_context, db_context, &game),
        get_nhl_roster_spots_in_game(app_context, db_context, &game),
        get_nhl_shifts_in_game(app_context, db_context, nhl_api, game.id),
    );
    plays_res?;
    roster_res?;
    shifts_res?;

    Ok(game)
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

    // Create minimal player stubs from roster spot data to satisfy FK constraints
    let players: Vec<NhlPlayer> = roster_spots
        .iter()
        .map(|rs| NhlPlayer {
            id: rs.player_id,
            first_name: rs.first_name.clone(),
            last_name: rs.last_name.clone(),
            is_active: true,
            current_team_id: Some(rs.team_id),
            current_team_abbrev: None,
            full_team_name: None,
            team_common_name: None,
            team_place_name_with_preposition: None,
            team_logo: None,
            sweater_number: Some(rs.sweater_number),
            position: rs.position_code.clone(),
            headshot: rs.headshot.clone(),
            hero_image: String::new(),
            height_in_inches: None,
            height_in_centimeters: None,
            weight_in_pounds: None,
            weight_in_kilograms: None,
            birth_date: chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap(),
            birth_city: String::new(),
            birth_state_province: None,
            birth_country: String::new(),
            shoots_catches: None,
            draft_year: None,
            draft_team_abbreviation: None,
            draft_round: None,
            draft_pick_in_round: None,
            draft_overall_pick: None,
            player_slug: format!(
                "{}-{}",
                rs.first_name.to_lowercase(),
                rs.last_name.to_lowercase()
            ),
            in_top100_all_time: false,
            in_hhof: false,
            raw_json: serde_json::json!({}),
            endpoint: rs.endpoint.clone(),
        })
        .collect();

    // Upsert player stubs first to satisfy FK constraints
    players.upsert_all(app_context, db_context).await;

    // Check for missing foreign keys (game, teams) before upserting
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

    let bracket_results = nhl_api
        .list_playoff_series_for_year(db_context, year_id)
        .await?;

    let mut brackets = Vec::new();
    let mut errors = Vec::new();
    for result in bracket_results {
        match result {
            Ok(item) => brackets.push(item),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        tracing::warn!(
            season_id,
            successful = brackets.len(),
            failed = errors.len(),
            "Parse errors during playoff bracket fetch"
        );
        for error in errors {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    let db_brackets = brackets.into_db_structs(app_context, "Parsing playoff bracket series");
    ensure_foreign_keys_exist(&db_brackets, db_context).await;

    db_brackets.upsert_all(app_context, db_context).await;
    Ok(db_brackets)
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
