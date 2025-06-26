use futures::future::join_all;

use crate::lp_error;

use crate::api::cacheable::CacheableApi;
use crate::models::nhl_franchise::NhlFranchise;
use crate::models::nhl_season::NhlSeason;
use crate::models::nhl_team::NhlTeam;
use crate::serde_helpers::try_get;
use crate::sqlx_operation_with_retries;

pub struct NhlStatsApi {
    pub client: reqwest::Client,
    pub base_url: String,
}
impl std::fmt::Debug for NhlStatsApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only print the base_url, not the whole client struct
        f.debug_struct("NhlStatsApi")
            .field("base_url", &self.base_url)
            .finish()
    }
}
impl CacheableApi for NhlStatsApi {
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl NhlStatsApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.nhle.com/stats/rest/en".to_string(),
        }
    }

    pub async fn get_nhl_seasons(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Vec<NhlSeason>, lp_error::LPError> {
        // query nhl_season database to see if the desired data is already present
        let seasons: Vec<NhlSeason> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlSeason>("SELECT * FROM nhl_season")
                .fetch_all(pool)
                .await
        )
        .await?;
        if !seasons.is_empty() {
            tracing::info!(
                "Returning {} cached NHL seasons from database.",
                seasons.len()
            );
            return Ok(seasons);
        }

        // construct endpoint url
        let endpoint = format!("{}/season", self.base_url);
        tracing::info!(endpoint = %endpoint, "No cached seasons found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let data = try_get("data", &raw_json, &endpoint)?;

        // parse data field into an array of nhl seasons
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;
        let mut seasons = Vec::with_capacity(data_array.len());

        // for each item in the seasons array, serde the json object into the NhlSeason struct
        // and add the raw_json and api_cache_endpoint fields that are not present in the api response
        for item in data_array {
            tracing::debug!(item = ?item);
            let mut season: NhlSeason = serde_json::from_value(item.clone())?;
            season.raw_json = Some(item.clone());
            season.api_cache_endpoint = Some(endpoint.clone());
            seasons.push(season);
        }

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting seasons into database...");
        let upserts = seasons.iter().map(|season| season.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed seasons
        for (season, result) in seasons.iter().zip(upsert_results.iter()) {
            if let Err(e) = result {
                tracing::warn!(
                    season_id = season.id,
                    error = ?e,
                    "Failed to upsert NHL season"
                );
            }
        }

        tracing::info!(
            "Upserted {} NHL seasons into database. Now returning them.",
            seasons.len()
        );

        Ok(seasons)
    }

    pub async fn get_nhl_teams(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Vec<NhlTeam>, lp_error::LPError> {
        // query nhl_team database to see if the desired data is already present
        let teams: Vec<NhlTeam> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlTeam>("SELECT * FROM nhl_team")
                .fetch_all(pool)
                .await
        )
        .await?;
        if !teams.is_empty() {
            tracing::info!("Returning {} cached NHL teams from database.", teams.len());
            return Ok(teams);
        }

        // construct endpoint url
        let endpoint = format!("{}/team", self.base_url);
        tracing::info!(endpoint = %endpoint, "No cached teams found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let data = try_get("data", &raw_json, &endpoint)?;

        // parse data field into an array of nhl teams
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;
        let mut teams = Vec::with_capacity(data_array.len());

        // for each item in the teams array, serde the json object into the NhlTeam struct and add
        // the raw_json and api_cache_endpoint fields that are not present in the api response
        for item in data_array {
            tracing::debug!(item = ?item);
            let mut team: NhlTeam = serde_json::from_value(item.clone())?;
            team.raw_json = Some(item.clone());
            team.api_cache_endpoint = Some(endpoint.clone());
            teams.push(team);
        }

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting teams into database...");
        let upserts = teams.iter().map(|season| season.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed teams
        for (team, result) in teams.iter().zip(upsert_results.iter()) {
            if let Err(e) = result {
                tracing::warn!(
                    team_id = team.id,
                    error = ?e,
                    "Failed to upsert NHL season"
                );
            }
        }

        tracing::info!(
            "Upserted {} NHL teams into database. Now returning them.",
            teams.len()
        );

        Ok(teams)
    }

    pub async fn get_nhl_franchises(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Vec<NhlFranchise>, lp_error::LPError> {
        // query nhl_franchise database to see if the desired data is already present
        let franchises: Vec<NhlFranchise> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlFranchise>("SELECT * FROM nhl_franchise")
                .fetch_all(pool)
                .await
        )
        .await?;
        if !franchises.is_empty() {
            tracing::info!(
                "Returning {} cached NHL franchises from database.",
                franchises.len()
            );
            return Ok(franchises);
        }

        // construct endpoint url
        let endpoint = format!("{}/franchise", self.base_url);
        tracing::info!(endpoint = %endpoint, "No cached franchises found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let data = try_get("data", &raw_json, &endpoint)?;

        // parse data field into an array of nhl franchises
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;
        let mut franchises = Vec::with_capacity(data_array.len());

        // for each item in the franchises array, serde the json object into the NhlFranchise struct
        // and add raw_json and api_cache_endpoint fields that are not present in the api response
        for item in data_array {
            tracing::debug!(item = ?item);
            let mut franchise: NhlFranchise = serde_json::from_value(item.clone())?;
            franchise.raw_json = Some(item.clone());
            franchise.api_cache_endpoint = Some(endpoint.clone());
            franchises.push(franchise);
        }

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting franchise into database...");
        let upserts = franchises.iter().map(|season| season.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed franchises
        for (franchise, result) in franchises.iter().zip(upsert_results.iter()) {
            if let Err(e) = result {
                tracing::warn!(
                    franchise_id = franchise.id,
                    error = ?e,
                    "Failed to upsert NHL season"
                );
            }
        }

        tracing::info!(
            "Upserted {} NHL franchise into database. Now returning them.",
            franchises.len()
        );

        Ok(franchises)
    }
}
