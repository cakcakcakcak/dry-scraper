use serde::{Deserialize, Deserializer, de::IntoDeserializer};

/// usage: `#[serde(deserialize_with = "deserialize_to_bool")]`
/// deserialize the String, int, or bool value indicated in the struct into a bool
pub(crate) fn deserialize_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    value_to_bool::<D::Error>(v)
}

/// deserialize a `serde::json::Value` into a bool
pub(crate) fn value_to_bool<E>(value: serde_json::Value) -> Result<bool, E>
where
    E: serde::de::Error,
{
    use serde_json::Value;
    match value {
        Value::Bool(b) => Ok(b),
        Value::Number(n) => {
            let number_val = n.as_i64();
            if number_val != Some(0) && number_val != Some(1) {
                tracing::warn!(
                    "int_to_bool: unexpected value {n} (expected 0 or 1), converting to `true`."
                );
            }
            Ok(n != 0.into())
        }
        Value::String(ref s) if s == "true" => Ok(true),
        Value::String(ref s) if s == "false" => Ok(false),
        Value::Null => {
            tracing::warn!(
                "value_to_bool: unexpected value `serde_json::Value::Null`, converting to `false`."
            );
            Ok(false)
        }
        other => Err(E::custom(format!(
            "Unable to meaningfully deserialize value to bool: {:?}",
            other
        ))),
    }
}

/// usage: `#[serde(deserialize_with = "deserialize_to_string")]`
/// deserialize the number value indicated in the struct into a string
pub(crate) fn deserialize_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    value_to_string::<D::Error>(value)
}

/// deserialize a `serde::json::Value` into a `String``
pub(crate) fn value_to_string<E>(value: serde_json::Value) -> Result<String, E>
where
    E: serde::de::Error,
{
    use serde_json::Value;
    match &value {
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => Ok(s.to_string()),
        Value::Null => Ok(String::new()),
        Value::Array(_) => Ok(value.to_string()),
        Value::Object(_) => Ok(value.to_string()),
    }
}

/// deserialize a `serde::json::Value` into an `i32`
pub(crate) fn value_to_i32<E>(value: serde_json::Value) -> Result<i32, E>
where
    E: serde::de::Error,
{
    use serde_json::Value;
    match &value {
        Value::Bool(true) => Ok(1),
        Value::Bool(false) => Ok(0),
        Value::Number(n) => n
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| E::custom(format!("Number out of range for i32: {n}"))),
        Value::String(s) => match s.parse::<i32>() {
            Ok(n) => Ok(n),
            Err(e) => {
                tracing::warn!("Could not parse string `{s}` as i32: {e}");
                Err(E::custom(e))
            }
        },
        Value::Null => {
            tracing::warn!(
                "value_to_i32: unexpected value `serde_json::Value::Null`, converting to `0`."
            );
            Ok(0)
        }
        other => Err(E::custom(format!(
            "Unable to meaningfully deserialize value to `i32`: {:?}",
            other
        ))),
    }
}

/// usage: `#[serde(deserialize_with = "deserialize_json_object_default_to_string")]`
/// deserialize the value from the '"default"' key in the json object to `String`
pub(crate) fn deserialize_json_object_default_to_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    let json = Value::deserialize(deserializer)?;
    match get_key_as_string(&json, "default") {
        Some(v) => Ok(Some(v)),
        None => Ok(None),
    }
}

/// usage: `#[serde(deserialize_with = "deserialize_json_object_default_to_i32")]`
/// deserialize the value from the '"default"' key in the json object to `i32`
pub(crate) fn deserialize_json_object_default_to_i32<'de, D>(
    deserializer: D,
) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    let json = Value::deserialize(deserializer)?;
    match get_key_as_i32(&json, "default") {
        Some(v) => Ok(Some(v)),
        None => Ok(None),
    }
}

/// usage: `#[serde(deserialize_with = "deserialize_json_object_default_to_bool")]`
/// deserialize the value from the '"default"' key in the json object to `bool`
pub(crate) fn deserialize_json_object_default_to_bool<'de, D>(
    deserializer: D,
) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    let json = Value::deserialize(deserializer)?;
    match get_key_as_bool(&json, "default") {
        Some(v) => Ok(Some(v)),
        None => Ok(None),
    }
}

pub(crate) fn get_key_as_value(value: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
    match value.get(&key) {
        Some(v) => Some(v.clone()),
        None => {
            tracing::warn!("Key `{key}` not found in json object: {value:?}");
            None
        }
    }
}

pub(crate) fn get_key_as_string(value: &serde_json::Value, key: &str) -> Option<String> {
    match value.get(&key) {
        Some(v) => value_to_string::<serde::de::value::Error>(v.clone()).ok(),
        None => {
            tracing::warn!("Key `{key}` not found in json object: {value:?}");
            None
        }
    }
}

pub(crate) fn get_key_as_i32(value: &serde_json::Value, key: &str) -> Option<i32> {
    match value.get(&key) {
        Some(v) => value_to_i32::<serde::de::value::Error>(v.clone()).ok(),
        None => {
            tracing::warn!("Key `{key}` not found in json object: {value:?}");
            None
        }
    }
}

