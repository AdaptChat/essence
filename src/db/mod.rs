mod auth;
mod channel;
mod guild;
mod invite;
mod member;
mod message;
mod role;
mod user;

pub use auth::AuthDbExt;
pub use channel::ChannelDbExt;
pub use guild::GuildDbExt;
pub use invite::InviteDbExt;
pub use member::MemberDbExt;
pub use message::MessageDbExt;
pub use role::RoleDbExt;
pub use user::UserDbExt;
pub(crate) use user::{DbRelationship, DbRelationshipType};

pub use sqlx;
use sqlx::{
    postgres::{PgConnection, PgPoolOptions},
    Pool, Postgres, Transaction,
};
use std::sync::OnceLock;

/// The global database pool.
pub static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

/// Connects to the database. This should only be called once.
///
/// # Errors
/// * If the database connection fails.
pub(crate) async fn connect(url: &str) -> Result<(), sqlx::Error> {
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

pub trait DbExt<'t>: Sized + Send {
    type Executor: sqlx::PgExecutor<'static>;
    type Transaction: sqlx::PgExecutor<'t>;

    fn executor(&self) -> Self::Executor;
    fn transaction(&mut self) -> Self::Transaction;
}

impl DbExt<'static> for &'static Pool<Postgres> {
    type Executor = Self;
    type Transaction = Self::Executor;

    #[inline]
    fn executor(&self) -> Self::Executor {
        self
    }

    #[inline]
    fn transaction(&mut self) -> Self::Transaction {
        self
    }
}

impl<'t> DbExt<'t> for Transaction<'static, Postgres> {
    type Executor = &'static Pool<Postgres>;
    type Transaction = &'t mut PgConnection;

    #[inline]
    fn executor(&self) -> Self::Executor {
        get_pool()
    }

    #[inline]
    fn transaction(&mut self) -> Self::Transaction {
        // SAFETY: `self` will only be acted on while the transaction is still active.
        let transaction: &mut Transaction<'static, Postgres> = unsafe { std::mem::transmute(self) };
        &mut *transaction
    }
}
