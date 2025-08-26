use sqlx::postgres::PgQueryResult;

use super::super::primary_key::*;
use super::{api::NhlApi, models::*};
use crate::common::{
    db::{DbContext, DbEntity, DbEntityVecExt},
    errors::LPError,
    models::{
        ApiCache, ItemParsedWithContext, ItemParsedWithContextVecExt,
        traits::{DbStruct, HasTypeName, IntoDbStruct},
    },
    util::track_and_filter_errors,
};

pub async fn get_resource<J, D, Fut>(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    fetch_fn: Fut,
) -> Result<Vec<D>, LPError>
where
    J: IntoDbStruct<DbStruct = D>,
    D: DbStruct + DbEntity<Pk = NhlPrimaryKey> + HasTypeName,
    Fut: std::future::Future<Output = Result<Vec<ItemParsedWithContext<J>>, LPError>>,
{
    let j_name: &'static str = J::type_name();
    let d_name: &'static str = D::type_name();

    tracing::info!("Fetching all available `{j_name}`s from NHL API.");
    let json_structs: Vec<ItemParsedWithContext<J>> = fetch_fn.await?;
    let json_struct_count: usize = json_structs.len();
    tracing::info!("Successfully fetched {json_struct_count} `{j_name}`s from NHL API",);

    let db_structs: Vec<D> = json_structs.into_db_structs();
    let db_struct_count: usize = db_structs.len();
    tracing::info!("Parsed {db_struct_count}/{json_struct_count} `{j_name}`s into `{d_name}`s.",);

    tracing::info!("Upserting {db_struct_count} `{d_name}`s into lp database.",);
    let upsert_results: Vec<Option<PgQueryResult>> =
        db_structs.upsert_all(db_context, nhl_api).await;
    let ok_upsert_count: usize = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{db_struct_count} `{d_name}`s into lp database.");

    Ok(db_structs)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_seasons(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlSeason>, LPError> {
    get_resource(db_context, nhl_api, nhl_api.seasons().list(db_context)).await
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_franchises(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlFranchise>, LPError> {
    get_resource(db_context, nhl_api, nhl_api.franchises().list(db_context)).await
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_teams(
    db_context: &DbContext,
    nhl_api: &NhlApi,
) -> Result<Vec<NhlTeam>, LPError> {
    get_resource(db_context, nhl_api, nhl_api.teams().list(db_context)).await
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_shifts_in_game(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    game_id: i32,
) -> Result<Vec<NhlShift>, LPError> {
    get_resource(
        db_context,
        nhl_api,
        nhl_api.shifts().list_shifts_for_game(db_context, game_id),
    )
    .await
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
    let game_ids: Vec<i32> = (1..=number_of_games)
        .map(|game_number| {
            let id_string: String = format!("{prefix}{game_number:04}");
            id_string.parse::<i32>().unwrap()
        })
        .collect();

    tracing::info!("Fetching {number_of_games} games from NHL API or cache.");
    let json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, LPError>> =
        nhl_api.games().get_many(db_context, game_ids).await;
    let ok_json_results: Vec<ItemParsedWithContext<NhlGameJson>> =
        track_and_filter_errors(json_results, db_context).await;
    let ok_json_result_count = ok_json_results.len();
    tracing::info!(
        "Successfully fetched {ok_json_result_count}/{number_of_games} games from NHL API or cache."
    );

    let games: Vec<NhlGame> = ok_json_results.into_db_structs();
    let game_count = games.len();
    tracing::info!("Parsed {game_count}/{number_of_games} games into lp database structs.");

    tracing::info!("Upserting {number_of_games} games  into lp database.");
    let upsert_results: Vec<Option<PgQueryResult>> = games.upsert_all(db_context, nhl_api).await;
    let ok_upsert_count: usize = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{number_of_games} games into lp database.");

    Ok(games)
}

#[tracing::instrument(skip(db_context, nhl_api))]
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
    let roster_spot_count = roster_spots.len();
    tracing::info!("Parsed {roster_spot_count} roster spots into lp database structs.");

    tracing::info!("Upserting {roster_spot_count} roster spots lp database.",);
    let upsert_results = roster_spots.upsert_all(db_context, nhl_api).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{roster_spot_count} roster spots into lp database.",);

    Ok(roster_spots)
}

#[tracing::instrument(skip(db_context, nhl_api))]
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
                    context: NhlGameContext {
                        game_id: game.id,
                        endpoint: game.endpoint.clone(),
                        raw_json,
                    },
                }
            })
            .collect();

    let plays: Vec<NhlPlay> = play_jsons_with_context.into_db_structs();
    let play_count = plays.len();
    tracing::info!("Parsed {play_count} plays into lp database structs.",);

    tracing::info!("Upserting {play_count} plays into lp database.",);
    let upsert_results = plays.upsert_all(db_context, nhl_api).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{play_count} plays into lp database.",);

    Ok(plays)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_playoff_bracket_series(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    season: &NhlSeason,
) -> Result<Vec<NhlPlayoffBracketSeries>, LPError> {
    let season_id: i32 = season.id;
    let year_id: i32 = season_id.to_string()[4..].parse::<i32>().unwrap();
    get_resource(
        db_context,
        nhl_api,
        nhl_api
            .playoff_bracket()
            .list_playoff_series_for_year(db_context, year_id),
    )
    .await
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_playoff_series(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    bracket_series: &NhlPlayoffBracketSeries,
) -> Result<NhlPlayoffSeries, LPError> {
    let season_id: i32 = bracket_series.season_id;
    let series_letter: &str = &bracket_series.series_letter;
    let series_json: ItemParsedWithContext<NhlPlayoffSeriesJson> = nhl_api
        .playoff_series()
        .get(db_context, season_id, series_letter)
        .await?;

    let series: NhlPlayoffSeries = series_json.clone().into_db_struct();
    tracing::info!("Parsed playoff series into lp database struct.");

    tracing::info!("Upserting playoff series into lp database.",);

    series
        .fix_relationships_and_upsert(db_context, nhl_api)
        .await?;
    tracing::info!("Upserted playoff series into lp database.",);

    let series_game_jsons: Vec<ItemParsedWithContext<NhlPlayoffSeriesGameJson>> = series_json
        .item
        .games
        .iter()
        .map(|game_json| {
                let raw_json: serde_json::Value = match serde_json::to_value(&game_json) {
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
    let series_game_count = series_games.len();
    tracing::info!("Parsed {series_game_count} games into lp database structs.",);

    tracing::info!("Upserting {series_game_count} games  into lp database.",);
    let upsert_results = series_games.upsert_all(db_context, nhl_api).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!("Upserted {ok_upsert_count}/{series_game_count} games into lp database.",);

    Ok(series)
}

#[tracing::instrument(skip(db_context, nhl_api))]
pub async fn get_nhl_games_in_playoff_series(
    db_context: &DbContext,
    nhl_api: &NhlApi,
    series: &NhlPlayoffSeries,
) -> Result<Vec<NhlGame>, LPError> {
    let game_ids: Vec<i32> = series.game_ids.to_vec();
    let number_of_games: usize = game_ids.len();

    tracing::info!("Fetching {number_of_games} game play-by-play reports from NHL API or cache.");
    let game_json_results: Vec<Result<ItemParsedWithContext<NhlGameJson>, LPError>> =
        nhl_api.games().get_many(db_context, game_ids).await;
    let game_jsons: Vec<ItemParsedWithContext<NhlGameJson>> =
        track_and_filter_errors(game_json_results, db_context).await;
    let ok_game_json_count: usize = game_jsons.len();
    tracing::info!(
        "Fetched {ok_game_json_count}/{number_of_games} game play-by-play reports from NHL API or cache."
    );
    let games: Vec<NhlGame> = game_jsons.into_db_structs();
    let ok_game_count: usize = games.len();
    tracing::info!(
        "Parsed {ok_game_count}/{number_of_games} game play-by-play reports lp database structs."
    );

    tracing::info!("Upserting {ok_game_count} game play-by-play reports into lp database.");
    let upsert_results = games.upsert_all(db_context, nhl_api).await;
    let ok_upsert_count = upsert_results.len();
    tracing::info!(
        "Upserted {ok_upsert_count}/{number_of_games} game play-by-play reports into lp database."
    );

    Ok(games)
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
    NhlPlayoffBracketSeries::warm_key_cache(db_context).await?;
    tracing::info!("Warmed NHL database key cache.");
    Ok(())
}
