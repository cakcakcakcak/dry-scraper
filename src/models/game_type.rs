use serde::Serialize;
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize)]
#[sqlx(type_name = "game_type", rename_all = "snake_case")]
pub enum GameType {
    Preseason,
    RegularSeason,
    Playoffs,
}
