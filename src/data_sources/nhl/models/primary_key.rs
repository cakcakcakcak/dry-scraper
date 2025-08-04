use crate::{
    LPError,
    common::db::{DbContext, Persistable, PrimaryKey, StaticPgQuery},
    common::models::{ApiCache, ApiCacheKey},
    data_sources::nhl::models::{
        NhlFranchise, NhlGame, NhlPlay, NhlPlayer, NhlPlayoffSeries, NhlRosterSpot, NhlSeason,
        NhlTeam,
    },
};

#[derive(Debug)]
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
impl PrimaryKey for NhlPrimaryKey {
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

#[derive(Debug)]
pub struct NhlSeasonKey {
    pub id: i32,
}
impl NhlSeasonKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * from nhl_season where id=$1").bind(self.id)
    }
}

#[derive(Debug)]
pub struct NhlFranchiseKey {
    pub id: i32,
}
impl NhlFranchiseKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_franchise WHERE id=$1"#).bind(self.id)
    }
}

#[derive(Debug)]
pub struct NhlTeamKey {
    pub id: i32,
}
impl NhlTeamKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_team WHERE id=$1").bind(self.id)
    }
}

#[derive(Debug)]
pub struct NhlPlayerKey {
    pub id: i32,
}
impl NhlPlayerKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_player WHERE id=$1"#).bind(self.id)
    }
}

#[derive(Debug)]
pub struct NhlGameKey {
    pub id: i32,
}
impl NhlGameKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_game WHERE id=$1").bind(self.id)
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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
