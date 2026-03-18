use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    RetryIf,
};

use crate::config::Config;

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
macro_rules! sqlx_operation_with_retries {
    ($cfg:expr, $($body:tt)*) => {
        $crate::common::util::sqlx_operation_with_retries(|| async { $($body)* }, $cfg)
    };
}

#[macro_export]
macro_rules! reqwest_with_retries {
    ($cfg:expr, $($body:tt)*) => {
        $crate::common::util::reqwest_with_retries(|| async { $($body)* }, $cfg)
    };
}

#[macro_export]
macro_rules! impl_has_type_name {
    ($t:ty) => {
        impl $crate::common::models::traits::HasTypeName for $t {
            fn type_name() -> &'static str {
                stringify!($t)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_pk_debug {
    ($t:ty) => {
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.fmt_debug(f)
            }
        }
    };
}

pub fn default_retry_strategy(cfg: &Config) -> impl Iterator<Item = std::time::Duration> {
    ExponentialBackoff::from_millis(cfg.retry_interval_ms)
        .max_delay(std::time::Duration::from_millis(cfg.retry_max_interval_ms))
        .map(jitter)
        .take(cfg.retries)
}

pub fn is_transient_sqlx_error(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Io(_) | sqlx::Error::Tls(_))
}

fn is_transient_reqwest_error(e: &reqwest::Error) -> bool {
    e.is_timeout()
        || e.is_connect()
        || e.to_string().contains("IncompleteMessage")
        || e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
}

pub async fn sqlx_operation_with_retries<F, Fut, T>(
    operation: F,
    cfg: &Config,
) -> Result<T, sqlx::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    RetryIf::spawn(
        default_retry_strategy(cfg),
        || async { operation().await },
        is_transient_sqlx_error,
    )
    .await
}

pub async fn reqwest_with_retries<F, Fut, T>(
    operation: F,
    cfg: &Config,
) -> Result<T, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
{
    RetryIf::spawn(
        default_retry_strategy(cfg),
        || async { operation().await },
        is_transient_reqwest_error,
    )
    .await
}
