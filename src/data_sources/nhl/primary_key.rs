use std::fmt::Debug;

use async_trait::async_trait;
use sqlx::{FromRow, Row, postgres::PgRow};

use crate::{
    LPError,
    common::{
        any_primary_key::AnyPrimaryKey,
        api::cacheable_api::SimpleApi,
        db::{DbContext, DbEntity, PrimaryKey, StaticPgQuery},
        models::{ApiCache, ApiCacheKey, ItemParsedWithContext},
    },
    data_sources::nhl::{
        api::{NhlApi, NhlStatsApi, NhlWebApi},
        models::{
            NhlFranchise, NhlGame, NhlGameJson, NhlPlay, NhlPlayer, NhlPlayerJson,
            NhlPlayoffSeries, NhlRosterSpot, NhlSeason, NhlTeam, NhlTeamJson,
        },
    },
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NhlPrimaryKey {
    ApiCache(ApiCacheKey),
    Season(NhlSeasonKey),
    Franchise(NhlFranchiseKey),
    Team(NhlTeamKey),
    Player(NhlPlayerKey),
    Game(NhlGameKey),
    RosterSpot(NhlRosterSpotKey),
    Play(NhlPlayKey),
    PlayoffSeries(NhlPlayoffSeriesKey),
}
impl<'r> FromRow<'r, PgRow> for NhlPrimaryKey {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let table_name: &str = row.try_get("table_name")?;
        match table_name {
            "api_cache" => Ok(NhlPrimaryKey::ApiCache(ApiCacheKey::from_row(row)?)),
            "nhl_season" => Ok(NhlPrimaryKey::Season(NhlSeasonKey::from_row(row)?)),
            "nhl_franchise" => Ok(NhlPrimaryKey::Franchise(NhlFranchiseKey::from_row(row)?)),
            "nhl_team" => Ok(NhlPrimaryKey::Team(NhlTeamKey::from_row(row)?)),
            "nhl_player" => Ok(NhlPrimaryKey::Player(NhlPlayerKey::from_row(row)?)),
            "nhl_game" => Ok(NhlPrimaryKey::Game(NhlGameKey::from_row(row)?)),
            "nhl_roster_spot" => Ok(NhlPrimaryKey::RosterSpot(NhlRosterSpotKey::from_row(row)?)),
            "nhl_play" => Ok(NhlPrimaryKey::Play(NhlPlayKey::from_row(row)?)),
            "nhl_playoff_series" => Ok(NhlPrimaryKey::PlayoffSeries(
                NhlPlayoffSeriesKey::from_row(row)?,
            )),
            _ => Err(sqlx::Error::ColumnNotFound(
                "Unknown table `{table_name}`".into(),
            )),
        }
    }
}
#[async_trait]
impl PrimaryKey for NhlPrimaryKey {
    type Api = NhlApi;

    fn any_pk(&self) -> AnyPrimaryKey {
        AnyPrimaryKey::Nhl(self.clone())
    }

    fn create_select_query(&self) -> StaticPgQuery {
        match self {
            NhlPrimaryKey::ApiCache(pk) => pk.create_select_query(),
            NhlPrimaryKey::Season(pk) => pk.create_select_query(),
            NhlPrimaryKey::Franchise(pk) => pk.create_select_query(),
            NhlPrimaryKey::Team(pk) => pk.create_select_query(),
            NhlPrimaryKey::Player(pk) => pk.create_select_query(),
            NhlPrimaryKey::Game(pk) => pk.create_select_query(),
            NhlPrimaryKey::RosterSpot(pk) => pk.create_select_query(),
            NhlPrimaryKey::Play(pk) => pk.create_select_query(),
            NhlPrimaryKey::PlayoffSeries(pk) => pk.create_select_query(),
        }
    }

    async fn upsert_from_api(&self, db_context: &DbContext, api: &NhlApi) -> Result<(), LPError> {
        match self {
            NhlPrimaryKey::ApiCache(pk) => {
                pk.upsert_from_api(
                    db_context,
                    &SimpleApi {
                        client: reqwest::Client::new(),
                    },
                )
                .await
            }
            NhlPrimaryKey::Team(pk) => pk.upsert_from_api(db_context, api).await,
            NhlPrimaryKey::Player(pk) => pk.upsert_from_api(db_context, api).await,
            NhlPrimaryKey::Game(pk) => pk.upsert_from_api(db_context, api).await,
            _ => Ok(()),
        }
    }
}
impl NhlPrimaryKey {
    pub async fn verify_by_key(
        self,
        db_context: &DbContext,
    ) -> Result<Option<NhlPrimaryKey>, LPError> {
        match self {
            NhlPrimaryKey::ApiCache(pk) => match ApiCache::verify_by_key(db_context, pk).await? {
                Some(pk) => Ok(Some(NhlPrimaryKey::ApiCache(pk))),
                None => Ok(None),
            },
            NhlPrimaryKey::Season(_) => NhlSeason::verify_by_key(db_context, self).await,
            NhlPrimaryKey::Franchise(_) => NhlFranchise::verify_by_key(db_context, self).await,
            NhlPrimaryKey::Team(_) => NhlTeam::verify_by_key(db_context, self).await,
            NhlPrimaryKey::Player(_) => NhlPlayer::verify_by_key(db_context, self).await,
            NhlPrimaryKey::Game(_) => NhlGame::verify_by_key(db_context, self).await,
            NhlPrimaryKey::RosterSpot(_) => NhlRosterSpot::verify_by_key(db_context, self).await,
            NhlPrimaryKey::Play(_) => NhlPlay::verify_by_key(db_context, self).await,
            NhlPrimaryKey::PlayoffSeries(_) => {
                NhlPlayoffSeries::verify_by_key(db_context, self).await
            }
        }
    }

