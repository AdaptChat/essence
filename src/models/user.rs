use crate::{builder_methods, serde_for_bitflags};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Represents a user account.
///
/// A lot of information is stored in the user's flags, including whether or not the user is a bot
/// account.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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

/// Represents information such as the name and color of a guild folder.
/// This is only shown in the client's UI.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GuildFolderInfo {
    /// The name of the folder.
    pub name: String,
    /// The color of the folder.
    pub color: u32,
}

/// Represents a folder that contains a collection of guilds. This is only shown in the client's UI.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ClientUser {
    /// The public user info about the client.
    #[serde(flatten)]
    pub user: User,
    /// The associated email of the client's account.
    ///
    /// If the client is a bot, this is `None`.
    #[cfg_attr(feature = "openapi", schema(format = "email"))]
    pub email: Option<String>,
    /// (Used internally) The hashed password of the client's account. This will never be present.
    #[serde(skip)]
    pub password: Option<String>,
    // /// A list of DM channels that the client has open.
    // pub dm_channels: Vec<DmChannel<u64>>,
    // /// A list of guilds that the client is a member of. This is a list of partial guilds that
    // /// include information such as the guild's ID, name, icon, and owner.
    // pub guilds: Vec<PartialGuild<u64>>,
    /// A list of relationships that the client has with other users.
    pub relationships: Vec<Relationship>,
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

/// Represents a client user with addition to their password.

/// Represents the type of relationship a user has with another user.
#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum RelationshipType {
    /// The user is added as a friend.
    #[default]
    Friend,
    /// The user is blocked.
    Blocked,
}

/// Represents a relationship that a user has with another user.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Relationship {
    /// The ID of the user that this relationship is with.
    pub id: u64,
    /// The type of relationship this is.
    #[serde(rename = "type")]
    pub kind: RelationshipType,
}
