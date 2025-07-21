use tokio_retry::RetryIf;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

use crate::config::CONFIG;
use crate::lp_error::LPError;
use crate::models::item_parsed_with_context::ItemParsedWithContext;
use crate::models::nhl::nhl_model_common::NhlApiDataArrayResponse;

#[macro_export]
macro_rules! sqlx_operation_with_retries {
    ($body:expr) => {
        $crate::util::sqlx_operation_with_retries(|| async { $body })
    };
}

#[macro_export]
macro_rules! reqwest_with_retries {
    ($body:expr) => {
        $crate::util::reqwest_with_retries(|| async { $body })
    };
}

#[macro_export]
macro_rules! impl_has_type_name {
    ($t:ty) => {
        impl $crate::models::traits::HasTypeName for $t {
            fn type_name() -> &'static str {
                stringify!($t)
            }
        }
    };
}

pub fn default_retry_strategy() -> impl Iterator<Item = std::time::Duration> {
    ExponentialBackoff::from_millis(CONFIG.retry_jitter_duration_ms)
        .map(jitter)
        .take(CONFIG.retries)
}

pub fn is_transient_sqlx_error(e: &sqlx::Error) -> bool {
    let is_transient = matches!(e, sqlx::Error::Io(_) | sqlx::Error::Tls(_));
    if is_transient {
        tracing::debug!("Retrying sqlx operation after transient error: {:?}", e);
    }
    is_transient
}

pub fn is_transient_reqwest_error(e: &reqwest::Error) -> bool {
    let is_transient = e.is_timeout() || e.is_connect();
    if is_transient {
        tracing::debug!("Retrying reqwest operation after transient error: {:?}", e);
    }
    is_transient
}

pub async fn sqlx_operation_with_retries<F, Fut, T>(operation: F) -> Result<T, sqlx::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    RetryIf::spawn(
        default_retry_strategy(),
        || async {
            let res = operation().await;
            if let Err(ref e) = res {
                tracing::debug!("sqlx operation failed after error: {:?}", e);
            }
            res
        },
        is_transient_sqlx_error,
    )
    .await
}

pub async fn reqwest_with_retries<F, Fut, T>(operation: F) -> Result<T, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
{
    RetryIf::spawn(
        default_retry_strategy(),
        || async {
            let res = operation().await;
            if let Err(ref e) = res {
                tracing::debug!("reqwest operation failed after error {:?}", e);
            }
            res
        },
        is_transient_reqwest_error,
    )
    .await
}

pub fn map_json_array_to_json_structs<T>(
    data_array_response: NhlApiDataArrayResponse,
    endpoint: &str,
) -> Vec<Result<ItemParsedWithContext<T>, LPError>>
where
    T: serde::de::DeserializeOwned,
{
    data_array_response
        .data
        .iter()
        .map(|item| {
            let raw_data = item.to_string();
            let parsed = serde_json::from_value(item.clone()).map_err(LPError::from);
            match parsed {
                Ok(item) => Ok(ItemParsedWithContext {
                    raw_data,
                    item,
                    endpoint: endpoint.to_string(),
                }),
                Err(e) => Err(e),
            }
        })
        .collect()
}

pub fn filter_and_log_results<T>(results: Vec<Result<T, LPError>>) -> Vec<T> {
    results
        .into_iter()
        .filter_map(|res| match res {
            Ok(season) => Some(season),
            Err(e) => {
                tracing::warn!("{e}");
                None
            }
        })
        .collect()
}
