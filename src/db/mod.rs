pub type DbPool = sqlx::Pool<sqlx::Postgres>;

pub mod init;
pub mod persistable;
