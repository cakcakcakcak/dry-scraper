pub type DbPool = sqlx::Pool<sqlx::Postgres>;
pub type SqlxJob = (
    std::pin::Pin<Box<dyn Future<Output = SqlxJobResult> + Send>>,
    tokio::sync::oneshot::Sender<SqlxJobResult>,
);
pub type SqlxJobResult = Result<sqlx::postgres::PgQueryResult, sqlx::Error>;
pub type SqlxJobSender = tokio::sync::mpsc::Sender<SqlxJob>;
pub type StaticPgQuery = sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments>;
pub type StaticPgQueryAs<T> =
    sqlx::query::QueryAs<'static, sqlx::Postgres, T, sqlx::postgres::PgArguments>;

pub mod db_entity;
pub mod init;
pub mod worker;

pub use db_entity::{DbEntity, DbEntityVecExt, PrimaryKey};
pub use init::{DbContext, init_db_context};
pub use worker::start_sqlx_worker;
