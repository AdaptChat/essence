#![cfg_attr(feature = "db", feature(let_chains))]
#![cfg_attr(feature = "db", feature(trait_alias))]
#![cfg_attr(any(feature = "auth", feature = "db"), feature(is_some_and))]
#![cfg_attr(feature = "async-trait", feature(async_fn_in_trait))]
#![cfg_attr(feature = "async-trait", allow(incomplete_features))]
#![allow(
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::significant_drop_tightening,
    clippy::collection_is_never_read // false positives, but when fixed this ignore can be removed
)]

#[cfg(any(feature = "auth", feature = "token-parsing"))]
pub mod auth;
pub mod bincode_impl;
#[cfg(feature = "db")]
pub mod cache;
#[cfg(feature = "db")]
pub mod db;
pub mod error;
pub mod http;
mod macros;
mod maybe;
pub mod models;
mod permissions;
#[cfg(feature = "snowflakes")]
pub mod snowflake;
pub mod ws;

pub use error::{Error, NotFoundExt, Result};
pub use maybe::Maybe;
pub use permissions::{calculate_permissions, calculate_permissions_sorted};
#[cfg(feature = "utoipa")]
pub use utoipa;

#[cfg(feature = "db")]
pub async fn connect(db_url: &str, redis_url: &str) -> sqlx::Result<()> {
    db::connect(db_url).await?;
    cache::connect(redis_url);

    Ok(())
}
