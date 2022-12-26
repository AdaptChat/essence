use crate::{builder_methods, serde_for_bitflags};
use serde::{Deserialize, Serialize};

#[cfg(feature = "db")]
use crate::get_pool;

/// Represents a user account.
///
/// A lot of information is stored in the user's flags, including whether or not the user is a bot
/// account.
#[derive(Clone, Debug, Default, Serialize)]
pub struct User {
    /// The snowflake ID of the user.
    pub id: u64,
    /// The username of the user.
    pub username: String,
    /// The discriminator of the user, between 0 and 9999.
    pub discriminator: u16,
    /// The URL of the user's avatar. This is `None` if the user has no avatar.
    pub avatar: Option<String>,
    /// The URL of the user's banner. This is `None` if the user has no banner.
    pub banner: Option<String>,
    /// The user's bio. This is `None` if the user has no bio.
    pub bio: Option<String>,
    /// A bitmask of extra information associated with this user.
    pub flags: UserFlags,
}

bitflags::bitflags! {
    /// A bitmask of extra information associated with a user.
    #[derive(Default)]
    pub struct UserFlags: u32 {
        /// The user is a bot account.
        const BOT = 1 << 0;
    }
}

serde_for_bitflags!(u32: UserFlags);

impl User {
    builder_methods! {
        id: u64 => set_id,
        username: String => set_username,
        discriminator: u16 => set_discriminator,
        avatar: String => set_avatar + Some,
        banner: String => set_banner + Some,
        bio: String => set_bio + Some,
        flags: UserFlags => set_flags,
    }
}

#[cfg(feature = "db")]
macro_rules! construct_user {
    ($data:ident) => {{
        User {
            id: $data.id as _,
            username: $data.username,
            discriminator: $data.discriminator as _,
            avatar: $data.avatar,
            banner: $data.banner,
            bio: $data.bio,
            flags: UserFlags::from_bits_truncate($data.flags as _),
        }
    }};
}

#[cfg(feature = "db")]
macro_rules! fetch_user {
    ($query:literal, $($arg:expr),* $(,)?) => {{
        let result = sqlx::query!($query, $($arg),*)
            .fetch_optional(get_pool())
            .await?
            .map(|r| construct_user!(r));

        Ok(result)
    }};
}

#[cfg(feature = "db")]
impl User {
    /// Fetches a user from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    pub async fn fetch_by_id(id: u64) -> sqlx::Result<Option<Self>> {
        fetch_user!("SELECT * FROM users WHERE id = $1", id as i64)
    }

    /// Fetches a user from the database with the given username and discriminator.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    pub async fn fetch_by_tag(username: &str, discriminator: u16) -> sqlx::Result<Option<Self>> {
        fetch_user!(
            "SELECT * FROM users WHERE username = $1 AND discriminator = $2",
            username,
            discriminator as i16,
        )
    }
}

/// Represents information such as the name and color of a guild folder.
/// This is only shown in the client's UI.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuildFolderInfo {
    /// The name of the folder.
    pub name: String,
    /// The color of the folder.
    pub color: u32,
}

/// Represents a folder that contains a collection of guilds. This is only shown in the client's UI.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuildFolder {
    /// The path of the folder, with the top-level folder first.
    ///
    /// This is `None` if this folder represents the collection of guilds
    /// that are not in any folders, or in other terms, the root folder.
    pub path: Option<Vec<GuildFolderInfo>>,
    /// A list of guild IDs representing guilds that were placed in this folder, in order from
    /// top to bottom.
    pub guilds: Vec<u64>,
}

/// Represents user info about the client. This has other information that is not available to the
/// public, such as emails, guilds, and relationships (friends and blocked users).
#[derive(Clone, Debug, Default, Serialize)]
pub struct ClientUser {
    /// The public user info about the client.
    #[serde(flatten)]
    pub user: User,
    /// The associated email of the client's account.
    ///
    /// If the client is a bot, this is `None`.
    pub email: Option<String>,
    // /// A list of guilds that the client is a member of. This is a list of partial guilds that
    // /// include information such as the guild's ID, name, icon, and owner.
    // pub guilds: Vec<PartialGuild<u64>>,
    /// A list of relationships that the client has with other users.
    pub relationships: Vec<Relationship>,
    // /// A list of DM channels that the client has open.
    // pub dm_channels: Vec<DmChannel<u64>>,
}

impl std::ops::Deref for ClientUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

impl std::ops::DerefMut for ClientUser {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.user
    }
}

impl ClientUser {
    builder_methods! {
        email: String => set_email + Some,
        // guilds: Vec<PartialGuild<u64>> => set_guilds,
        relationships: Vec<Relationship> => set_relationships,
        // dm_channels: Vec<DmChannel<u64>> => set_dm_channels,
    }
}

#[cfg(feature = "db")]
impl ClientUser {
    /// Fetches the client user from the database.
    ///
    /// # Errors
    /// * If an error occurs with fetching the client user.
    pub async fn fetch(id: u64) -> sqlx::Result<Option<Self>> {
        let result = sqlx::query!("SELECT * FROM users WHERE id = $1", id as i64)
            .fetch_optional(get_pool())
            .await?
            .map(|r| Self {
                user: construct_user!(r),
                email: r.email,
                relationships: vec![],
            });

        Ok(result)
    }
}

/// Represents the type of relationship a user has with another user.
#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// The user is added as a friend.
    #[default]
    Friend,
    /// The user is blocked.
    Blocked,
}

/// Represents a relationship that a user has with another user.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Relationship {
    /// The ID of the user that this relationship is with.
    pub id: u64,
    /// The type of relationship this is.
    #[serde(rename = "type")]
    pub kind: RelationshipType,
}
