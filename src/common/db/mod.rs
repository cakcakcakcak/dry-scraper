use std::hash::{Hash, Hasher};

/// Type-erased key for the DB key cache.
/// Represents any entity key across all data sources.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CacheKey {
    pub source: &'static str,
    pub table: &'static str,
    pub id: String,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.table.hash(state);
        self.id.hash(state);
    }
}

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
