use serde::de;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::Type;

use crate::LPError;
use crate::models::ItemParsedWithContext;
use crate::models::traits::{HasTypeName, IntoDbStruct};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "defending_side", rename_all = "snake_case")]
pub enum DefendingSide {
    Left,
    Right,
}

#[derive(Clone)]
pub struct DefaultNhlContext {
    pub endpoint: String,
    pub raw_json: serde_json::Value,
}
#[derive(Clone)]
pub struct GameNhlContext {
    pub game_id: i32,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize)]
#[sqlx(type_name = "game_type", rename_all = "snake_case")]
pub enum GameType {
    Preseason = 1,
    RegularSeason = 2,
    Playoffs = 3,
}
impl TryFrom<i32> for GameType {
    type Error = &'static str;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(GameType::Preseason),
            2 => Ok(GameType::RegularSeason),
            3 => Ok(GameType::Playoffs),
            _ => Err("invalid game type"),
        }
    }
}

impl<'de> Deserialize<'de> for GameType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = i32::deserialize(deserializer)?;
        GameType::try_from(v).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "period_type", rename_all = "snake_case")]
pub enum PeriodTypeJson {
    #[serde(rename = "REG")]
    Regulation = 1,
    #[serde(rename = "OT")]
    Overtime = 2,
    #[serde(rename = "SO")]
    Shootout = 3,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodDescriptorJson {
    pub number: i32,
    pub period_type: PeriodTypeJson,
    pub max_regulation_periods: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedNameJson {
    pub default: String,
    pub fr: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlApiDataArrayResponse {
    pub data: Vec<serde_json::Value>,
    pub total: i32,
}
impl NhlApiDataArrayResponse {
    pub fn map_json_array_to_json_structs<T>(
        self,
        endpoint: &str,
    ) -> Vec<Result<ItemParsedWithContext<T>, LPError>>
    where
        T: serde::de::DeserializeOwned + HasTypeName + IntoDbStruct<Context = DefaultNhlContext>,
    {
        self.data
            .iter()
            .map(|json_value| {
                let raw_data: String = json_value.to_string();
                let parsed: Result<T, LPError> =
                    serde_json::from_value(json_value.clone()).map_err(LPError::from);
                match parsed {
                    Ok(item) => Ok(ItemParsedWithContext {
                        item,
                        context: DefaultNhlContext{raw_json: json_value.clone(), endpoint: endpoint.to_string()}
                    }),
                    Err(e) => {
                        tracing::warn!(endpoint=%endpoint, error=%e, "Failed to parse item to `{}`.", T::type_name());
                        tracing::debug!(raw_data);
                        Err(e)
                    }
                }
            })
            .collect()
    }
}
