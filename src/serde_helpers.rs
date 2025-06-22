//! This module provides helper functions and utilities for working with Serde serialization and deserialization.
//! 
//! It imports the necessary traits and types from the `serde` crate, including `Deserialize` and `Deserializer`,
//! to facilitate custom (de)serialization logic throughout the project.

use serde::{Deserialize, Deserializer};

pub(crate) fn int_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let v = i32::deserialize(deserializer)?;
    Ok(v != 0)
}

pub(crate) fn number_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        _ => Err(D::Error::custom("expected string or integer for id")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_int_to_bool_with_1() {
        let b: bool = int_to_bool(&json!(1)).unwrap();
        assert!(b);
    }

    #[test]
    fn test_int_to_bool_with_0() {
        let b: bool = int_to_bool(&json!(0)).unwrap();
        assert!(!b);
    }

    #[test]
    fn test_int_to_bool_with_2() {
        let b: bool = int_to_bool(&json!(2)).unwrap();
        assert!(b);
    }

    #[test]
    fn test_number_to_string_with_integer() {
        let s: String = number_to_string(&serde_json::json!(12345)).unwrap();
        assert_eq!(s, "12345");
    }

    #[test]
    fn test_number_to_string_with_string() {
        let s: String = number_to_string(&serde_json::json!("abc123")).unwrap();
        assert_eq!(s, "abc123");
    }

    #[test]
    fn test_number_to_string_with_negative_integer() {
        let s: String = number_to_string(&serde_json::json!(-42)).unwrap();
        assert_eq!(s, "-42");
    }

    #[test]
    fn test_number_to_string_with_float() {
        let s: String = number_to_string(&serde_json::json!(3.14)).unwrap();
        assert_eq!(s, "3.14");
    }

    #[test]
    fn test_number_to_string_with_bool_should_fail() {
        let result = number_to_string(&serde_json::json!(true));
        assert!(result.is_err());
    }

    #[test]
    fn test_number_to_string_with_null_should_fail() {
        let result = number_to_string(&serde_json::json!(null));
        assert!(result.is_err());
    }
}
