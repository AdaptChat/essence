pub mod user;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::OnceLock;

/// The global database pool.
pub static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

/// Connects to the database. This should only be called once.
///
/// # Errors
/// * If the database connection fails.
pub async fn connect() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .connect(dotenv!(
            "DATABASE_URL",
            "DATABASE_URL environment variable not set"
        ))
        .await?;

    POOL.set(pool)
        .expect("cannot initialize database pool more than once");
    Ok(())
}

/// Retrieves the database pool.
#[must_use]
#[inline]
pub fn get_pool() -> &'static Pool<Postgres> {
    POOL.get().expect("database pool not initialized")
}

/// Migrates the database.
pub async fn migrate() {
    sqlx::migrate!("./migrations")
        .run(get_pool())
        .await
        .expect("could not run database migrations");
}
