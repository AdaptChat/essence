pub mod auth;
pub mod user;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Transaction};
use std::sync::OnceLock;

/// The global database pool.
pub static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

/// Connects to the database. This should only be called once.
///
/// # Errors
/// * If the database connection fails.
pub async fn connect(url: &str) -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new().connect(url).await?;

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

pub trait DbExt<'a>: Sized
where
    Self: 'a,
{
    type Executor: sqlx::PgExecutor<'a>;
    type Transaction: sqlx::PgExecutor<'a>;

    fn executor(&'a self) -> Self::Executor;
    fn transaction(&'a mut self) -> Self::Transaction;
}

impl<'a> DbExt<'a> for Pool<Postgres>
where
    Self: 'a,
{
    type Executor = &'a Self;
    type Transaction = Self::Executor;

    #[inline]
    fn executor(&'a self) -> Self::Executor {
        self
    }

    #[inline]
    fn transaction(&'a mut self) -> Self::Transaction {
        self
    }
}

impl<'a> DbExt<'a> for Transaction<'a, Postgres>
where
    Self: 'a,
{
    type Executor = &'a Pool<Postgres>;
    type Transaction = &'a mut Self;

    #[inline]
    fn executor(&'a self) -> Self::Executor {
        get_pool()
    }

    #[inline]
    fn transaction(&'a mut self) -> Self::Transaction {
        self
    }
}
