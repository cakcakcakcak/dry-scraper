use chrono;

use crate::{
    bind,
    common::{db::DbContext, errors::LPError},
};

#[derive(Clone)]
pub struct DataSourceError {
    pub error_message: String,
    pub occurred_at: chrono::NaiveDateTime,
}
impl DataSourceError {
    pub fn new(error: LPError) -> Self {
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
    pub async fn track_error(error: LPError, db_context: &DbContext) {
        Self::new(error).upsert_fire_and_forget(db_context).await
    }
}
