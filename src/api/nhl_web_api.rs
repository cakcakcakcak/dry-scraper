use crate::lp_error;

use crate::api::cacheable::CacheableApi;
use crate::models::nhl_game::NhlGame;
use crate::models::nhl_player::NhlPlayer;
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
                .bind(player_id)
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
                .bind(game_id)
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
}