    pub fn api_cache<S: AsRef<str>>(endpoint: S) -> Self {
        NhlPrimaryKey::ApiCache(ApiCacheKey {
            endpoint: endpoint.as_ref().to_string(),
        })
    }

    pub fn season(id: i32) -> Self {
        NhlPrimaryKey::Season(NhlSeasonKey { id })
    }

    pub fn franchise(id: i32) -> Self {
        NhlPrimaryKey::Franchise(NhlFranchiseKey { id })
    }

    pub fn team(id: i32) -> Self {
        NhlPrimaryKey::Team(NhlTeamKey { id })
    }

    pub fn player(id: i32) -> Self {
        NhlPrimaryKey::Player(NhlPlayerKey { id })
    }

    pub fn game(id: i32) -> Self {
        NhlPrimaryKey::Game(NhlGameKey { id })
    }

    pub fn roster_spot(game_id: i32, player_id: i32) -> Self {
        NhlPrimaryKey::RosterSpot(NhlRosterSpotKey { game_id, player_id })
    }

    pub fn play(game_id: i32, sort_order: i32) -> Self {
        NhlPrimaryKey::Play(NhlPlayKey {
            game_id,
            sort_order,
        })
    }

    pub fn playoff_series<S: AsRef<str>>(season_id: i32, series_letter: S) -> Self {
        NhlPrimaryKey::PlayoffSeries(NhlPlayoffSeriesKey {
            season_id,
            series_letter: series_letter.as_ref().to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlSeasonKey {
    pub id: i32,
}
impl NhlSeasonKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * from nhl_season where id=$1").bind(self.id)
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlFranchiseKey {
    pub id: i32,
}
impl NhlFranchiseKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_franchise WHERE id=$1"#).bind(self.id)
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlTeamKey {
    pub id: i32,
}
impl NhlTeamKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_team WHERE id=$1").bind(self.id)
    }
    async fn upsert_from_api(
        &self,
        db_context: &DbContext,
        nhl_api: &NhlApi,
    ) -> Result<(), LPError> {
        let team_id = self.id;

        let team_json_with_context: ItemParsedWithContext<NhlTeamJson> =
            nhl_api.get_nhl_team(db_context, team_id).await?;
        let team = team_json_with_context.to_db_struct();

        tracing::debug!("Upserting team with id {team_id} into lp database.");
        team.upsert(db_context).await?;
        tracing::debug!("Upserted team with id {team_id} into lp database.");
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayerKey {
    pub id: i32,
}
impl NhlPlayerKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_player WHERE id=$1"#).bind(self.id)
    }

    async fn upsert_from_api(
        &self,
        db_context: &DbContext,
        nhl_api: &NhlApi,
    ) -> Result<(), LPError> {
        let player_id = self.id;

        let player_json_with_context: ItemParsedWithContext<NhlPlayerJson> = nhl_api
            .fetch_by_id::<NhlPlayerJson>(db_context, player_id)
            .await?;
        let player: NhlPlayer = player_json_with_context.to_db_struct();

        tracing::debug!("Upserting player with id {player_id} into lp database.");
        player.upsert(db_context).await?;
        tracing::debug!("Upserted player with id {player_id} into lp database.");

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlGameKey {
    pub id: i32,
}
impl NhlGameKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_game WHERE id=$1").bind(self.id)
    }

    async fn upsert_from_api(
        &self,
        db_context: &DbContext,
        nhl_api: &NhlApi,
    ) -> Result<(), LPError> {
        let game_id = self.id;

        let game_json_with_context: ItemParsedWithContext<NhlGameJson> = nhl_api
            .fetch_by_id::<NhlGameJson>(db_context, game_id)
            .await?;
        let game: NhlGame = game_json_with_context.to_db_struct();

        tracing::debug!("Upserting game with id {game_id} into lp database.");
        game.upsert(db_context).await?;
        tracing::debug!("Upserted game with id {game_id} into lp database.");

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlRosterSpotKey {
    pub game_id: i32,
    pub player_id: i32,
}
impl NhlRosterSpotKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_roster_spot WHERE game_id=$1 AND player_id=$2")
            .bind(self.game_id)
            .bind(self.player_id)
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayKey {
    pub game_id: i32,
    pub sort_order: i32,
}
impl NhlPlayKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_play WHERE game_id=$1 AND sort_order=$2")
            .bind(self.game_id)
            .bind(self.sort_order)
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayoffSeriesKey {
    pub season_id: i32,
    pub series_letter: String,
}
impl NhlPlayoffSeriesKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_playoff_series WHERE season_id=$1 AND series_letter=$2")
            .bind(self.season_id)
            .bind(self.series_letter.clone())
    }
}
