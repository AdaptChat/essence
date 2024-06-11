use std::collections::HashMap;

#[cfg(feature = "db")]
use crate::db::{DbRelationship, DbRelationshipType};
use crate::serde_for_bitflags;
use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Represents a user account.
///
/// A lot of information is stored in the user's flags, including whether or not the user is a bot
/// account.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct User {
    /// The snowflake ID of the user.
    pub id: u64,
    /// The username of the user.
    pub username: String,
    /// The display name of the user. This is `None` if the user has no display name.
    pub display_name: Option<String>,
    /// The URL of the user's avatar. This is `None` if the user has no avatar.
    pub avatar: Option<String>,
    /// The URL of the user's banner. This is `None` if the user has no banner.
    pub banner: Option<String>,
    /// The user's bio. This is `None` if the user has no bio.
    pub bio: Option<String>,
    /// A bitmask of extra information associated with this user.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub flags: UserFlags,
}

bitflags::bitflags! {
    /// A bitmask of extra information associated with a user.
    #[derive(Default)]
    pub struct UserFlags: u32 {
        /// The user is a bot account.
        const BOT = 1 << 0;
        /// The user has a verified email address.
        const VERIFIED = 1 << 1;
        /// The user is a maintainer of Adapt.
        const MAINTAINER = 1 << 2;
        /// The user is a contributor to Adapt.
        const CONTRIBUTOR = 1 << 3;
        /// The user has reported security issues or bugs within Adapt.
        const BUG_HUNTER = 1 << 4;
        /// The user has elevated privileges on the Adapt platform.
        const PRIVILEGED = 1 << 5;
    }
}

serde_for_bitflags!(u32: UserFlags);

/// Represents information such as the name and color of a guild folder.
/// This is only shown in the client's UI.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct GuildFolderInfo {
    /// The name of the folder.
    pub name: String,
    /// The color of the folder.
    pub color: u32,
}

/// Represents a folder that contains a collection of guilds. This is only shown in the client's UI.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct ClientUser {
    /// The public user info about the client.
    #[serde(flatten)]
    pub user: User,
    /// The associated email of the client's account.
    ///
    /// If the client is a bot, this is `None`.
    #[cfg_attr(feature = "utoipa", schema(format = "email"))]
    pub email: Option<String>,
    /// (Used internally) The hashed password of the client's account.
    ///
    /// This will never be present:
    /// * The **field** will exist if the `db` feature enabled, otherwise this field is not
    ///   present and guarded by a `cfg` attribute.
    /// * The **value** will always be `None` unless it is internally returned by the database.
    #[serde(skip)]
    #[cfg(feature = "db")]
    pub password: Option<String>,
    /// Controls who can open and/or send direct messages to the client.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub dm_privacy: PrivacyConfiguration,
    /// Controls who can add the client to group DMs.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub group_dm_privacy: PrivacyConfiguration,
    /// Controls who can request to add the client as a friend.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub friend_request_privacy: PrivacyConfiguration,
    /// Onboarding flags that indicate which onboarding steps the user has completed.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub onboarding_flags: UserOnboardingFlags,
    /// Bitmask of client settings.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub settings: Settings,
    /// A map for notification settings override.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub notification_override: HashMap<u64, NotificationFlags>,
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct Settings: i32 {
        /// Whether the user wants to receive push notifications.
        const NOTIFICATIONS = 1 << 0;
        /// Whether the user wants to always show guilds in the sidebar.
        const ALWAYS_SHOW_GUILDS_IN_SIDEBAR = 1 << 1;
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct NotificationFlags: i16 {
        const ALL = 1 << 0;
        const ALL_MENTIONS = 1 << 1;
        const DIRECT_MENTIONS = 1 << 2;
        const HIGHLIGHTS = 1 << 3;
    }
}

serde_for_bitflags!(i32: Settings);
serde_for_bitflags!(i16: NotificationFlags);

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

bitflags::bitflags! {
    /// Represents a privacy configuration.
    #[derive(Default)]
    pub struct PrivacyConfiguration: i16 {
        /// This configuration is public for friends.
        const FRIENDS = 1 << 0;
        /// This configuration is public for mutual friends (friends of friends).
        const MUTUAL_FRIENDS = 1 << 1;
        /// This configuration is public for users who share a guild with you.
        const GUILD_MEMBERS = 1 << 2;
        /// This configuration is public for everyone. This overwrites all other configurations.
        const EVERYONE = 1 << 3;

        // Aliases
        /// Default configuration for ``dm_privacy``.
        const DEFAULT_DM_PRIVACY = Self::FRIENDS.bits
            | Self::MUTUAL_FRIENDS.bits
            | Self::GUILD_MEMBERS.bits;
        /// Default configuration for ``group_dm_privacy``.
        const DEFAULT_GROUP_DM_PRIVACY = Self::FRIENDS.bits;
        /// Default configuration for ``friend_request_privacy``.
        const DEFAULT_FRIEND_REQUEST_PRIVACY = Self::EVERYONE.bits;
    }
}

serde_for_bitflags!(i16: PrivacyConfiguration);

bitflags::bitflags! {
    /// A bitmask of onboarding and tutorial steps that a user has completed.
    #[derive(Default)]
    pub struct UserOnboardingFlags: i64 {
        // Learn Adapt
        const CONNECT_WITH_FRIENDS = 1 << 0;
        const CREATE_A_COMMUNITY = 1 << 1;
        const DISCOVER_COMMUNITIES = 1 << 2;
    }
}

serde_for_bitflags!(i64: UserOnboardingFlags);

/// A section to include in a user's sidebar.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "snake_case")]
pub enum SidebarSection {
    /// Show unmuted channels that recently had activity. (Unread messages)
    UnreadMessages,
    /// Show channels you recently accessed.
    RecentChannels,
}

/// Represents the type of relationship a user has with another user.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// The other user is added as a friend.
    Friend,
    /// The client user has sent a friend request to the other user which is still pending.
    OutgoingRequest,
    /// The other user has sent a friend request to the client user which is still pending.
    IncomingRequest,
    /// The client user has blocked the other user.
    Blocked,
}

#[cfg(feature = "db")]
impl From<DbRelationshipType> for RelationshipType {
    #[inline]
    fn from(kind: DbRelationshipType) -> Self {
        match kind {
            DbRelationshipType::Friend => Self::Friend,
            DbRelationshipType::Incoming => Self::IncomingRequest,
            DbRelationshipType::Outgoing => Self::OutgoingRequest,
            DbRelationshipType::Blocked => Self::Blocked,
        }
    }
}

/// Represents a relationship that a user has with another user.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Relationship {
    /// The user that this relationship is with.
    pub user: User,
    /// The type of relationship this is.
    #[serde(rename = "type")]
    pub kind: RelationshipType,
}

#[cfg(feature = "db")]
impl crate::models::Relationship {
    /// Creates a new relationship from a database row.
    /// This is used internally by the database module.
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub(crate) fn from_db_relationship(data: DbRelationship) -> Self {
        Self {
            user: User {
                id: data.target_id as _,
                username: data.username,
                display_name: data.display_name,
                avatar: data.avatar,
                banner: data.banner,
                bio: data.bio,
                flags: UserFlags::from_bits_truncate(data.flags as _),
            },
            kind: RelationshipType::from(data.kind),
        }
    }
}
