use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::super::primary_key::*;
use crate::{
    bind,
    common::{
        db::{DbContext, DbEntity, RelationshipIntegrity, StaticPgQuery, StaticPgQueryAs},
        errors::LPError,
        models::traits::{DbStruct, IntoDbStruct},
    },
    data_sources::models::{LocalizedNameJson, NhlSeasonContext},
    impl_has_type_name, impl_pk_debug, verify_fk,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffBracketJson {
    pub bracket_logo: String,
    pub bracket_logo_fr: String,
    pub series: Vec<NhlPlayoffBracketSeriesJson>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffBracketTeamJson {
    pub id: i32,
    pub abbrev: String,
    pub name: Option<LocalizedNameJson>,
    pub common_name: LocalizedNameJson,
    pub place_name_with_preposition: LocalizedNameJson,
    pub logo: String,
    pub dark_logo: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffBracketSeriesJson {
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
    pub top_seed_team: NhlPlayoffBracketTeamJson,
    pub bottom_seed_team: NhlPlayoffBracketTeamJson,
}
impl IntoDbStruct for NhlPlayoffBracketSeriesJson {
    type DbStruct = NhlPlayoffBracketSeries;
    type Context = NhlSeasonContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlPlayoffBracketSeriesJson {
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
        let NhlSeasonContext {
            season_id,
            endpoint,
            raw_json,
        } = context;
        let NhlPlayoffBracketTeamJson {
            id: top_seed_team_id,
            abbrev: top_seed_team_abbrev,
            name: top_seed_team_name,
            common_name: top_seed_team_common_name,
            place_name_with_preposition: top_seed_team_place_name_with_preposition,
            logo: top_seed_team_logo,
            dark_logo: top_seed_team_dark_logo,
        } = top_seed_team;
        let NhlPlayoffBracketTeamJson {
            id: bottom_seed_team_id,
            abbrev: bottom_seed_team_abbrev,
            name: bottom_seed_team_name,
            common_name: bottom_seed_team_common_name,
            place_name_with_preposition: bottom_seed_team_place_name_with_preposition,
            logo: bottom_seed_team_logo,
            dark_logo: bottom_seed_team_dark_logo,
        } = bottom_seed_team;
        NhlPlayoffBracketSeries {
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
            top_seed_team_name: top_seed_team_name.map(|name| name.best_str()),
            top_seed_team_common_name: top_seed_team_common_name.best_str(),
            top_seed_team_place_name_with_preposition: top_seed_team_place_name_with_preposition
                .best_str(),
            top_seed_team_logo,
            top_seed_team_dark_logo,
            bottom_seed_team_id,
            bottom_seed_team_abbrev,
            bottom_seed_team_name: bottom_seed_team_name.map(|name| name.best_str()),
            bottom_seed_team_common_name: bottom_seed_team_common_name.best_str(),
            bottom_seed_team_place_name_with_preposition:
                bottom_seed_team_place_name_with_preposition.best_str(),
            bottom_seed_team_logo,
            bottom_seed_team_dark_logo,
            endpoint,
            raw_json,
        }
    }
}
#[derive(Clone, FromRow)]
pub struct NhlPlayoffBracketSeries {
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
    pub top_seed_team_name: Option<String>,
    pub top_seed_team_common_name: String,
    pub top_seed_team_place_name_with_preposition: String,
    pub top_seed_team_logo: String,
    pub top_seed_team_dark_logo: String,
    pub bottom_seed_team_id: i32,
    pub bottom_seed_team_abbrev: String,
    pub bottom_seed_team_name: Option<String>,
    pub bottom_seed_team_common_name: String,
    pub bottom_seed_team_place_name_with_preposition: String,
    pub bottom_seed_team_logo: String,
    pub bottom_seed_team_dark_logo: String,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
}
impl DbStruct for NhlPlayoffBracketSeries {
    type IntoDbStruct = NhlPlayoffBracketSeriesJson;
}
#[async_trait]
impl DbEntity for NhlPlayoffBracketSeries {
    type Pk = NhlPrimaryKey;

    fn pk(&self) -> Self::Pk {
        Self::Pk::PlayoffBracketSeries(NhlPlayoffBracketSeriesKey {
            season_id: self.season_id,
            series_letter: self.series_letter.clone(),
        })
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>(
            "SELECT 'nhl_playoff_bracket_series' AS table_name, season_id, series_letter from nhl_playoff_bracket_series",
        )
    }

    fn foreign_keys(&self) -> Vec<Self::Pk> {
        vec![
            Self::Pk::api_cache(&self.endpoint),
            Self::Pk::season(self.season_id),
            Self::Pk::team(self.top_seed_team_id),
            Self::Pk::team(self.bottom_seed_team_id),
        ]
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_playoff_bracket_series (
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
                                        raw_json,
                                        endpoint
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
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#
            ),
            self.season_id,
            self.series_letter,
            self.series_url,
            self.series_title,
            self.series_abbrev,
            self.playoff_round,
            self.top_seed_rank,
            self.top_seed_rank_abbrev,
            self.top_seed_wins,
            self.bottom_seed_rank,
            self.bottom_seed_rank_abbrev,
            self.bottom_seed_wins,
            self.winning_team_id,
            self.losing_team_id,
            self.top_seed_team_id,
            self.top_seed_team_abbrev,
            self.top_seed_team_name,
            self.top_seed_team_common_name,
            self.top_seed_team_place_name_with_preposition,
            self.top_seed_team_logo,
            self.top_seed_team_dark_logo,
            self.bottom_seed_team_id,
            self.bottom_seed_team_abbrev,
            self.bottom_seed_team_name,
            self.bottom_seed_team_common_name,
            self.bottom_seed_team_place_name_with_preposition,
            self.bottom_seed_team_logo,
            self.bottom_seed_team_dark_logo,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlPlayoffBracketJson);
impl_has_type_name!(NhlPlayoffBracketSeriesJson);
impl_has_type_name!(NhlPlayoffBracketSeries);
impl_pk_debug!(NhlPlayoffBracketSeries);
