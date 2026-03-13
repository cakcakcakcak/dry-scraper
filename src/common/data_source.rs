use async_trait::async_trait;
use std::any::Any;

use crate::common::{app_context::AppContext, db::DbContext, errors::DSError};

/// trait for data sources (NHL, ESPN, etc). currently focused on cache warming.
/// job execution and routing will be added in phase 2 when JobSpec is designed.
#[async_trait]
pub trait DataSource: Send + Sync {
    fn name(&self) -> &'static str;

    async fn warm_cache(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
    ) -> Result<(), DSError>;

    fn as_any(&self) -> &dyn Any;
}
