use crate::api::api_common::{ApiContext, FromId, HasEndpoint};
use crate::api::cacheable_api::CacheableApi;
use crate::db::DbPool;
use crate::lp_error::LPError;
use crate::models::item_parsed_with_context::ItemParsedWithContext;
use crate::models::nhl::nhl_game::NhlGameJson;
use crate::models::nhl::nhl_player::NhlPlayerJson;

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
impl ApiContext for NhlWebApi {
    fn base_url(&self) -> &str {
        &self.base_url
    }
}
#[derive(Debug, Default)]
pub struct NhlPlayerParams {
    pub player_id: i32,
}
impl FromId for NhlPlayerParams {
    fn from_id(player_id: i32) -> Self {
        NhlPlayerParams { player_id }
    }
}
impl HasEndpoint for NhlPlayerJson {
    type Params = NhlPlayerParams;

    fn endpoint<A: ApiContext>(api: &A, params: Self::Params) -> String {
        format!("{}/player/{}/landing", api.base_url(), params.player_id)
    }
}
#[derive(Debug, Default)]
pub struct NhlGameParams {
    pub game_id: i32,
}
impl FromId for NhlGameParams {
    fn from_id(game_id: i32) -> Self {
        NhlGameParams { game_id }
    }
}
impl HasEndpoint for NhlGameJson {
    type Params = NhlGameParams;

