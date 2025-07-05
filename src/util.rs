use tokio_retry::RetryIf;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

use crate::config::CONFIG;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_retry_strategy_count_and_range() {
        let mut strategy = default_retry_strategy();
        let mut durations = vec![];
        while let Some(d) = strategy.next() {
            durations.push(d);
        }
        assert_eq!(durations.len(), 5);
        // all durations should be >= 0
        assert!(durations.iter().all(|d| *d >= Duration::from_millis(0)));
    }

    #[test]
    fn test_is_transient_sqlx_error() {
        let io_err = sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "io"));
        // apparently it is impossible to construct a tls error without a real tls error
        let decode_err = sqlx::Error::Decode("decode".into());

        assert!(is_transient_sqlx_error(&io_err));
        assert!(!is_transient_sqlx_error(&decode_err));
    }

    #[tokio::test]
    async fn test_is_transient_reqwest_error_timeout() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();

        // unroutable ip address should time out
        let res = client.get("http://10.255.255.123").send().await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(is_transient_reqwest_error(&err));
    }

    #[tokio::test]
    async fn test_is_transient_reqwest_error_connect() {
        // invalid port should fail to connect
        let res = reqwest::get("http://localhost:0").await; // invalid port, should fail to connect
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(is_transient_reqwest_error(&err));
    }
}
