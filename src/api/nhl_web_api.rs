use futures::future::join_all;

use crate::api::cacheable::CacheableApi;
use crate::api::nhl_stats_api::NhlStatsApi;
use crate::lp_error;
use crate::models::nhl_game::NhlGame;
use crate::models::nhl_player::NhlPlayer;
use crate::models::nhl_playoff_series::{NhlPlayoffBracket, NhlPlayoffSeries};
use crate::models::period_type::PeriodType;
use crate::serde_helpers::JsonExt;

use crate::sqlx_operation_with_retries;

pub struct NhlWebApi {
    pub client: reqwest::Client,
    pub base_url: String,
}
impl NhlWebApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api-web.nhle.com/v1".to_string(),
        }
    }
}
impl std::fmt::Debug for NhlWebApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // don't print the Client struct or the base_url
        f.debug_struct("NhlWebApi").finish()
    }
}
impl CacheableApi for NhlWebApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl NhlWebApi {
    #[tracing::instrument(skip(pool))]
    pub async fn get_nhl_player(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        player_id: i32,
    ) -> Result<NhlPlayer, lp_error::LPError> {
        tracing::debug!("Querying lp database for player with ID {player_id}.");
        let player: Option<NhlPlayer> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlPlayer>("SELECT * FROM nhl_player WHERE id = $1")
                .bind(&player_id)
                .fetch_optional(pool)
                .await
        )
        .await?;

        if let Some(player) = player {
            return Ok(player);
        }
        tracing::debug!("NHL player with ID {player_id} not found in lp database.");

        // construct endpoint url
        let base_url = &self.base_url;
        let endpoint = format!("{base_url}/player/{player_id}/landing",);

        // get or cache contents of endpoint and serde the response into json
        tracing::debug!("Fetching NHL player with ID {player_id} from API.");
        let raw_json = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let json_value: serde_json::Value = serde_json::from_str(&raw_json)?;

        let mut player: NhlPlayer = serde_json::from_value(json_value.clone())?;
        if let Some(draft_details) =
            json_value.get_key_as_logged::<serde_json::Value>("draftDetails")
        {
            player.draft_year = draft_details.get_key_as_i32("year");
            player.draft_team_abbreviation =
                draft_details.get_key_as_logged::<String>("teamAbbrev");
            player.draft_team_abbreviation = draft_details.get_key_as_string("teamAbbrev");
            player.draft_round = draft_details.get_key_as_i32("round");
            player.draft_pick_in_round = draft_details.get_key_as_i32("pickInRound");
            player.draft_overall_pick = draft_details.get_key_as_i32("overallPick");
        }

        player.raw_json = Some(json_value.clone());
        player.api_cache_endpoint = Some(endpoint.clone());

        tracing::debug!("Upserting player with ID {player_id} into lp database.");
        player.upsert(&pool).await?;
        tracing::debug!("Upserted player with ID {player_id} into lp database.");