    fn endpoint<A: ApiContext>(api: &A, params: Self::Params) -> String {
        format!(
            "{}/gamecenter/{}/play-by-play",
            api.base_url(),
            params.game_id
        )
    }
}
impl NhlWebApi {
    #[tracing::instrument(skip(pool))]
    pub async fn fetch_from_id<T>(
        &self,
        pool: &DbPool,
        id: i32,
    ) -> Result<ItemParsedWithContext<T>, LPError>
    where
        T: serde::de::DeserializeOwned + HasEndpoint,
        T::Params: FromId,
    {
        let endpoint: String = T::endpoint(self, T::Params::from_id(id));
        let raw_data: String = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let json_value: serde_json::Value = serde_json::from_str(&raw_data)?;

        let parsed: Result<T, LPError> =
            serde_json::from_value(json_value.clone()).map_err(LPError::from);
        match parsed {
            Ok(item) => Ok(ItemParsedWithContext {
                raw_data,
                item,
                endpoint: endpoint.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    // #[tracing::instrument(skip(pool))]
    // pub async fn get_nhl_game(
    //     &self,
    //     nhl_stats_api: &NhlStatsApi,
    //     pool: &Pool,
    //     game_id: i32,
    // ) -> Result<NhlGameJson, lp_error::LPError> {
    //     tracing::debug!("Querying lp database for game with ID {game_id}.");
    //     let game: Option<NhlGameJson> = sqlx_operation_with_retries!(
    //         sqlx::query_as::<sqlx::Postgres, NhlGameJson>("SELECT * FROM nhl_game WHERE id = $1")
    //             .bind(&game_id)
    //             .fetch_optional(pool)
    //             .await
    //     )
    //     .await?;

    //     if let Some(game) = game {
    //         return Ok(game);
    //     }
    //     tracing::debug!("NHL game with ID {game_id} not found in lp database.");

    //     // construct endpoint url
    //     let base_url = &self.base_url;
    //     let endpoint = nhl_game_endpoint(game_id);

    //     // get or cache contents of endpoint and serde the response into json
    //     tracing::debug!("Fetching NHL game with ID {game_id} from API.");
    //     let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
    //     let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

    //     let mut game: NhlGameJson =
    //         serde_json::from_value(raw_json.clone()).map_err(|e| lp_error::LPError::Serde(e))?;

    //     if let Some(period_descriptor) = raw_json.get_key_as_value("periodDescriptor") {
    //         game.period_descriptor_number = period_descriptor.get_key_as_i32("number");
    //         if let Some(s) = period_descriptor.get_key_as_string("periodType") {
    //             game.period_descriptor_type = PeriodType::from_str(&s);
    //         }
    //         game.period_descriptor_max_regulation_periods =
    //             period_descriptor.get_key_as_i32("maxRegulationPeriods");
    //     }

    //     if let Some(away_team) = raw_json.get_key_as_value("awayTeam") {
    //         game.away_team_id = away_team.get_key_as_i32("id");
    //         game.away_team_name = away_team.get_nested_as_string(&["commonName", "default"]);
    //         game.away_team_abbrev = away_team.get_key_as_string("abbrev");
    //         game.away_team_score = away_team.get_key_as_i32("score");
    //         game.away_team_sog = away_team.get_key_as_i32("sog");
    //         game.away_team_logo = away_team.get_key_as_string("logo");
    //         game.away_team_dark_logo = away_team.get_key_as_string("darkLogo");
    //         game.away_team_place_name = away_team.get_nested_as_string(&["placeName", "default"]);
    //         game.away_team_place_name_with_preposition =
    //             away_team.get_nested_as_string(&["placeNameWithPreposition", "default"]);
    //     }

    //     if let Some(home_team) = raw_json.get_key_as_value("homeTeam") {
    //         game.home_team_id = home_team.get_key_as_i32("id");
    //         game.home_team_name = home_team.get_nested_as_string(&["commonName", "default"]);
    //         game.home_team_abbrev = home_team.get_key_as_string("abbrev");
    //         game.home_team_score = home_team.get_key_as_i32("score");
    //         game.home_team_sog = home_team.get_key_as_i32("sog");
    //         game.home_team_logo = home_team.get_key_as_string("logo");
    //         game.home_team_dark_logo = home_team.get_key_as_string("darkLogo");
    //         game.home_team_place_name = home_team.get_nested_as_string(&["placeName", "default"]);
    //         game.home_team_place_name_with_preposition =
    //             home_team.get_nested_as_string(&["placeNameWithPreposition", "default"]);
    //     }
    //     if let Some(s) = raw_json.get_nested_as_string(&["gameOutcome", "lastPeriodType"]) {
    //         game.game_outcome_last_period_type = PeriodType::from_str(&s);
    //     }

    //     game.raw_json = Some(raw_json.clone());
    //     game.endpoint = Some(endpoint.clone());

    //     tracing::debug!("Upserting game with ID {game_id} into lp database.");
    //     game.upsert(nhl_stats_api, &pool).await?;
    //     tracing::debug!("Upserted game with ID {game_id} into lp database.");

    //     Ok(game)
    // }

    fn year_to_season_id(year: &str) -> i32 {
        let end_year: i32 = year.parse().expect("Year must be a valid integer string");
        (end_year - 1) * 10000 + end_year
    }

    // #[tracing::instrument(skip(pool))]
    // pub async fn get_nhl_playoff_series(
    //     &self,
    //     nhl_stats_api: &NhlStatsApi,
    //     pool: &Pool,
    // ) -> Result<Vec<NhlPlayoffSeries>, lp_error::LPError> {
    //     // query nhl_playoff_series database to see if the desired data is already present
    //     let all_db_series: Vec<NhlPlayoffSeries> = sqlx_operation_with_retries!(
    //         sqlx::query_as::<sqlx::Postgres, NhlPlayoffSeries>(
    //             "SELECT * FROM nhl_playoff_series"
    //         )
    //         .fetch_all(pool)
    //         .await
    //     )
    //     .await?;

    //     // if the series are found, return them
    //     if !all_db_series.is_empty() {
    //         tracing::info!(
    //             "Returning {} cached NHL playoff series from database.",
    //             all_db_series.len()
    //         );
    //         return Ok(all_db_series);
    //     }

    //     // query nhl_season table to retrieve list of seasons of form `20242025`
    //     let seasons: Vec<i32> = sqlx::query_scalar("SELECT id FROM nhl_season")
    //         .fetch_all(pool)
    //         .await?;
    //     // take the latter 4 digits of each season to get the right playoff seasons
    //     let playoff_seasons: Vec<String> = seasons
    //         .into_iter()
    //         .map(|season| season.to_string()[4..].to_string())
    //         .collect();

    //     // use base_url and the list of seasons to generate all the endpoints of interest
    //     let base_url = &self.base_url;
    //     let bracket_endpoints: Vec<String> = playoff_seasons
    //         .iter()
    //         .map(|season| format!("{base_url}/playoff-bracket/{season}"))
    //         .collect();

    //     // generate a list of futures to get_or_cache_endpoint
    //     let bracket_futures = bracket_endpoints
    //         .iter()
    //         .map(|endpoint| self.get_or_cache_endpoint(&pool, &endpoint));

    //     // join_all futures and await the result of each request
    //     let bracket_results: Vec<Result<String, lp_error::LPError>> =
    //         join_all(bracket_futures).await;

    //     // filter_map out the Err(...) results, leaving only Strings of the json response
    //     let bracket_jsons: Vec<String> = bracket_results
    //         .into_iter()
    //         .filter_map(|result| match result {
    //             Ok(json) => Some(json),
    //             Err(e) => {
    //                 tracing::warn!("Failed to fetch playoff bracket: {e}");
    //                 None
    //             }
    //         })
    //         .collect();

    //     // attempt to `serde_json`` each `playoff_bracket`` json struct into `NhlPlayoffBracket` rust structs
    //     // filter_map out the Err(...) values
    //     let playoff_bracket_values: Vec<NhlPlayoffBracketJson> = bracket_jsons
    //         .iter()
    //         .map(|json| serde_json::from_str::<NhlPlayoffBracketJson>(&json))
    //         .zip(&bracket_endpoints)
    //         .filter_map(|(result, endpoint)| match result {
    //             Ok(json) => Some(json),
    //             Err(e) => {
    //                 tracing::warn!("Failed to parse {endpoint} playoff bracket: {e}");
    //                 None
    //             }
    //         })
    //         .collect();

    //     struct BracketContext {
    //         bracket: NhlPlayoffBracketJson,
    //         season_id: i32,
    //         endpoint: String,
    //         json: serde_json::Value,
    //     }

    //     let series_tuples: Vec<BracketContext> = playoff_bracket_values
    //         .into_iter()
    //         .zip(playoff_seasons)
    //         .zip(bracket_endpoints)
    //         .zip(bracket_jsons)
    //         .map(|(((bracket, year), endpoint), json)| BracketContext {
    //             bracket,
    //             season_id: Self::year_to_season_id(&year),
    //             endpoint,
    //             json: serde_json::from_str(&json).unwrap(),
    //         })
    //         .collect();

    //     let all_series: Vec<NhlPlayoffSeries> = series_tuples
    //         .into_iter()
    //         .flat_map(|context| {
    //             let series_json_array = context
    //                 .json
    //                 .get("series")
    //                 .and_then(|v| v.as_array())
    //                 .unwrap();
    //             context
    //                 .bracket
    //                 .series
    //                 .into_iter()
    //                 .zip(series_json_array)
    //                 .filter_map(move |(mut series, series_json)| {
    //                             ()
    //                         }
    //                         None => tracing::warn!(
    //                             "Failed to parse bottomSeedTeam for {} bracket, series {}",
    //                             context.season_id,
    //                             series.series_letter
    //                         ),
    //                     }
    //                     Some(series)
    //                 })
    //                 .collect::<Vec<_>>()
    //         })
    //         .collect();

    //     let upserts = all_series
    //         .iter()
    //         .map(|series| series.upsert(nhl_stats_api, &pool));
    //     let upsert_results = join_all(upserts).await;

    //     // log any failed upserts
    //     all_series
    //         .iter()
    //         .zip(upsert_results)
    //         .for_each(|(series, result)| {
    //             if let Err(e) = result {
    //                 tracing::warn!(
    //                     year = series.season_id,
    //                     series_letter = series.series_letter,
    //                     error = ?e,
    //                     "Failed to upsert NHL playoff series"
    //                 );
    //             }
    //         });

    //     tracing::info!(
    //         "Upserted {} NHL playoff series into database. Now returning them.",
    //         all_series.len()
    //     );

    //     Ok(all_series)
    // }
}
