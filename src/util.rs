use tokio_retry::RetryIf;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

use crate::config::CONFIG;
use crate::lp_error::LPError;

#[macro_export]
macro_rules! bind {
    ($query:expr, $($val:expr),+ $(,)?) => {{
        let mut q = $query;
        $(
            q = q.bind($val.clone());
        )+
        q
    }};
}

#[macro_export]
macro_rules! with_progress_bar {
    ($count:expr, |$pb:ident| $body:block) => {{
        let $pb = indicatif::ProgressBar::new($count as u64);
        $pb.set_style($crate::config::CONFIG.progress_bar_style.clone());
        let result = { $body };
        $pb.finish_using_style();
        result
    }};
}

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
            let res: Result<T, sqlx::Error> = operation().await;
            if let Err(ref e) = res {
                tracing::warn!("A sqlx operation failed after error: {:?}", e);
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
                tracing::warn!("A reqwest operation failed after error {:?}", e);
            }
            res
        },
        is_transient_reqwest_error,
    )
    .await
}

pub fn filter_results<T>(results: Vec<Result<T, LPError>>) -> Vec<T> {
    results.into_iter().filter_map(Result::ok).collect()
}
