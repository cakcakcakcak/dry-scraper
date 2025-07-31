pub type DbPool = sqlx::Pool<sqlx::Postgres>;
pub type SqlxJob = (
    std::pin::Pin<Box<dyn Future<Output = SqlxJobResult> + Send>>,
    tokio::sync::oneshot::Sender<SqlxJobResult>,
);
pub type SqlxJobResult = Result<sqlx::postgres::PgQueryResult, sqlx::Error>;
pub type SqlxJobSender = tokio::sync::mpsc::Sender<SqlxJob>;
pub type StaticPgQuery = sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments>;

pub mod init;
pub mod persistable;
pub mod worker;

pub use init::{DbContext, init_db_context};
pub use persistable::{Persistable, PrimaryKey, RelationshipIntegrity};
pub use worker::start_sqlx_worker;
