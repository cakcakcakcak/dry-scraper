impl<'de> serde::Deserialize<'de> for crate::models::game_type::GameType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use crate::models::game_type::GameType;

        let v = i32::deserialize(deserializer)?;
        match v {
            1 => Ok(GameType::Preseason),
            2 => Ok(GameType::RegularSeason),
            3 => Ok(GameType::Playoffs),
            other => {
                tracing::error!("Unknown `game_type` value: {}", other);
                Err(serde::de::Error::custom(format!(
                    "Unknown `game_type`: {}",
                    other
                )))
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for crate::models::period_type::PeriodType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use crate::models::period_type::PeriodType;

        let v = i32::deserialize(deserializer)?;
        match v {
            1 => Ok(PeriodType::Regulation),
            2 => Ok(PeriodType::Overtime),
            3 => Ok(PeriodType::Shootout),
            other => {
                tracing::error!("Unknown `period_type` value: {}", other);
                Err(serde::de::Error::custom(format!(
                    "Unknown `period_type`: {}",
                    other
                )))
            }
        }
    }
}

macro_rules! make_deserialize_to_type {
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
            use serde::Deserialize;
            use $crate::serde_helpers::JsonExt;
            let value = serde_json::Value::deserialize(deserializer)?;
            value.as_logged::<$ty>().ok_or_else(|| {
                serde::de::Error::custom(format!("Failed to deserialize value `{value}` to `{}`.", stringify!($ty)))
            })
        }
    };
}

make_deserialize_to_type!(deserialize_to_bool, bool);

macro_rules! make_deserialize_key_to_type {
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
        pub fn $func_name<'de, D>(deserializer: D) -> Result<$ty, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::Deserialize;
            use $crate::serde_helpers::JsonExt;
            let value = serde_json::Value::deserialize(deserializer)?;
            value.get_key_as_logged::<$ty>($key).ok_or_else(|| {
                serde::de::Error::custom(format!("Failed to deserialize value `{value}` to `{}`.", stringify!($ty)))
            })
        }
    };
}

make_deserialize_key_to_type!(deserialize_default_to_string, "default", String);

macro_rules! make_deserialize_key_to_option_type {
    ($func_name:ident, $key:expr, $ty:ty) => {
        #[doc = concat!(
                                                    "Deserializes the value from the `\"",
                                                    $key,
                                                    "\"` key in the JSON object to `Option<",
                                                    stringify!($ty),
                                                    ">`.\nUsage: `#[serde(deserialize_with = \"",
                                                    stringify!($func_name),
                                                    "\")]`"
                                                )]
        pub fn $func_name<'de, D>(deserializer: D) -> Result<Option<$ty>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::Deserialize;
            use $crate::serde_helpers::JsonExt;
            let value = serde_json::Value::deserialize(deserializer)?;
            match value.get_key_as_logged::<$ty>($key) {
                Some(v) => Ok(Some(v)),
                None => Err(serde::de::Error::custom(format!("Failed to deserialize value `{value}` to `Option<{}>`.", stringify!($ty))))
            }
        }
    };
}

make_deserialize_key_to_option_type!(deserialize_default_to_option_string, "default", String);

pub trait AsLogged: Sized {
    fn as_logged(value: &serde_json::Value) -> Option<Self>;
    fn get_key_as_logged(value: &serde_json::Value, key: &str) -> Option<Self> {
        match value.get(&key) {
            Some(v) => v.as_logged::<Self>(),
            None => {
                tracing::warn!("Key `{key}` not found in json object: {value:?}");
                None
            }
        }
    }
    fn get_nested_as_logged(value: &serde_json::Value, keys: &[&str]) -> Option<Self> {
        let mut current = value;
        for &key in keys {
            current = match current.get(key) {
                Some(v) => v,
                None => {
                    tracing::warn!("Key `{key}` not found in json object: {current:?}");
                    return None;
                }
            }
        }
        Self::as_logged(current)
    }
}
impl AsLogged for serde_json::Value {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        Some(value.clone())
    }
}
impl AsLogged for String {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        use serde_json::Value;
        match value {
            Value::String(s) => Some(s.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::Null => Some(String::new()),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                let s = value.to_string();
                tracing::warn!("Converting complex JSON value to string: {}", s);
                Some(s)
            }
        }
    }
}

impl AsLogged for i32 {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        use serde_json::Value;
        match value {
            Value::Bool(true) => Some(1),
            Value::Bool(false) => Some(0),
            Value::Number(n) => n.as_i64().and_then(|v| {
                i32::try_from(v).ok().or_else(|| {
                    tracing::warn!("Number out of range for i32: {n}");
                    None
                })
            }),
            Value::String(s) => match s.parse::<i32>() {
                Ok(n) => Some(n),
                Err(e) => {
                    tracing::warn!("Could not parse string `{s}` as i32: {e}");
                    None
                }
            },
            Value::Null => {
                tracing::warn!(
                    "value_to_i32: unexpected value `serde_json::Value::Null`, converting to `0`."
                );
                Some(0)
            }
            other => {
                tracing::warn!("Unable to meaningfully deserialize value {other} to `i32`");
                None
            }
        }
    }
}

impl AsLogged for bool {
    fn as_logged(value: &serde_json::Value) -> Option<Self> {
        use serde_json::Value;

        match value {
            Value::Bool(b) => Some(*b),
            Value::String(s) if s == "true" => Some(true),
            Value::String(s) if s == "false" => Some(false),
            Value::Number(n) => {
                let number_val = n.as_i64();
                if number_val != Some(0) && number_val != Some(1) {
                    tracing::warn!(
                        "int_to_bool: unexpected value {n} (expected 0 or 1), converting to `true`."
                    );
                }
                Some(number_val != Some(0))
            }
            Value::Null => {
                tracing::warn!(
                    "value_to_bool: unexpected value `serde_json::Value::Null`, converting to `false`."
                );
                Some(false)
            }
            other => {
                tracing::warn!(
                    "value_to_bool: unable to meaningfully convert value {other} to bool"
                );
                None
            }
        }
    }
}

// Extension trait for ergonomic method call
pub trait JsonExt {
    fn as_logged<T: AsLogged>(&self) -> Option<T>;
    fn get_key_as_logged<T: AsLogged>(&self, key: &str) -> Option<T>;
    fn get_nested_as_logged<T: AsLogged>(&self, keys: &[&str]) -> Option<T>;
    fn get_key_as_string(&self, key: &str) -> Option<String> {
        self.get_key_as_logged::<String>(key)
    }
    fn get_key_as_i32(&self, key: &str) -> Option<i32> {
        self.get_key_as_logged::<i32>(key)
    }
    fn get_key_as_bool(&self, key: &str) -> Option<bool> {
        self.get_key_as_logged::<bool>(key)
    }
    fn get_key_as_value(&self, key: &str) -> Option<serde_json::Value> {
        self.get_key_as_logged::<serde_json::Value>(key)
    }
    fn get_nested_as_string(&self, keys: &[&str]) -> Option<String> {
        self.get_nested_as_logged::<String>(keys)
    }
}

impl JsonExt for serde_json::Value {
    fn as_logged<T: AsLogged>(&self) -> Option<T> {
        T::as_logged(self)
    }
    fn get_key_as_logged<T: AsLogged>(&self, key: &str) -> Option<T> {
        T::get_key_as_logged(self, key)
    }
    fn get_nested_as_logged<T: AsLogged>(&self, keys: &[&str]) -> Option<T> {
        T::get_nested_as_logged(self, keys)
    }
}
