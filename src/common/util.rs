use tokio_retry::{
    RetryIf,
    strategy::{ExponentialBackoff, jitter},
};

use crate::{
    common::{db::DbContext, errors::LPError, models::DataSourceError},
    config::CONFIG,
};

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
macro_rules! verify_fk {
    ($missing:ident, $db_context:expr, $key:expr) => {
        ()
    };
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
macro_rules! with_spinner {
    ($msg:expr, |$pb:ident| $body:block) => {{
        let $pb = indicatif::ProgressBar::new_spinner();
        $pb.set_message($msg);
        $pb.enable_steady_tick(std::time::Duration::from_millis(100));
        $pb.set_style($crate::config::CONFIG.spinner_style.clone());
        let result = { $body };
        $pb.finish_using_style();
        result
    }};
}

#[macro_export]
macro_rules! sqlx_operation_with_retries {
    ($body:expr) => {
        $crate::common::util::sqlx_operation_with_retries(|| async { $body })
    };
}

#[macro_export]
macro_rules! reqwest_with_retries {
    ($body:expr) => {
        $crate::common::util::reqwest_with_retries(|| async { $body })
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

pub fn default_retry_strategy() -> impl Iterator<Item = std::time::Duration> {
    ExponentialBackoff::from_millis(CONFIG.retry_interval_ms)
        .max_delay(std::time::Duration::from_millis(
            CONFIG.retry_max_interval_ms,
        ))
        .map(jitter)
        .take(CONFIG.retries)
}

pub fn is_transient_sqlx_error(e: &sqlx::Error) -> bool {
    let is_transient = matches!(e, sqlx::Error::Io(_) | sqlx::Error::Tls(_));
    if is_transient {
        tracing::warn!("Retrying sqlx operation after transient error: {:?}", e);
    }
    is_transient
}

fn is_transient_reqwest_error(e: &reqwest::Error) -> bool {
    let is_transient = e.is_timeout() || e.is_connect();
    if is_transient {
        tracing::warn!("Retrying reqwest operation after transient error: {:?}", e);
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

#[tracing::instrument(skip(results, db_context))]
pub async fn track_and_filter_errors<T>(
    results: Vec<Result<T, LPError>>,
    db_context: &DbContext,
) -> Vec<T> {
    let futures = results.into_iter().map(|res| {
        let db_context = db_context;
        async move {
            match res {
                Ok(val) => Some(val),
                Err(e) => {
                    DataSourceError::track_error(e, db_context).await;
                    None
                }
            }
        }
    });
    futures::future::join_all(futures)
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect()
}
