pub type DbPool = sqlx::Pool<sqlx::Postgres>;
pub struct SqlxJob {
    pub query: StaticPgQuery,
    pub result_tx: tokio::sync::oneshot::Sender<SqlxJobResult>,
}
pub enum SqlxJobOrFlush {
    Job(SqlxJob),
    Flush,
}
pub type SqlxJobResult = Result<sqlx::postgres::PgQueryResult, sqlx::Error>;
pub type SqlxJobSender = tokio::sync::mpsc::Sender<SqlxJobOrFlush>;
pub type SqlxTransaction = sqlx::Transaction<'static, sqlx::Postgres>;
pub type StaticPgQuery = sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments>;
pub type StaticPgQueryAs<T> =
    sqlx::query::QueryAs<'static, sqlx::Postgres, T, sqlx::postgres::PgArguments>;

pub mod db_entity;
pub mod init;
pub mod worker;

pub use db_entity::{DbEntity, DbEntityVecExt, PrimaryKey};
pub use init::DbContext;
pub use worker::start_sqlx_worker;
