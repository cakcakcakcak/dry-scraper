use serde::Serialize;
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize)]
#[sqlx(type_name = "game_type", rename_all = "snake_case")]
pub enum GameType {
    Preseason,
    RegularSeason,
    Playoffs,
}
impl GameType {
    fn from_str(n: i32) -> Option<GameType> {
        match n {
            1 => Some(crate::models::game_type::GameType::Preseason),
            2 => Some(crate::models::game_type::GameType::RegularSeason),
            3 => Some(crate::models::game_type::GameType::Playoffs),
            n => {
                tracing::warn!("Encountered unexpected value {n} when accessing `GameType`");
                None
            }
        }
    }
}
