use futures::future::join_all;

use crate::lp_error;

use crate::api::cacheable::CacheableApi;
use crate::models::nhl_franchise::NhlFranchise;
use crate::models::nhl_season::NhlSeason;
use crate::models::nhl_team::NhlTeam;
use crate::serde_helpers::JsonExt;
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

    #[tracing::instrument(skip(pool))]
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
        tracing::info!(endpoint = %&endpoint, "No cached seasons found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let Some(data) = raw_json.get_key_as_logged::<serde_json::Value>("data") else {
            return Err(lp_error::LPError::ApiCustom(format!(
                "Missing 'data' field in response from {endpoint}"
            )));
        };

        // parse data field into an array of nhl seasons
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;

        // serde each json object in data_array into an NhlSeason struct and add the raw_json
        // and api_cache_endpoint fields that are not present in the api
        let seasons: Vec<NhlSeason> = data_array
            .iter()
            .map(|item| {
                tracing::debug!(item = ?item);
                let mut season: NhlSeason = serde_json::from_value(item.clone())?;
                season.raw_json = Some(item.clone());
                season.api_cache_endpoint = Some(endpoint.clone());
                Ok(season)
            })
            .collect::<Result<Vec<_>, serde_json::Error>>()?;

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting seasons into database...");
        let upserts = seasons.iter().map(|season| season.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed seasons
        seasons
            .iter()
            .zip(upsert_results)
            .for_each(|(season, result)| {
                if let Err(e) = result {
                    tracing::warn!(
                        season_id = season.id,
                        error = ?e,
                        "Failed to upsert NHL season"
                    );
                }
            });

        tracing::info!(
            "Upserted {} NHL seasons into database. Now returning them.",
            seasons.len()
        );

        Ok(seasons)
    }

    #[tracing::instrument(skip(pool))]
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
        tracing::info!(endpoint = %&endpoint, "No cached teams found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let Some(data) = raw_json.get_key_as_logged::<serde_json::Value>("data") else {
            return Err(lp_error::LPError::ApiCustom(format!(
                "Missing 'data' field in response from {endpoint}"
            )));
        };

        // parse data field into an array of nhl teams
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;
        let teams = data_array
            .iter()
            .map(|item| {
                tracing::debug!(item = ?item);
                let mut team: NhlTeam = serde_json::from_value(item.clone())?;
                team.raw_json = Some(item.clone());
                team.api_cache_endpoint = Some(endpoint.clone());
                Ok(team)
            })
            .collect::<Result<Vec<_>, serde_json::Error>>()?;

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting teams into database...");
        let upserts = teams.iter().map(|team| team.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed teams
        teams.iter().zip(upsert_results).for_each(|(team, result)| {
            if let Err(e) = result {
                tracing::warn!(
                    team_id = team.id,
                    error = ?e,
                    "Failed to upsert NHL season"
                );
            }
        });

        tracing::info!(
            "Upserted {} NHL teams into database. Now returning them.",
            teams.len()
        );

        Ok(teams)
    }

    #[tracing::instrument(skip(pool))]
    pub async fn get_nhl_team(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        team_id: i32,
    ) -> Result<NhlTeam, lp_error::LPError> {
        // query nhl_team database to see if the desired data is already present
        let team: Option<NhlTeam> = sqlx_operation_with_retries!(
            sqlx::query_as::<sqlx::Postgres, NhlTeam>("SELECT * FROM nhl_team WHERE id = $1")
                .bind(&team_id)
                .fetch_optional(pool)
                .await
        )
        .await?;
        if team.is_some() {
            tracing::debug!("NHL team with ID {team_id} found in lp database.");
            return Ok(team.unwrap());
        }

        // construct endpoint url
        let endpoint = format!("{}/team/id/{team_id}", self.base_url);
        tracing::info!(endpoint = %&endpoint, "Team not found in lp database, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let json_value: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let Some(data) = json_value.get_key_as_logged::<serde_json::Value>("data") else {
            return Err(lp_error::LPError::ApiCustom(format!(
                "Missing 'data' field in response from {endpoint}"
            )));
        };

        // parse data field into an array of nhl teams
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;

        // response should have just one team, so attempt to take the first team from the array
        let team_json = match data_array.first() {
            Some(team) => team,
            None => {
                return Err(lp_error::LPError::ApiCustom(
                    "API returned no team with id {team_id}.".to_string(),
                ));
            }
        };
        tracing::debug!(team_json = ?team_json);
        let mut team: NhlTeam = serde_json::from_value(team_json.clone())?;
        team.raw_json = Some(team_json.clone());
        team.api_cache_endpoint = Some(endpoint);

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting team into database...");
        match team.upsert(&pool).await {
            Ok(_) => (),
            Err(e) => tracing::warn!(team = ?team, "Failed to upsert NHL team: {e}"),
        }

        tracing::info!("Upserted NHL team with id {team_id} into database. Now returning it.");

        Ok(team)
    }

    #[tracing::instrument(skip(pool))]
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
        tracing::info!(endpoint = %&endpoint, "No cached franchises found, fetching from API");

        // get or cache contents of endpoint and serde the response into json
        let raw_data = self.get_or_cache_endpoint(pool, &endpoint).await?;
        let raw_json: serde_json::Value = serde_json::from_str(&raw_data)?;

        // retrieve the data field from the json
        let Some(data) = raw_json.get_key_as_logged::<serde_json::Value>("data") else {
            return Err(lp_error::LPError::ApiCustom(format!(
                "Missing 'data' field in response from {endpoint}"
            )));
        };

        // parse data field into an array of nhl franchise structs
        let data_array = data.as_array().ok_or_else(|| {
            lp_error::LPError::ApiCustom("Expected 'data' to be an array".to_string())
        })?;

        let franchises = data_array
            .iter()
            .map(|item| {
                tracing::debug!(item = ?item);
                let mut franchise: NhlFranchise = serde_json::from_value(item.clone())?;
                franchise.raw_json = Some(item.clone());
                franchise.api_cache_endpoint = Some(endpoint.clone());
                Ok(franchise)
            })
            .collect::<Result<Vec<_>, serde_json::Error>>()?;

        // build the upsert futures, run them concurrently, and propagate any errors
        tracing::info!("Upserting franchise into database...");
        let upserts = franchises.iter().map(|franchise| franchise.upsert(&pool));
        let upsert_results = join_all(upserts).await;

        // log any failed franchises
        franchises
            .iter()
            .zip(upsert_results)
            .for_each(|(franchise, result)| {
                if let Err(e) = result {
                    tracing::warn!(
                        franchise_id = franchise.id,
                        error = ?e,
                        "Failed to upsert NHL season"
                    );
                }
            });

        tracing::info!(
            "Upserted {} NHL franchise into database. Now returning them.",
            franchises.len()
        );

        Ok(franchises)
    }
}
