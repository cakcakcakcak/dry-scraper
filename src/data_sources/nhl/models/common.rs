use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::Type;

use crate::common::models::traits::{HasTypeName, IntoDbStruct};
use crate::common::models::ItemParsedWithContext;
use crate::DSError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "defending_side", rename_all = "snake_case")]
pub enum DefendingSide {
    Left,
    Right,
}

#[derive(Clone)]
pub struct NhlDefaultContext {
    pub raw_json: serde_json::Value,
    pub endpoint: String,
}
#[derive(Clone)]
pub struct NhlSeasonContext {
    pub season_id: i32,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
}
#[derive(Clone)]
pub struct NhlGameContext {
    pub game_id: i32,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
}
#[derive(Clone)]
pub struct NhlPlayoffSeriesContext {
    pub series_letter: String,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodDescriptorJson {
    pub number: i32,
    pub period_type: PeriodTypeJson,
    pub max_regulation_periods: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedNameJson {
    pub default: Option<String>,
    pub cs: Option<String>,
    pub de: Option<String>,
    pub sv: Option<String>,
    pub fi: Option<String>,
    pub sk: Option<String>,
    pub en: Option<String>,
    pub fr: Option<String>,
    pub es: Option<String>,
}
impl LocalizedNameJson {
    pub fn best_str(self) -> String {
        self.default
            .or(self.en)
            .or(self.fr)
            .or(self.es)
            .or(self.de)
            .or(self.fi)
            .or(self.sv)
            .or(self.cs)
            .or(self.sk)
            .unwrap_or("".to_string())
    }
}
pub trait LocalizedNameJsonExt {
    fn best_str_or_none(self) -> Option<String>;
}

impl LocalizedNameJsonExt for Option<LocalizedNameJson> {
    fn best_str_or_none(self) -> Option<String> {
        self.map(|name| name.best_str())
    }
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
    ) -> Vec<Result<ItemParsedWithContext<T>, DSError>>
    where
        T: serde::de::DeserializeOwned + HasTypeName + IntoDbStruct<Context = NhlDefaultContext>,
    {
        self.data
            .iter()
            .map(|json_value| {
                let parsed: Result<T, DSError> =
                    serde_json::from_value(json_value.clone()).map_err(DSError::from);
                match parsed {
                    Ok(item) => Ok(ItemParsedWithContext {
                        item,
                        context: NhlDefaultContext{raw_json: json_value.clone(), endpoint: endpoint.to_string()}
                    }),
                    Err(e) => {
                        tracing::warn!(endpoint=%endpoint, error=%e, "Failed to parse item to `{}`.", T::type_name());
                        tracing::info!(?json_value, "Raw JSON that failed to parse");
                        Err(e)
                    }
                }
            })
            .collect()
    }
}
