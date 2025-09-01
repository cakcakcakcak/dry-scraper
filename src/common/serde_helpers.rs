use serde_json::Value;
use sqlx::postgres::types::PgInterval;

#[macro_export]
macro_rules! make_deserialize_to_type {
    ($func_name:ident, Option<$ty:ty>) => {
        #[doc = concat!(
                                                    "Deserializes the JSON object to `Option<",
                                                    stringify!($ty),
                                                    ">`.\nUsage: `#[serde(deserialize_with = \"",
                                                    stringify!($func_name),
                                                    "\")]`"
                                                )]
        pub fn $func_name<'de, D>(deserializer: D) -> Result<Option<$ty>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = serde_json::Value::deserialize(deserializer)?;
            Ok(value.as_logged::<$ty>())
        }
    };
    ($func_name:ident, $ty:ty) => {
        #[doc = concat!(
                                                    "Deserializes the JSON object to `",
                                                    stringify!($ty),
                                                    "`.\nUsage: `#[serde(deserialize_with = \"",
                                                    stringify!($func_name),
                                                    "\")]`"
                                                )]
        pub fn $func_name<'de, D>(deserializer: D) -> Result<$ty, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = serde_json::Value::deserialize(deserializer)?;
            value.as_logged::<$ty>().ok_or_else(|| {
                serde::de::Error::custom(format!("Failed to deserialize value `{value}` to `{}`.", stringify!($ty)))
            })
        }
    };
}

#[macro_export]
macro_rules! make_deserialize_key_to_type {
    ($func_name:ident, $key:expr, Option<$ty:ty>) => {
        #[doc = concat!(
                                                    "Deserializes the value from the `\"",
                                                    $key,
                                                    "\"` key in the JSON object to `Option<",
                                                    stringify!($ty),
                                                    ">`.\nUsage: `#[serde(deserialize_with = \"",
                                                    stringify!($func_name),
                                                    "\")]`"
                                                )]
        fn $func_name<'de, D>(deserializer: D) -> Result<Option<$ty>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = serde_json::Value::deserialize(deserializer)?;
            Ok(value._get_key_as_logged::<$ty>($key))
        }
    };
    ($func_name:ident, $key:expr, $ty:ty) => {
        #[doc = concat!(
                                                    "Deserializes the value from the `\"",
                                                    $key,
                                                    "\"` key in the JSON object to `",
                                                    stringify!($ty),
                                                    "`.\nUsage: `#[serde(deserialize_with = \"",
                                                    stringify!($func_name),
                                                    "\")]`"
                                                )]
        fn $func_name<'de, D>(deserializer: D) -> Result<$ty, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = serde_json::Value::deserialize(deserializer)?;
            value._get_key_as_logged::<$ty>($key).ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "Failed to deserialize value `{value}` to `{}`.",
                    stringify!($ty)
                ))
            })
        }
    };
}

#[macro_export]
macro_rules! make_deserialize_nested_key_to_type {
    ($func_name:ident, [$($key:expr),+], Option<$ty:ty>) => {
        #[doc = concat!(
            "Deserializes the value from the nested keys `[",
            $(stringify!($key), ", "),+,
            "]` in the JSON object to `Option<",
            stringify!($ty),
            ">`.\nUsage: `#[serde(deserialize_with = \"",
            stringify!($func_name),
            "\")]`"
        )]
        fn $func_name<'de, D>(deserializer: D) -> Result<Option<$ty>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = serde_json::Value::deserialize(deserializer)?;
            Ok(value.get_nested_key_as_logged::<$ty>(&[$($key),+]))
        }
    };
    ($func_name:ident, [$($key:expr),+], $ty:ty) => {
        #[doc = concat!(
            "Deserializes the value from the nested keys `[",
            $(stringify!($key), ", "),+,
            "]` in the JSON object to `",
            stringify!($ty),
            "`.\nUsage: `#[serde(deserialize_with = \"",
            stringify!($func_name),
            "\")]`"
        )]
        fn $func_name<'de, D>(deserializer: D) -> Result<$ty, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = serde_json::Value::deserialize(deserializer)?;
            value.get_nested_key_as_logged::<$ty>(&[$($key),+]).ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "Failed to deserialize nested keys `[{}]` to `{}`.",
                    vec![$($key),+].join(", "),
                    stringify!($ty)
                ))
            })
        }
    };
}

pub trait AsLogged: Sized {
    fn as_logged(value: &serde_json::Value) -> Option<Self>;
}
impl AsLogged for serde_json::Value {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        Some(value.clone())
    }
}
impl AsLogged for String {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        match value {
            Value::String(s) => Some(s.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::Null => Some(String::new()),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                let s = value.to_string();
                tracing::info!("Converting complex JSON value to string: {}", s);
                Some(s)
            }
        }
    }
}
impl AsLogged for i32 {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        match value {
            Value::Bool(true) => Some(1),
            Value::Bool(false) => Some(0),
            Value::Number(n) => n.as_i64().and_then(|v| {
                i32::try_from(v).ok().or_else(|| {
                    tracing::info!("Number out of range for i32: {n}");
                    None
                })
            }),
            Value::String(s) => match s.parse::<i32>() {
                Ok(n) => Some(n),
                Err(e) => {
                    tracing::info!("Could not parse string `{s}` as i32: {e}");
                    None
                }
            },
            Value::Null => {
                tracing::info!("Unexpected value `serde_json::Value::Null`, converting to `0`.");
                Some(0)
            }
            other => {
                tracing::info!("Unable to meaningfully deserialize value {other} to `i32`");
                None
            }
        }
    }
}
impl AsLogged for bool {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        match value {
            Value::Bool(b) => Some(*b),
            Value::String(s) if s == "true" => Some(true),
            Value::String(s) if s == "false" => Some(false),
            Value::Number(n) => {
                let number_val = n.as_i64();
                if number_val != Some(0) && number_val != Some(1) {
                    tracing::info!("Unexpected value {n} (expected 0 or 1), converting to `true`.");
                }
                Some(number_val != Some(0))
            }
            Value::Null => {
                tracing::info!(
                    "Unexpected value `serde_json::Value::Null`, converting to `false`."
                );
                Some(false)
            }
            other => {
                tracing::info!("Unable to meaningfully convert value {other} to bool");
                None
            }
        }
    }
}

pub trait JsonExt {
    fn as_logged<T: AsLogged>(&self) -> Option<T>;
}

impl JsonExt for serde_json::Value {
    fn as_logged<T: AsLogged>(&self) -> Option<T> {
        T::as_logged(self)
    }
}

pub fn parse_mmss_to_pginterval(s: &str) -> sqlx::postgres::types::PgInterval {
    let parts: Vec<&str> = s.split(':').collect();
    let (minutes, seconds) = match parts.as_slice() {
        [mm, ss] => (
            mm.parse::<i64>().unwrap_or(0),
            ss.parse::<i64>().unwrap_or(0),
        ),
        _ => (0, 0),
    };
    let total_us = (minutes * 60 + seconds) * 1_000_000;
    PgInterval {
        months: 0,
        days: 0,
        microseconds: total_us,
    }
}
