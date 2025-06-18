use futures::future::join_all;
use tokio_retry::RetryIf;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

use crate::config;

#[macro_export]
macro_rules! sqlx_operation_with_retries {
    ($body:expr) => {
        $crate::util::sqlx_operation_with_retries(|| async { $body }).await
    };
}

#[macro_export]
macro_rules! reqwest_with_retries {
    ($body:expr) => {
        $crate::util::reqwest_with_retries(|| async { $body }).await
    };
}

pub async fn run_futures_concurrently<I, F, T, E>(futures: I) -> Result<Vec<T>, E>
where
    I: IntoIterator<Item = F>,
    F: std::future::Future<Output = Result<T, E>>,
{
    let results = join_all(futures).await;

    // collect errors
    results.into_iter().collect()
}

pub fn default_retry_strategy() -> impl Iterator<Item = std::time::Duration> {
    ExponentialBackoff::from_millis(config::ENVIRONMENT_VARIABLES.retry_jitter_duration_ms)
        .map(jitter)
        .take(config::ENVIRONMENT_VARIABLES.retries)
}

pub fn is_transient_sqlx_error(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Io(_) | sqlx::Error::Tls(_))
}

pub fn is_transient_reqwest_error(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect()
}

pub async fn sqlx_operation_with_retries<F, Fut, T>(operation: F) -> Result<T, sqlx::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    RetryIf::spawn(default_retry_strategy(), operation, is_transient_sqlx_error).await
}

pub async fn reqwest_with_retries<F, Fut, T>(operation: F) -> Result<T, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
{
    RetryIf::spawn(
        default_retry_strategy(),
        operation,
        is_transient_reqwest_error,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::time::Duration;

    #[tokio::test]
    async fn test_run_futures_concurrently_all_ok() {
        let futures: Vec<Pin<Box<dyn Future<Output = Result<i32, &'static str>> + Send>>> = vec![
            Box::pin(async { Ok::<_, &'static str>(1) }),
            Box::pin(async { Ok::<_, &'static str>(2) }),
            Box::pin(async { Ok::<_, &'static str>(3) }),
        ];
        let res = run_futures_concurrently(futures).await;
        assert_eq!(res, Ok(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn test_run_futures_concurrently_with_error() {
        let futures: Vec<Pin<Box<dyn Future<Output = Result<i32, &'static str>> + Send>>> = vec![
            Box::pin(async { Ok::<_, &'static str>(1) }),
            Box::pin(async { Err::<i32, _>("fail") }),
            Box::pin(async { Ok::<_, &'static str>(3) }),
        ];
        let res = run_futures_concurrently(futures).await;
        assert_eq!(res, Err("fail"));
    }

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
