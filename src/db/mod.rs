pub type DbPool = sqlx::Pool<sqlx::Postgres>;

pub mod init;
pub mod persistable;

pub use init::init_db;
pub use persistable::Persistable;
