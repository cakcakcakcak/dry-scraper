use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::LPError;
use crate::db::{DbPool, Persistable};
use crate::models::nhl::DefaultNhlContext;
use crate::models::traits::{DbStruct, IntoDbStruct};

use crate::impl_has_type_name;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffBracketJson {
    pub bracket_logo: String,
    pub bracket_logo_fr: String,
    pub series: Vec<NhlPlayoffSeriesJson>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlSeedTeamJson {
    pub id: i32,
    pub abbrev: String,
    pub name: String,
    pub common_name: String,
    pub place_name_with_preposition: String,
    pub logo: String,
    pub dark_logo: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeriesJson {
    pub series_letter: String,
    pub series_url: String,
    pub series_title: String,
    pub series_abbrev: String,
    pub playoff_round: i32,
    pub top_seed_rank: i32,
    pub top_seed_rank_abbrev: String,
    pub top_seed_wins: i32,
    pub bottom_seed_rank: i32,
    pub bottom_seed_rank_abbrev: String,
    pub bottom_seed_wins: i32,
    pub winning_team_id: Option<i32>, // as far as i can tell, only optional because of the 1919 SCF, cancelled due to spanish flu
    pub losing_team_id: Option<i32>,
    pub top_seed_team: NhlSeedTeamJson,
    pub bottom_seed_team: NhlSeedTeamJson,
}
impl IntoDbStruct for NhlPlayoffSeriesJson {
    type DbStruct = NhlPlayoffSeries;
    type Context = DefaultNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlPlayoffSeriesJson {
            series_letter,
            series_url,
            series_title,
            series_abbrev,
            playoff_round,
            top_seed_rank,
            top_seed_rank_abbrev,
            top_seed_wins,
            bottom_seed_rank,
            bottom_seed_rank_abbrev,
            bottom_seed_wins,
            winning_team_id,
            losing_team_id,
            top_seed_team,
            bottom_seed_team,
        } = self;
        let DefaultNhlContext { endpoint, raw_json } = context;
        let season_id = series_url.split("/").collect::<Vec<&str>>()[3]
            .parse::<i32>()
            .unwrap();
        let NhlSeedTeamJson {
            id: top_seed_team_id,
            abbrev: top_seed_team_abbrev,
            name: top_seed_team_name,
            common_name: top_seed_team_common_name,
            place_name_with_preposition: top_seed_team_place_name_with_preposition,
            logo: top_seed_team_logo,
            dark_logo: top_seed_team_dark_logo,
        } = top_seed_team;
        let NhlSeedTeamJson {
            id: bottom_seed_team_id,
            abbrev: bottom_seed_team_abbrev,
            name: bottom_seed_team_name,
            common_name: bottom_seed_team_common_name,
            place_name_with_preposition: bottom_seed_team_place_name_with_preposition,
            logo: bottom_seed_team_logo,
            dark_logo: bottom_seed_team_dark_logo,
        } = bottom_seed_team;
        NhlPlayoffSeries {
            season_id,
            series_letter,
            series_url,
            series_title,
            series_abbrev,
            playoff_round,
            top_seed_rank,
            top_seed_rank_abbrev,
            top_seed_wins,
            bottom_seed_rank,
            bottom_seed_rank_abbrev,
            bottom_seed_wins,
            winning_team_id,
            losing_team_id,
            top_seed_team_id,
            top_seed_team_abbrev,
            top_seed_team_name,
            top_seed_team_common_name,
            top_seed_team_place_name_with_preposition,
            top_seed_team_logo,
            top_seed_team_dark_logo,
            bottom_seed_team_id,
            bottom_seed_team_abbrev,
            bottom_seed_team_name,
            bottom_seed_team_common_name,
            bottom_seed_team_place_name_with_preposition,
            bottom_seed_team_logo,
            bottom_seed_team_dark_logo,
            endpoint,
            raw_json,
            last_updated: None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeries {
    pub season_id: i32,
    pub series_letter: String,
    pub series_url: String,
    pub series_title: String,
    pub series_abbrev: String,
    pub playoff_round: i32,
    pub top_seed_rank: i32,
    pub top_seed_rank_abbrev: String,
    pub top_seed_wins: i32,
    pub bottom_seed_rank: i32,
    pub bottom_seed_rank_abbrev: String,
    pub bottom_seed_wins: i32,
    pub winning_team_id: Option<i32>, // as far as i can tell, only optional because of the 1919 SCF, cancelled due to spanish flu
    pub losing_team_id: Option<i32>,
    pub top_seed_team_id: i32,
    pub top_seed_team_abbrev: String,
    pub top_seed_team_name: String,
    pub top_seed_team_common_name: String,
    pub top_seed_team_place_name_with_preposition: String,
    pub top_seed_team_logo: String,
    pub top_seed_team_dark_logo: String,
    pub bottom_seed_team_id: i32,
    pub bottom_seed_team_abbrev: String,
    pub bottom_seed_team_name: String,
    pub bottom_seed_team_common_name: String,
    pub bottom_seed_team_place_name_with_preposition: String,
    pub bottom_seed_team_logo: String,
    pub bottom_seed_team_dark_logo: String,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlPlayoffSeries {}
#[async_trait]
impl Persistable for NhlPlayoffSeries {
    type Id = (i32, String);

    fn id(&self) -> Self::Id {
        (self.season_id, self.series_letter.clone())
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(
                r#"SELECT * FROM nhl_playoff_series WHERE season_id=$1 AND series_letter=$2"#
            )
            .bind(id.0.clone())
            .bind(id.1.clone())
            .fetch_optional(pool)
            .await
        )
        .await
        .map_err(LPError::from)
    }

    fn create_upsert_query(
        &self,
    ) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(r#"INSERT INTO nhl_playoff_series (
                                        season_id,
                                        series_letter,
                                        series_url,
                                        series_title,
                                        series_abbreviation,
                                        playoff_round,
                                        top_seed_rank,
                                        top_seed_rank_abbreviation,
                                        top_seed_wins,
                                        bottom_seed_rank,
                                        bottom_seed_rank_abbreviation,
                                        bottom_seed_wins,
                                        winning_team_id,
                                        losing_team_id,
                                        top_seed_team_id,
                                        top_seed_team_abbrev,
                                        top_seed_team_name,
                                        top_seed_team_common_name,
                                        top_seed_team_place_name_with_preposition,
                                        top_seed_team_logo,
                                        top_seed_team_dark_logo,
                                        bottom_seed_team_id,
                                        bottom_seed_team_abbrev,
                                        bottom_seed_team_name,
                                        bottom_seed_team_common_name,
                                        bottom_seed_team_place_name_with_preposition,
                                        bottom_seed_team_logo,
                                        bottom_seed_team_dark_logo,
                                        endpoint,
                                        raw_json
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                        $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30)
                                    ON CONFLICT (season_id, series_letter) DO UPDATE SET
                                        series_url = EXCLUDED.series_url,
                                        series_title = EXCLUDED.series_title,
                                        series_abbreviation = EXCLUDED.series_abbreviation,
                                        playoff_round = EXCLUDED.playoff_round,
                                        top_seed_rank = EXCLUDED.top_seed_rank,
                                        top_seed_rank_abbreviation = EXCLUDED.top_seed_rank_abbreviation,
                                        top_seed_wins = EXCLUDED.top_seed_wins,
                                        bottom_seed_rank = EXCLUDED.bottom_seed_rank,
                                        bottom_seed_rank_abbreviation = EXCLUDED.bottom_seed_rank_abbreviation,
                                        bottom_seed_wins = EXCLUDED.bottom_seed_wins,
                                        winning_team_id = EXCLUDED.winning_team_id,
                                        losing_team_id = EXCLUDED.losing_team_id,
                                        top_seed_team_id = EXCLUDED.top_seed_team_id,
                                        top_seed_team_abbrev = EXCLUDED.top_seed_team_abbrev,
                                        top_seed_team_name = EXCLUDED.top_seed_team_name,
                                        top_seed_team_common_name = EXCLUDED.top_seed_team_common_name,
                                        top_seed_team_place_name_with_preposition = EXCLUDED.top_seed_team_place_name_with_preposition,
                                        top_seed_team_logo = EXCLUDED.top_seed_team_logo,
                                        top_seed_team_dark_logo = EXCLUDED.top_seed_team_dark_logo,
                                        bottom_seed_team_id = EXCLUDED.bottom_seed_team_id,
                                        bottom_seed_team_abbrev = EXCLUDED.bottom_seed_team_abbrev,
                                        bottom_seed_team_name = EXCLUDED.bottom_seed_team_name,
                                        bottom_seed_team_common_name = EXCLUDED.bottom_seed_team_common_name,
                                        bottom_seed_team_place_name_with_preposition = EXCLUDED.bottom_seed_team_place_name_with_preposition,
                                        bottom_seed_team_logo = EXCLUDED.bottom_seed_team_logo,
                                        bottom_seed_team_dark_logo = EXCLUDED.bottom_seed_team_dark_logo,
                                        endpoint = EXCLUDED.endpoint,
                                        raw_json = EXCLUDED.raw_json,
                                        last_updated = now()
                                    "#
            )
            .bind(&self.season_id)
            .bind(&self.series_letter)
            .bind(&self.series_url)
            .bind(&self.series_title)
            .bind(&self.series_abbrev)
            .bind(&self.playoff_round)
            .bind(&self.top_seed_rank)
            .bind(&self.top_seed_rank_abbrev)
            .bind(&self.top_seed_wins)
            .bind(&self.bottom_seed_rank)
            .bind(&self.bottom_seed_rank_abbrev)
            .bind(&self.bottom_seed_wins)
            .bind(&self.winning_team_id)
            .bind(&self.losing_team_id)
            .bind(&self.top_seed_team_id)
            .bind(&self.top_seed_team_abbrev)
            .bind(&self.top_seed_team_name)
            .bind(&self.top_seed_team_common_name)
            .bind(&self.top_seed_team_place_name_with_preposition)
            .bind(&self.top_seed_team_logo)
            .bind(&self.top_seed_team_dark_logo)
            .bind(&self.bottom_seed_team_id)
            .bind(&self.bottom_seed_team_abbrev)
            .bind(&self.bottom_seed_team_name)
            .bind(&self.bottom_seed_team_common_name)
            .bind(&self.bottom_seed_team_place_name_with_preposition)
            .bind(&self.bottom_seed_team_logo)
            .bind(&self.bottom_seed_team_dark_logo)
            .bind(&self.endpoint)
            .bind(&self.raw_json)
    }
}

impl_has_type_name!(NhlPlayoffSeriesJson);
impl_has_type_name!(NhlPlayoffSeries);
impl_has_type_name!(NhlPlayoffBracketJson);