        Ok(player)
    }

    #[tracing::instrument(skip(pool))]
    pub async fn get_nhl_game(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        game_id: i32,
    ) -> Result<NhlGame, lp_error::LPError> {
        tracing::debug!("Querying lp database for game with ID {game_id}.");
        let game: Option<NhlGame> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlGame>("SELECT * FROM nhl_game WHERE id = $1")
                .bind(&game_id)
                .fetch_optional(pool)
                .await
        )
        .await?;

        if let Some(game) = game {
            return Ok(game);
        }
        tracing::debug!("NHL game with ID {game_id} not found in lp database.");

        // construct endpoint url
        let base_url = &self.base_url;
        let endpoint = format!("{base_url}/gamecenter/{game_id}/play-by-play");

        // get or cache contents of endpoint and serde the response into json
        tracing::debug!("Fetching NHL game with ID {game_id} from API.");
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        let mut game: NhlGame =
            serde_json::from_value(raw_json.clone()).map_err(|e| lp_error::LPError::Serde(e))?;

        if let Some(period_descriptor) = raw_json.get_key_as_value("periodDescriptor") {
            game.period_descriptor_number = period_descriptor.get_key_as_i32("number");
            if let Some(s) = period_descriptor.get_key_as_string("periodType") {
                game.period_descriptor_type = PeriodType::from_str(&s);
            }
            game.period_descriptor_max_regulation_periods =
                period_descriptor.get_key_as_i32("maxRegulationPeriods");
        }

        if let Some(away_team) = raw_json.get_key_as_value("awayTeam") {
            game.away_team_id = away_team.get_key_as_i32("id");
            game.away_team_name = away_team.get_nested_as_string(&["commonName", "default"]);
            game.away_team_abbrev = away_team.get_key_as_string("abbrev");
            game.away_team_score = away_team.get_key_as_i32("score");
            game.away_team_sog = away_team.get_key_as_i32("sog");
            game.away_team_logo = away_team.get_key_as_string("logo");
            game.away_team_dark_logo = away_team.get_key_as_string("darkLogo");
            game.away_team_place_name = away_team.get_nested_as_string(&["placeName", "default"]);
            game.away_team_place_name_with_preposition =
                away_team.get_nested_as_string(&["placeNameWithPreposition", "default"]);
        }

        if let Some(home_team) = raw_json.get_key_as_value("homeTeam") {
            game.home_team_id = home_team.get_key_as_i32("id");
            game.home_team_name = home_team.get_nested_as_string(&["commonName", "default"]);
            game.home_team_abbrev = home_team.get_key_as_string("abbrev");
            game.home_team_score = home_team.get_key_as_i32("score");
            game.home_team_sog = home_team.get_key_as_i32("sog");
            game.home_team_logo = home_team.get_key_as_string("logo");
            game.home_team_dark_logo = home_team.get_key_as_string("darkLogo");
            game.home_team_place_name = home_team.get_nested_as_string(&["placeName", "default"]);
            game.home_team_place_name_with_preposition =
                home_team.get_nested_as_string(&["placeNameWithPreposition", "default"]);
        }
        if let Some(s) = raw_json.get_nested_as_string(&["gameOutcome", "lastPeriodType"]) {
            game.game_outcome_last_period_type = PeriodType::from_str(&s);
        }

        game.raw_json = Some(raw_json.clone());
        game.api_cache_endpoint = Some(endpoint.clone());

        tracing::debug!("Upserting game with ID {game_id} into lp database.");
        game.upsert(&pool).await?;
        tracing::debug!("Upserted game with ID {game_id} into lp database.");

        Ok(game)
    }
    #[tracing::instrument(skip(pool))]
    pub async fn get_nhl_playoff_series(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Vec<NhlPlayoffSeries>, lp_error::LPError> {
        // query nhl_playoff_series database to see if the desired data is already present
        let all_db_series: Vec<NhlPlayoffSeries> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlPlayoffSeries>("SELECT * FROM nhl_playoff_series")
                .fetch_all(pool)
                .await
        )
        .await?;

        // if the series are found, return them
        if !all_db_series.is_empty() {
            tracing::info!(
                "Returning {} cached NHL playoff series from database.",
                all_db_series.len()
            );
            return Ok(all_db_series);
        }

        // query nhl_season table to retrieve list of seasons of form `20242025`
        let seasons: Vec<i32> = sqlx::query_scalar("SELECT id FROM nhl_season")
            .fetch_all(pool)
            .await?;
        // take the latter 4 digits of each season to get the right playoff seasons
        let playoff_seasons: Vec<String> = seasons
            .into_iter()
            .map(|season| season.to_string()[4..].to_string())
            .collect();

        // use base_url and the list of seasons to generate all the endpoints of interest
        let base_url = &self.base_url;
        let bracket_endpoints: Vec<String> = playoff_seasons
            .iter()
            .map(|season| format!("{base_url}/playoff-bracket/{season}"))
            .collect();

        // generate a list of futures to get_or_cache_endpoint
        let bracket_futures = bracket_endpoints
            .iter()
            .map(|endpoint| self.get_or_cache_endpoint(&pool, &endpoint));

        // join_all futures and await the result of each request
        let bracket_results: Vec<Result<String, lp_error::LPError>> =
            join_all(bracket_futures).await;

        // filter_map out the Err(...) results, leaving only Strings of the json response
        let bracket_jsons: Vec<String> = bracket_results
            .into_iter()
            .filter_map(|result| match result {
                Ok(json) => Some(json),
                Err(e) => {
                    tracing::warn!("Failed to fetch playoff bracket: {e}");
                    None
                }
            })
            .collect();

        // attempt to `serde_json`` each `playoff_bracket`` json struct into `NhlPlayoffBracket` rust structs
        // filter_map out the Err(...) values
        let playoff_bracket_values: Vec<NhlPlayoffBracket> = bracket_jsons
            .iter()
            .map(|json| serde_json::from_str::<NhlPlayoffBracket>(&json))
            .zip(&bracket_endpoints)
            .filter_map(|(result, endpoint)| match result {
                Ok(json) => Some(json),
                Err(e) => {
                    tracing::warn!("Failed to parse {endpoint} playoff bracket: {e}");
                    None
                }
            })
            .collect();

        struct BracketContext {
            bracket: NhlPlayoffBracket,
            year: i32,
            endpoint: String,
            json: serde_json::Value,
        }

        let series_tuples: Vec<BracketContext> = playoff_bracket_values
            .into_iter()
            .zip(playoff_seasons)
            .zip(bracket_endpoints)
            .zip(bracket_jsons)
            .map(|(((bracket, year), endpoint), json)| BracketContext {
                bracket,
                year: year.parse::<i32>().unwrap(),
                endpoint,
                json: serde_json::from_str(&json).unwrap(),
            })
            .collect();

        let all_series: Vec<NhlPlayoffSeries> = series_tuples
            .into_iter()
            .flat_map(|context| {
                let series_json_array = context
                    .json
                    .get("series")
                    .and_then(|v| v.as_array())
                    .unwrap();
                context
                    .bracket
                    .series
                    .into_iter()
                    .zip(series_json_array)
                    .filter_map(move |(mut series, series_json)| {
                        series.year = Some(context.year);
                        series.raw_json = Some(series_json.clone());
                        series.api_cache_endpoint = Some(context.endpoint.clone());

                        match series_json.clone().get_key_as_value("topSeedTeam") {
                            Some(top_seed_team) => {
                                series.top_seed_team_id = top_seed_team.get_key_as_i32("id");
                                series.top_seed_team_abbrev =
                                    top_seed_team.get_key_as_string("abbrev");

                                // if `team_name` not present, use `common_name``, and vice-versa
                                series.top_seed_team_name = top_seed_team
                                    .get_nested_as_string(&["name", "default"])
                                    .or_else(|| {
                                        top_seed_team
                                            .get_nested_as_string(&["commonName", "default"])
                                    });
                                series.top_seed_team_common_name = top_seed_team
                                    .get_nested_as_string(&["commonName", "default"])
                                    .or_else(|| {
                                        top_seed_team.get_nested_as_string(&["name", "default"])
                                    })
                                    .clone();

                                series.top_seed_team_place_name_with_preposition = top_seed_team
                                    .get_nested_as_string(&["placeNameWithPreposition", "default"]);
                                series.top_seed_team_logo = top_seed_team.get_key_as_string("logo");
                                series.top_seed_team_dark_logo =
                                    top_seed_team.get_key_as_string("darkLogo");
                            }
                            None => tracing::warn!(
                                "Failed to parse topSeedTeam for {} bracket, series {}",
                                context.year,
                                series.series_letter
                            ),
                        }
                        match series_json.clone().get_key_as_value("bottomSeedTeam") {
                            Some(bottom_seed_team) => {
                                series.bottom_seed_team_id = bottom_seed_team.get_key_as_i32("id");
                                series.bottom_seed_team_abbrev =
                                    bottom_seed_team.get_key_as_string("abbrev");

                                // if `team_name` not present, use `common_name``, and vice-versa
                                series.bottom_seed_team_name = bottom_seed_team
                                    .get_nested_as_string(&["name", "default"])
                                    .or_else(|| {
                                        bottom_seed_team
                                            .get_nested_as_string(&["commonName", "default"])
                                    });
                                series.bottom_seed_team_common_name = bottom_seed_team
                                    .get_nested_as_string(&["commonName", "default"])
                                    .or_else(|| {
                                        bottom_seed_team.get_nested_as_string(&["name", "default"])
                                    });

                                series.bottom_seed_team_place_name_with_preposition =
                                    bottom_seed_team.get_nested_as_string(&[
                                        "placeNameWithPreposition",
                                        "default",
                                    ]);
                                series.bottom_seed_team_logo =
                                    bottom_seed_team.get_key_as_string("logo");
                                series.bottom_seed_team_dark_logo =
                                    bottom_seed_team.get_key_as_string("darkLogo");
                            }
                            None => tracing::warn!(
                                "Failed to parse bottomSeedTeam for {} bracket, series {}",
                                context.year,
                                series.series_letter
                            ),
                        }
                        Some(series)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let all_team_ids: std::collections::HashSet<i32> = all_series
            .iter()
            .flat_map(|series| [series.top_seed_team_id, series.bottom_seed_team_id])
            .flatten()
            .collect();
        let existing_team_ids: std::collections::HashSet<i32> =
            sqlx::query_scalar("SELECT id FROM nhl_team WHERE id = ANY($1)")
                .bind(&all_team_ids.iter().collect::<Vec<_>>())
                .fetch_all(pool)
                .await?
                .into_iter()
                .collect();
        let missing_team_ids = all_team_ids.difference(&existing_team_ids);

        for team_id in missing_team_ids {
            NhlStatsApi::new().get_nhl_team(&pool, *team_id).await?;
        }

        let upserts = all_series.iter().map(|series| series.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed upserts
        all_series
            .iter()
            .zip(upsert_results)
            .for_each(|(series, result)| {
                if let Err(e) = result {
                    tracing::warn!(
                        year = series.year,
                        series_letter = series.series_letter,
                        error = ?e,
                        "Failed to upsert NHL playoff series"
                    );
                }
            });

        tracing::info!(
            "Upserted {} NHL playoff series into database. Now returning them.",
            all_series.len()
        );

        Ok(all_series)
    }
}
