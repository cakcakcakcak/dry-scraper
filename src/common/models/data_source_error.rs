use chrono;

use crate::{
    bind,
    common::{db::DbContext, errors::DSError},
};

/// Partition results into successes and errors, logging warnings and tracking errors.
///
/// This is the unified error handler for all operations that return `Vec<Result<T, E>>`:
/// - **External operations**: API fetches, data parsing, file operations
/// - **Internal operations**: Database upserts, batch processing
///
/// Both cases require the same handling:
/// 1. Log warnings with success/failure counts for visibility
/// 2. Track errors to database for later analysis/debugging
/// 3. Return successes to continue processing (partial success is valuable)
/// 4. Non-fatal - errors don't stop the entire operation
///
/// Returns the successful items and the count of errors.
///
/// # Examples
///
/// ```rust,ignore
/// // External: API fetch errors
/// let results = api.fetch_many_things(db_context).await?;
/// let (items, _) = partition_and_track_errors(
///     results,
///     db_context,
///     "Parse errors during thing fetch"
/// ).await;
///
/// // Internal: Database upsert errors
/// let results = items.upsert_all(app_context, db_context).await;
/// let (successful, failed) = partition_and_track_errors(
///     results,
///     db_context,
///     "Database upsert failures"
/// ).await;
/// ```
pub async fn partition_and_track_errors<T>(
    results: Vec<Result<T, DSError>>,
    db_context: &DbContext,
    operation_description: &str,
) -> (Vec<T>, usize) {
    let (items, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
    let items: Vec<_> = items.into_iter().map(Result::unwrap).collect();

    let error_count = errors.len();
    if error_count > 0 {
        tracing::warn!(
            successful = items.len(),
            failed = error_count,
            "{operation_description}"
        );
        for error in errors.into_iter().filter_map(Result::err) {
            DataSourceError::track_error(error, db_context).await;
        }
    }

    (items, error_count)
}

#[derive(Clone)]
pub struct DataSourceError {
    pub error_message: String,
    pub occurred_at: chrono::NaiveDateTime,
}
impl DataSourceError {
    pub fn new(error: DSError) -> Self {
        DataSourceError {
            error_message: error.to_string(),
            occurred_at: chrono::Local::now().naive_local(),
        }
    }
    pub async fn upsert_fire_and_forget(self, db_context: &DbContext) {
        let db_context = db_context.clone();
        tokio::spawn(async move {
            let _ = bind!(
                sqlx::query(
                    "INSERT INTO data_source_error (error_message, occurred_at) VALUES ($1, $2)"
                ),
                self.error_message,
                self.occurred_at,
            )
            .execute(&db_context.pool)
            .await;
        });
    }
    pub async fn track_error(error: DSError, db_context: &DbContext) {
        Self::new(error).upsert_fire_and_forget(db_context).await
    }
}
