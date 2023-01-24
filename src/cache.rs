#![allow(clippy::must_use_candidate)]

use crate::models::{ChannelType, Permissions, UserFlags};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

static LOCAL_CACHE: OnceLock<Arc<RwLock<Cache>>> = OnceLock::new();

/// Initializes the local cache.
pub fn setup() {
    LOCAL_CACHE
        .set(Arc::new(RwLock::new(Cache::default())))
        .expect("failed to initialize local cache");
    // invalidate cache every 30 minutes
    spawn_invalidator(Duration::from_secs(1800));
}

/// Spawns a cache invalidator task. This is a task that runs in the background and periodically
/// invalidates the cache every specified interval. This will be removed once a proper shared cache
/// strategy is implemented.
pub fn spawn_invalidator(interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            *write().await = Cache::default();
        }
    });
}

/// Acquires the cache for reading.
pub async fn read() -> RwLockReadGuard<'static, Cache> {
    LOCAL_CACHE
        .get()
        .expect("local cache not initialized")
        .read()
        .await
}

/// Acquires the cache for writing.
pub async fn write() -> RwLockWriteGuard<'static, Cache> {
    LOCAL_CACHE
        .get()
        .expect("local cache not initialized")
        .write()
        .await
}

pub type ChannelInspection = (Option<u64>, Option<u64>, ChannelType);

/// Caches database data in memory for faster access. This may be migrated to a microservice or
/// Redis in the future for shared access through multiple nodes.
#[derive(Debug, Default)]
pub struct Cache {
    /// Maps tokens to their associated user ID and flags.
    pub tokens: HashMap<String, (u64, UserFlags)>,
    /// Maps guild IDs to their associated guild caches.
    pub guilds: HashMap<u64, GuildCache>,
    /// Stores a `HashSet` of all known guild IDs to exist.
    pub existing_guild_ids: Option<HashSet<u64>>,
    /// Maps channel IDs to their inspection data.
    pub channels: HashMap<u64, ChannelInspection>,
}

impl Cache {
    /// Returns a reference to the guild cache for the given guild ID, if it exists.
    pub fn guild(&self, guild_id: u64) -> Option<&GuildCache> {
        self.guilds.get(&guild_id)
    }

    /// Returns a mutable reference to the guild cache for the given guild ID, if it exists.
    pub fn guild_mut(&mut self, guild_id: u64) -> Option<&mut GuildCache> {
        self.guilds.get_mut(&guild_id)
    }

    /// Returns the user ID associated with the given token, if it is cached. Otherwise, returns
    /// `None`.
    pub fn user_info_for_token(&self, token: impl AsRef<str>) -> Option<(u64, UserFlags)> {
        self.tokens.get(token.as_ref()).copied()
    }

    /// Caches a user ID for the given token.
    pub fn cache_token(&mut self, token: String, user_id: u64, flags: UserFlags) {
        self.tokens.insert(token, (user_id, flags));
    }

    /// Invalidates the cache mapping to user ID for the given token.
    pub fn invalidate_token(&mut self, token: impl AsRef<str>) {
        self.tokens.remove(token.as_ref());
    }

    /// Invalidates all tokens for the given user ID.
    pub fn invalidate_tokens_for(&mut self, user_id: u64) {
        self.tokens.retain(|_, (id, _)| *id != user_id);
    }
}

/// Caches guild data in memory for faster access.
#[derive(Debug, Default)]
pub struct GuildCache {
    /// Stores a `HashSet` of all member IDs in the guild.
    pub members: Option<HashSet<u64>>,
    /// Stores the owner ID of the guild.
    pub owner_id: Option<u64>,
    /// Stores calculated permissions both guild-wide and for every channel. Maps user IDs to
    /// another mapping of channel IDs (or None) to permissions.
    pub member_permissions: HashMap<u64, HashMap<Option<u64>, Permissions>>,
}

impl GuildCache {
    /// Returns whether or not the given user ID is a member of the guild. Returns `None` if this
    /// information is not cached.
    pub fn is_member(&self, user_id: u64) -> bool {
        self.members
            .as_ref()
            .is_some_and(|members| members.contains(&user_id))
    }

    /// Returns whether or not the given user ID is the owner of the guild. Returns `None` if this
    /// information is not cached.
    pub fn is_owner(&self, user_id: u64) -> bool {
        self.owner_id.is_some_and(|owner_id| owner_id == user_id)
    }

    /// Returns the calculated permissions for the given user ID in the given channel ID, if they
    /// are cached. Otherwise, returns `None`.
    pub fn permissions_for(&self, user_id: u64, channel_id: Option<u64>) -> Option<Permissions> {
        self.member_permissions
            .get(&user_id)
            .and_then(|map| map.get(&channel_id).copied())
    }
}
