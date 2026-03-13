use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::super::primary_key::*;
use crate::{
    bind,
    common::{
        db::{CacheKey, DbEntity, StaticPgQuery, StaticPgQueryAs},
        models::traits::IntoDbStruct,
    },
    data_sources::models::{
        LocalizedNameJson, LocalizedNameJsonExt, NhlDefaultContext, NhlPlayoffSeriesGameJson,
    },
    impl_has_type_name, impl_pk_debug,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeriesTeamJson {
    pub id: i32,
    pub name: LocalizedNameJson,
    pub abbrev: String,
    pub place_name: Option<LocalizedNameJson>,
    pub place_name_with_preposition: Option<LocalizedNameJson>,
    pub conference: Option<NhlConferenceJson>,
    pub record: String,
    pub series_wins: i32,
    pub division_abbrev: Option<String>,
    pub seed: i32,
    pub logo: Option<String>,
    pub dark_logo: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlConferenceJson {
    pub name: Option<String>,
    pub abbrev: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlPlayoffSeriesJson {
    pub round: i32,
    pub round_abbrev: String,
    pub round_label: String,
    pub series_letter: String,
    pub series_logo: Option<String>,
    pub series_logo_fr: Option<String>,
    pub needed_to_win: i32,
    pub length: i32,
    pub bottom_seed_team: NhlPlayoffSeriesTeamJson,
    pub top_seed_team: NhlPlayoffSeriesTeamJson,
    pub games: Vec<NhlPlayoffSeriesGameJson>,
    pub full_coverage_url: Option<LocalizedNameJson>,
}
impl IntoDbStruct for NhlPlayoffSeriesJson {
    type DbStruct = NhlPlayoffSeries;
    type Context = NhlDefaultContext;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlPlayoffSeriesJson {
            round_abbrev,
            round,
            round_label,
            series_letter,
            series_logo,
            series_logo_fr,
            needed_to_win,
            length,
            bottom_seed_team,
            top_seed_team,
            games,
            full_coverage_url,
        } = self;
        let NhlDefaultContext { endpoint, raw_json } = context;
        let NhlPlayoffSeriesTeamJson {
            id: bottom_seed_team_id,
            name: bottom_seed_team_name,
            abbrev: bottom_seed_team_abbrev,
            place_name: bottom_seed_team_place_name,
            place_name_with_preposition: bottom_seed_team_place_name_with_preposition,
            conference: bottom_seed_team_conference,
            record: bottom_seed_team_record,
            series_wins: bottom_seed_team_series_wins,
            division_abbrev: bottom_seed_team_division_abbrev,
            seed: bottom_seed_team_seed,
            logo: bottom_seed_team_logo,
            dark_logo: bottom_seed_team_dark_logo,
        } = bottom_seed_team;
        let NhlConferenceJson {
            name: bottom_seed_team_conference_name,
            abbrev: bottom_seed_team_conference_abbrev,
        } = bottom_seed_team_conference.unwrap_or(NhlConferenceJson {
            name: None,
            abbrev: None,
        });
        let NhlPlayoffSeriesTeamJson {
            id: top_seed_team_id,
            name: top_seed_team_name,
            abbrev: top_seed_team_abbrev,
            place_name: top_seed_team_place_name,
            place_name_with_preposition: top_seed_team_place_name_with_preposition,
            conference: top_seed_team_conference,
            record: top_seed_team_record,
            series_wins: top_seed_team_series_wins,
            division_abbrev: top_seed_team_division_abbrev,
            seed: top_seed_team_seed,
            logo: top_seed_team_logo,
            dark_logo: top_seed_team_dark_logo,
        } = top_seed_team;
        let NhlConferenceJson {
            name: top_seed_team_conference_name,
            abbrev: top_seed_team_conference_abbrev,
        } = top_seed_team_conference.unwrap_or(NhlConferenceJson {
            name: None,
            abbrev: None,
        });
        let season_id: i32 = games[0].season;
        let game_ids: Vec<i32> = games.iter().map(|game| game.id).collect();
        let series_length: i32 = game_ids
            .len()
            .try_into()
            .expect("Series length should fit in i32");
        NhlPlayoffSeries {
            season_id,
            round,
            round_abbrev,
            round_label,
            series_letter,
            series_logo,
            series_logo_fr,
            needed_to_win,
            length,
            bottom_seed_team_id,
            bottom_seed_team_name: bottom_seed_team_name.best_str(),
            bottom_seed_team_abbrev,
            bottom_seed_team_place_name: bottom_seed_team_place_name.best_str_or_none(),
            bottom_seed_team_place_name_with_preposition:
                bottom_seed_team_place_name_with_preposition.best_str_or_none(),
            bottom_seed_team_conference_name,
            bottom_seed_team_conference_abbrev,
            bottom_seed_team_record,
            bottom_seed_team_series_wins,
            bottom_seed_team_division_abbrev,
            bottom_seed_team_seed,
            bottom_seed_team_logo,
            bottom_seed_team_dark_logo,
            top_seed_team_id,
            top_seed_team_name: top_seed_team_name.best_str(),
            top_seed_team_abbrev,
            top_seed_team_place_name: top_seed_team_place_name.best_str_or_none(),
            top_seed_team_place_name_with_preposition: top_seed_team_place_name_with_preposition
                .best_str_or_none(),
            top_seed_team_conference_name,
            top_seed_team_conference_abbrev,
            top_seed_team_record,
            top_seed_team_series_wins,
            top_seed_team_division_abbrev,
            top_seed_team_seed,
            top_seed_team_logo,
            top_seed_team_dark_logo,
            series_length,
            game_ids,
            full_coverage_url: full_coverage_url.best_str_or_none(),
            raw_json,
            endpoint,
        }
    }
}

#[derive(Clone, FromRow)]
pub struct NhlPlayoffSeries {
    pub season_id: i32,
    pub round: i32,
    pub round_abbrev: String,
    pub round_label: String,
    pub series_letter: String,
    pub series_logo: Option<String>,
    pub series_logo_fr: Option<String>,
    pub needed_to_win: i32,
    pub length: i32,
    pub bottom_seed_team_id: i32,
    pub bottom_seed_team_name: String,
    pub bottom_seed_team_abbrev: String,
    pub bottom_seed_team_place_name: Option<String>,
    pub bottom_seed_team_place_name_with_preposition: Option<String>,
    pub bottom_seed_team_conference_name: Option<String>,
    pub bottom_seed_team_conference_abbrev: Option<String>,
    pub bottom_seed_team_record: String,
    pub bottom_seed_team_series_wins: i32,
    pub bottom_seed_team_division_abbrev: Option<String>,
    pub bottom_seed_team_seed: i32,
    pub bottom_seed_team_logo: Option<String>,
    pub bottom_seed_team_dark_logo: Option<String>,
    pub top_seed_team_id: i32,
    pub top_seed_team_name: String,
    pub top_seed_team_abbrev: String,
    pub top_seed_team_place_name: Option<String>,
    pub top_seed_team_place_name_with_preposition: Option<String>,
    pub top_seed_team_conference_name: Option<String>,
    pub top_seed_team_conference_abbrev: Option<String>,
    pub top_seed_team_record: String,
    pub top_seed_team_series_wins: i32,
    pub top_seed_team_division_abbrev: Option<String>,
    pub top_seed_team_seed: i32,
    pub top_seed_team_logo: Option<String>,
    pub top_seed_team_dark_logo: Option<String>,
    pub series_length: i32,
    pub game_ids: Vec<i32>,
    pub full_coverage_url: Option<String>,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
}
#[async_trait]
impl DbEntity for NhlPlayoffSeries {
    type Pk = NhlPlayoffSeriesKey;

    fn pk(&self) -> Self::Pk {
        NhlPlayoffSeriesKey {
            season_id: self.season_id,
            series_letter: self.series_letter.clone(),
        }
    }

    fn select_key_query() -> StaticPgQueryAs<Self::Pk> {
        sqlx::query_as::<_, Self::Pk>("SELECT season_id, series_letter from nhl_playoff_series")
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        vec![
            CacheKey {
                source: "api_cache",
                table: "api_cache",
                id: self.endpoint.clone(),
            },
            CacheKey {
                source: "nhl",
                table: "season",
                id: self.season_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "team",
                id: self.top_seed_team_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "team",
                id: self.bottom_seed_team_id.to_string(),
            },
            CacheKey {
                source: "nhl",
                table: "playoff_bracket_series",
                id: format!("{}:{}", self.season_id, self.series_letter),
            },
        ]
    }

    fn upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_playoff_series (
                                    season_id,
                                    round,
                                    round_abbrev,
                                    round_label,
                                    series_letter,
                                    series_logo,
                                    series_logo_fr,
                                    needed_to_win,
                                    length,
                                    bottom_seed_team_id,
                                    bottom_seed_team_name,
                                    bottom_seed_team_abbrev,
                                    bottom_seed_team_place_name,
                                    bottom_seed_team_place_name_with_preposition,
                                    bottom_seed_team_conference_name,
                                    bottom_seed_team_conference_abbrev,
                                    bottom_seed_team_record,
                                    bottom_seed_team_series_wins,
                                    bottom_seed_team_division_abbrev,
                                    bottom_seed_team_seed,
                                    bottom_seed_team_logo,
                                    bottom_seed_team_dark_logo,
                                    top_seed_team_id,
                                    top_seed_team_name,
                                    top_seed_team_abbrev,
                                    top_seed_team_place_name,
                                    top_seed_team_place_name_with_preposition,
                                    top_seed_team_conference_name,
                                    top_seed_team_conference_abbrev,
                                    top_seed_team_record,
                                    top_seed_team_series_wins,
                                    top_seed_team_division_abbrev,
                                    top_seed_team_seed,
                                    top_seed_team_logo,
                                    top_seed_team_dark_logo,
                                    series_length,
                                    game_ids,
                                    full_coverage_url,
                                    raw_json,
                                    endpoint
                                ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,
                                        $11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                                        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                                        $31,$32,$33,$34,$35,$36,$37,$38,$39,$40)
                                ON CONFLICT (season_id, series_letter) DO UPDATE SET
                                    season_id = EXCLUDED.season_id,
                                    round = EXCLUDED.round,
                                    round_abbrev = EXCLUDED.round_abbrev,
                                    round_label = EXCLUDED.round_label,
                                    series_letter = EXCLUDED.series_letter,
                                    series_logo = EXCLUDED.series_logo,
                                    series_logo_fr = EXCLUDED.series_logo_fr,
                                    needed_to_win = EXCLUDED.needed_to_win,
                                    length = EXCLUDED.length,
                                    bottom_seed_team_id = EXCLUDED.bottom_seed_team_id,
                                    bottom_seed_team_name = EXCLUDED.bottom_seed_team_name,
                                    bottom_seed_team_abbrev = EXCLUDED.bottom_seed_team_abbrev,
                                    bottom_seed_team_place_name = EXCLUDED.bottom_seed_team_place_name,
                                    bottom_seed_team_place_name_with_preposition = EXCLUDED.bottom_seed_team_place_name_with_preposition,
                                    bottom_seed_team_conference_name = EXCLUDED.bottom_seed_team_conference_name,
                                    bottom_seed_team_conference_abbrev = EXCLUDED.bottom_seed_team_conference_abbrev,
                                    bottom_seed_team_record = EXCLUDED.bottom_seed_team_record,
                                    bottom_seed_team_series_wins = EXCLUDED.bottom_seed_team_series_wins,
                                    bottom_seed_team_division_abbrev = EXCLUDED.bottom_seed_team_division_abbrev,
                                    bottom_seed_team_seed = EXCLUDED.bottom_seed_team_seed,
                                    bottom_seed_team_logo = EXCLUDED.bottom_seed_team_logo,
                                    bottom_seed_team_dark_logo = EXCLUDED.bottom_seed_team_dark_logo,
                                    top_seed_team_id = EXCLUDED.top_seed_team_id,
                                    top_seed_team_name = EXCLUDED.top_seed_team_name,
                                    top_seed_team_abbrev = EXCLUDED.top_seed_team_abbrev,
                                    top_seed_team_place_name = EXCLUDED.top_seed_team_place_name,
                                    top_seed_team_place_name_with_preposition = EXCLUDED.top_seed_team_place_name_with_preposition,
                                    top_seed_team_conference_name = EXCLUDED.top_seed_team_conference_name,
                                    top_seed_team_conference_abbrev = EXCLUDED.top_seed_team_conference_abbrev,
                                    top_seed_team_record = EXCLUDED.top_seed_team_record,
                                    top_seed_team_series_wins = EXCLUDED.top_seed_team_series_wins,
                                    top_seed_team_division_abbrev = EXCLUDED.top_seed_team_division_abbrev,
                                    top_seed_team_seed = EXCLUDED.top_seed_team_seed,
                                    top_seed_team_logo = EXCLUDED.top_seed_team_logo,
                                    top_seed_team_dark_logo = EXCLUDED.top_seed_team_dark_logo,
                                    series_length = EXCLUDED.series_length,
                                    game_ids = EXCLUDED.game_ids,
                                    full_coverage_url = EXCLUDED.full_coverage_url,
                                    raw_json = EXCLUDED.raw_json,
                                    endpoint = EXCLUDED.endpoint,
                                    last_updated = now()
                "#
            ),
            self.season_id,
            self.round,
            self.round_abbrev,
            self.round_label,
            self.series_letter,
            self.series_logo,
            self.series_logo_fr,
            self.needed_to_win,
            self.length,
            self.bottom_seed_team_id,
            self.bottom_seed_team_name,
            self.bottom_seed_team_abbrev,
            self.bottom_seed_team_place_name,
            self.bottom_seed_team_place_name_with_preposition,
            self.bottom_seed_team_conference_name,
            self.bottom_seed_team_conference_abbrev,
            self.bottom_seed_team_record,
            self.bottom_seed_team_series_wins,
            self.bottom_seed_team_division_abbrev,
            self.bottom_seed_team_seed,
            self.bottom_seed_team_logo,
            self.bottom_seed_team_dark_logo,
            self.top_seed_team_id,
            self.top_seed_team_name,
            self.top_seed_team_abbrev,
            self.top_seed_team_place_name,
            self.top_seed_team_place_name_with_preposition,
            self.top_seed_team_conference_name,
            self.top_seed_team_conference_abbrev,
            self.top_seed_team_record,
            self.top_seed_team_series_wins,
            self.top_seed_team_division_abbrev,
            self.top_seed_team_seed,
            self.top_seed_team_logo,
            self.top_seed_team_dark_logo,
            self.series_length,
            self.game_ids,
            self.full_coverage_url,
            self.raw_json,
            self.endpoint
        )
    }
}

impl_has_type_name!(NhlPlayoffSeriesJson);
impl_has_type_name!(NhlPlayoffSeries);
impl_pk_debug!(NhlPlayoffSeries);
