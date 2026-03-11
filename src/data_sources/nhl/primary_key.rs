use std::fmt::Debug;

use sqlx::FromRow;

use crate::{
    common::db::{CacheKey, PrimaryKey, StaticPgQuery},
    data_sources::nhl::models::{
        NhlFranchise, NhlGame, NhlPlay, NhlPlayer, NhlPlayoffBracketSeries, NhlPlayoffSeries,
        NhlPlayoffSeriesGame, NhlRosterSpot, NhlSeason, NhlShift, NhlTeam,
    },
};

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlSeasonKey {
    pub id: i32,
}
impl PrimaryKey for NhlSeasonKey {
    type Entity = NhlSeason;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * from nhl_season where id=$1").bind(self.id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "season",
            id: self.id.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlFranchiseKey {
    pub id: i32,
}
impl PrimaryKey for NhlFranchiseKey {
    type Entity = NhlFranchise;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_franchise WHERE id=$1"#).bind(self.id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "franchise",
            id: self.id.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlTeamKey {
    pub id: i32,
}
impl PrimaryKey for NhlTeamKey {
    type Entity = NhlTeam;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_team WHERE id=$1").bind(self.id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "team",
            id: self.id.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayerKey {
    pub id: i32,
}
impl PrimaryKey for NhlPlayerKey {
    type Entity = NhlPlayer;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(r#"SELECT * FROM nhl_player WHERE id=$1"#).bind(self.id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "player",
            id: self.id.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlGameKey {
    pub id: i32,
}
impl PrimaryKey for NhlGameKey {
    type Entity = NhlGame;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_game WHERE id=$1").bind(self.id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "game",
            id: self.id.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlRosterSpotKey {
    pub game_id: i32,
    pub player_id: i32,
}
impl PrimaryKey for NhlRosterSpotKey {
    type Entity = NhlRosterSpot;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_roster_spot WHERE game_id=$1 AND player_id=$2")
            .bind(self.game_id)
            .bind(self.player_id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "nhl_roster_spot",
            id: format!("{}|{}", self.game_id, self.player_id),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayKey {
    pub game_id: i32,
    pub event_id: i32,
}
impl PrimaryKey for NhlPlayKey {
    type Entity = NhlPlay;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_play WHERE game_id=$1 AND event_id=$2")
            .bind(self.game_id)
            .bind(self.event_id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "play",
            id: format!("{}|{}", self.game_id, self.event_id),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlShiftKey {
    pub game_id: i32,
    pub player_id: i32,
    pub shift_number: i32,
}
impl PrimaryKey for NhlShiftKey {
    type Entity = NhlShift;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * from nhl_shift WHERE game_id=$1 AND player_id=$2 AND shift_number=$3")
            .bind(self.game_id)
            .bind(self.player_id)
            .bind(self.shift_number)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "shift",
            id: format!("{}|{}|{}", self.game_id, self.player_id, self.shift_number),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayoffBracketSeriesKey {
    pub season_id: i32,
    pub series_letter: String,
}
impl PrimaryKey for NhlPlayoffBracketSeriesKey {
    type Entity = NhlPlayoffBracketSeries;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query(
            "SELECT * FROM nhl_playoff_bracket_series WHERE season_id=$1 AND series_letter=$2",
        )
        .bind(self.season_id)
        .bind(self.series_letter.clone())
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "playoff_bracket_series",
            id: format!("{}|{}", self.season_id, self.series_letter),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayoffSeriesKey {
    pub season_id: i32,
    pub series_letter: String,
}
impl PrimaryKey for NhlPlayoffSeriesKey {
    type Entity = NhlPlayoffSeries;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_playoff_series WHERE season_id=$1 AND series_letter=$2")
            .bind(self.season_id)
            .bind(self.series_letter.clone())
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "playoff_series",
            id: format!("{}|{}", self.season_id, self.series_letter),
        }
    }
}

#[derive(Clone, Debug, Eq, FromRow, Hash, PartialEq)]
pub struct NhlPlayoffSeriesGameKey {
    pub id: i32,
}
impl PrimaryKey for NhlPlayoffSeriesGameKey {
    type Entity = NhlPlayoffSeriesGame;

    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM nhl_playoff_series_game WHERE id=$1").bind(self.id)
    }

    fn cache_key(&self) -> CacheKey {
        CacheKey {
            source: "nhl",
            table: "playoff_series_game",
            id: self.id.to_string(),
        }
    }
}
