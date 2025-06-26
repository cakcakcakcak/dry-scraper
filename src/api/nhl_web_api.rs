use crate::lp_error;

use crate::api::cacheable::CacheableApi;
use crate::models::nhl_player::NhlPlayer;
use crate::serde_helpers::{get_key_as_i32, get_key_as_string, get_key_as_value};

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
    pub async fn get_nhl_player(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        player_id: i32,
    ) -> Result<NhlPlayer, lp_error::LPError> {
        tracing::debug!("NHL player with id {} not found in lp database.", player_id);
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
        tracing::debug!("NHL player with id {} not found in lp database.", player_id);

        // construct endpoint url
        let endpoint = format!("{}/player/{}/landing", self.base_url, player_id);
        tracing::debug!(
            "NHL player with id {} not found in lp database. Fetching from API.",
            player_id
        );

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        let mut player: NhlPlayer = serde_json::from_value(raw_json.clone())?;
        if let Some(draft_details) = get_key_as_value(&raw_json, "draftDetails") {
            player.draft_year = get_key_as_i32(&draft_details, "year");
            player.draft_team_abbreviation = get_key_as_string(&draft_details, "teamAbbrev");
            player.draft_round = get_key_as_i32(&draft_details, "round");
            player.draft_pick_in_round = get_key_as_i32(&draft_details, "pickInRound");
            player.draft_overall_pick = get_key_as_i32(&draft_details, "overallPick");
        }

        player.raw_json = Some(raw_json.clone());
        player.api_cache_endpoint = Some(endpoint.clone());

        tracing::debug!("Upserting player {} into lp database.", player_id);
        player.upsert(&pool).await?;
        tracing::debug!("Upserted player {} into lp database.", player_id);

        Ok(player)
    }
}
