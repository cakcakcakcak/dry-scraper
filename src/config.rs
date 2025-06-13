use dotenvy;
use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::PgPoolOptions;
use std::env;

pub async fn init_db() -> Result<sqlx::Pool<sqlx::Postgres>, sqlx::Error> {
    _ = match dotenvy::dotenv() {
        Ok(_) => (),
        Err(error) => println!(
            "A file named \".env\" should be present to at minimum define the database host, username, and password. Attempting to continue without one.\n{error:?}"
        ),
    };

    let db_url = database_url();

    // check if the database exists, if not, create it
    if !sqlx::Postgres::database_exists(&db_url).await? {
        sqlx::Postgres::create_database(&db_url).await?;
    }

    // create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    let is_dev = env::var("DEV_MODE").unwrap_or_else(|_| "false".to_string()) == "true";
    if is_dev {
        sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
            .execute(&pool)
            .await?;
    }

    sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
        .execute(&pool)
        .await?;

    // run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    Ok(pool)
}

fn database_url() -> String {
    let pg_host = env::var("PG_HOST").expect("Environment variable PG_HOST not set.");
    let pg_user = env::var("PG_USER").expect("Environment variable PG_USER not set.");
    let pg_pass = env::var("PG_PASS").expect("Environment variable PG_PASS not set.");

    let db_url = format!("postgres://{pg_user}:{pg_pass}@{pg_host}/lp");
    db_url
}