pub(crate) fn get_key_as_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    match value.get(&key) {
        Some(v) => value_to_bool::<serde::de::value::Error>(v.clone()).ok(),
        None => {
            tracing::warn!("Key `{key}` not found in json object: {value:?}");
            None
        }
    }
}

pub(crate) fn try_get<'a>(
    field: &str,
    json: &'a serde_json::Value,
    endpoint: &str,
) -> Result<&'a serde_json::Value, crate::lp_error::LPError> {
    json.get(field).ok_or_else(|| {
        crate::lp_error::LPError::ApiCustom(format!(
            "No '{}' field found in API response for endpoint {}",
            field, endpoint
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json::json;

    #[test]
    fn test_value_to_bool() {
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(json!(true)).unwrap(),
            true
        );
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(json!(false)).unwrap(),
            false
        );
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(json!(1)).unwrap(),
            true
        );
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(json!(0)).unwrap(),
            false
        );
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(json!("true")).unwrap(),
            true
        );
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(json!("false")).unwrap(),
            false
        );
        assert_eq!(
            value_to_bool::<serde::de::value::Error>(serde_json::Value::Null).unwrap(),
            false
        );

        let err = value_to_bool::<serde::de::value::Error>(json!("notabool"));
        assert!(err.is_err());
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(
            value_to_string::<serde::de::value::Error>(json!(true)).unwrap(),
            "true"
        );
        assert_eq!(
            value_to_string::<serde::de::value::Error>(json!(123)).unwrap(),
            "123"
        );
        assert_eq!(
            value_to_string::<serde::de::value::Error>(json!("abc")).unwrap(),
            "abc"
        );
        assert_eq!(
            value_to_string::<serde::de::value::Error>(serde_json::Value::Null).unwrap(),
            ""
        );
        assert_eq!(
            value_to_string::<serde::de::value::Error>(json!([1, 2, 3])).unwrap(),
            "[1,2,3]"
        );
        assert_eq!(
            value_to_string::<serde::de::value::Error>(json!({"a":1})).unwrap(),
            "{\"a\":1}"
        );
    }

    #[test]
    fn test_value_to_i32() {
        assert_eq!(
            value_to_i32::<serde::de::value::Error>(json!(true)).unwrap(),
            1
        );
        assert_eq!(
            value_to_i32::<serde::de::value::Error>(json!(false)).unwrap(),
            0
        );
        assert_eq!(
            value_to_i32::<serde::de::value::Error>(json!(42)).unwrap(),
            42
        );
        assert_eq!(
            value_to_i32::<serde::de::value::Error>(json!("123")).unwrap(),
            123
        );
        assert_eq!(
            value_to_i32::<serde::de::value::Error>(serde_json::Value::Null).unwrap(),
            0
        );

        let err = value_to_i32::<serde::de::value::Error>(json!("notanint"));
        assert!(err.is_err());
        let err = value_to_i32::<serde::de::value::Error>(json!([1, 2, 3]));
        assert!(err.is_err());
    }
    #[test]
    fn test_get_key_as_value() {
        let v = json!({"default": "hello"});
        assert_eq!(get_key_as_value(&v, "default"), Some(json!("hello")));
        let v = json!({});
        assert_eq!(get_key_as_value(&v, "missing"), None);
    }

    #[test]
    fn test_get_key_as_string() {
        let v = json!({"default": "hello"});
        assert_eq!(get_key_as_string(&v, "default"), Some("hello".to_string()));
        let v = json!({});
        assert_eq!(get_key_as_string(&v, "missing"), None);
    }

    #[test]
    fn test_get_key_as_i32() {
        let v = json!({"default": 42});
        assert_eq!(get_key_as_i32(&v, "default"), Some(42));
        let v = json!({"default": "123"});
        assert_eq!(get_key_as_i32(&v, "default"), Some(123));
        let v = json!({});
        assert_eq!(get_key_as_i32(&v, "missing"), None);
    }

    #[test]
    fn test_get_key_as_bool() {
        let v = json!({"default": true});
        assert_eq!(get_key_as_bool(&v, "default"), Some(true));
        let v = json!({"default": 0});
        assert_eq!(get_key_as_bool(&v, "default"), Some(false));
        let v = json!({});
        assert_eq!(get_key_as_bool(&v, "missing"), None);
    }

    #[test]
    fn test_deserialize_json_object_default_to_string() {
        let v = json!({"default": "abc"});
        let s: Option<String> =
            deserialize_json_object_default_to_string(v.into_deserializer()).unwrap();
        assert_eq!(s, Some("abc".to_string()));
    }

    #[test]
    fn test_deserialize_json_object_default_to_i32() {
        let v = json!({"default": 99});
        let s: Option<i32> = deserialize_json_object_default_to_i32(v.into_deserializer()).unwrap();
        assert_eq!(s, Some(99));
    }

    #[test]
    fn test_deserialize_json_object_default_to_bool() {
        let v = json!({"default": false});
        let s: Option<bool> =
            deserialize_json_object_default_to_bool(v.into_deserializer()).unwrap();
        assert_eq!(s, Some(false));
    }
}
