use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::{
    DefaultNhlContext, DefendingSide, GameNhlContext, GameType, LocalizedNameJson, NhlGameKey,
    NhlPlayerKey, NhlPlayoffSeriesKey, NhlPrimaryKey, NhlRosterSpotJson, NhlSeason, NhlSeasonKey,
    NhlTeam, NhlTeamKey, PeriodDescriptorJson, PeriodTypeJson,
};
use crate::{
    bind,
    common::{
        db::{DbContext, DbEntity, PrimaryKey, RelationshipIntegrity, StaticPgQuery},
        errors::LPError,
        models::{
            ApiCache, ApiCacheKey,
            traits::{DbStruct, IntoDbStruct},
        },
        serde_helpers::JsonExt,
    },
    impl_has_type_name, make_deserialize_key_to_type, make_deserialize_to_type,
    sqlx_operation_with_retries, verify_fk,
};

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
#[derive(Clone, Debug, FromRow)]
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
impl DbStruct for NhlPlayoffSeries {
    type IntoDbStruct = NhlPlayoffSeriesJson;

    fn create_context_struct(&self) -> <<Self as DbStruct>::IntoDbStruct as IntoDbStruct>::Context {
        DefaultNhlContext {
            endpoint: self.endpoint.clone(),
            raw_json: self.raw_json.clone(),
        }
    }
}
#[async_trait]
impl DbEntity for NhlPlayoffSeries {
    type Pk = NhlPrimaryKey;

    fn id(&self) -> Self::Pk {
        Self::Pk::PlayoffSeries(NhlPlayoffSeriesKey {
            season_id: self.season_id,
            series_letter: self.series_letter.clone(),
        })
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        let mut missing: Vec<Self::Pk> = vec![];

        verify_fk!(missing, db_context, Self::Pk::season(self.season_id));
        verify_fk!(missing, db_context, Self::Pk::team(self.top_seed_team_id));
        verify_fk!(
            missing,
            db_context,
            Self::Pk::team(self.bottom_seed_team_id)
        );
        verify_fk!(missing, db_context, Self::Pk::api_cache(&self.endpoint));

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }

    fn create_upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO nhl_playoff_series (
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
            self.endpoint,
            self.raw_json,
        )
    }
}

impl_has_type_name!(NhlPlayoffSeriesJson);
impl_has_type_name!(NhlPlayoffSeries);
impl_has_type_name!(NhlPlayoffBracketJson);
