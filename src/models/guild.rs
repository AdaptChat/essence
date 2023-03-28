use crate::{
    models::{GuildChannel, Role, User},
    serde_for_bitflags,
};
use chrono::{DateTime, Utc};
#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Potentially a partial user.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(untagged)]
pub enum MaybePartialUser {
    /// A user with full information.
    Full(User),
    /// A user with only an ID.
    Partial { id: u64 },
}

/// Represents a member of a guild. Members are user objects associated with a guild.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Member {
    /// The user associated with this member. This could be `None` in some cases.
    #[serde(flatten)]
    pub user: MaybePartialUser,
    /// The ID of the guild this member is in.
    pub guild_id: u64,
    /// The nickname of the member. `None` if the member has no nickname.
    pub nick: Option<String>,
    /// A list of IDs of the roles that the member has. This could be `None` in some cases.
    pub roles: Option<Vec<u64>>,
    /// The time that the member joined the guild.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub joined_at: DateTime<Utc>,
}

impl Member {
    /// The ID of the user associated with this member.
    #[inline]
    #[must_use]
    pub const fn user_id(&self) -> u64 {
        match &self.user {
            MaybePartialUser::Full(user) => user.id,
            MaybePartialUser::Partial { id } => *id,
        }
    }

    /// The display name of the member. This is the nickname if the member has one,
    /// else the username.
    ///
    /// If the user information is not available, this will return `None`.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        match &self.user {
            MaybePartialUser::Full(user) => Some(self.nick.as_deref().unwrap_or(&user.username)),
            MaybePartialUser::Partial { .. } => None,
        }
    }
}

/// Represents member counts for a guild.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct GuildMemberCount {
    /// The total number of members in the guild.
    pub total: u32,
    /// The number of members that are online. If this was part of a partial guild object, then
    /// this will be `None`.
    pub online: Option<u32>,
}

/// Represents a guild with partial information, sometimes referred to as a server.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialGuild {
    /// The snowflake ID of the guild.
    pub id: u64,
    /// The name of the guild.
    pub name: String,
    /// The description of the guild.
    pub description: Option<String>,
    /// The URL of the icon of the guild.
    pub icon: Option<String>,
    /// The URL of the banner of the guild.
    pub banner: Option<String>,
    /// The ID of the owner of the guild.
    pub owner_id: u64,
    /// Extra information about the guild represented through bitflags.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub flags: GuildFlags,
    /// The amount of members in the guild. This could be `None` at times. For partial guilds, the
    /// `online` field of this will also be `None`.
    pub member_count: Option<GuildMemberCount>,
    /// The vanity URL code of the guild. This solely includes the code, not the full URL.
    /// This is `None` if the guild does not have a vanity URL.
    ///
    /// Guilds have the ability to set vanity URLs once they surpass 100 non-bot members *and* have
    /// their visibility set to public. The vanity URL code can be between 3 and 32 characters long.
    pub vanity_url: Option<String>,
}

/// Represents a guild with all information, sometimes referred to as a server.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Guild {
    /// The information available to partial guilds, including the name and ID.
    #[serde(flatten)]
    pub partial: PartialGuild,
    /// A list of resolved members in the guild.
    ///
    /// This is only available during the following events:
    /// * Fetching the guild directly
    /// * The client retrieves the response after a request to join a guild through an invite
    /// * The client receives a ready event containing all guild data through the gateway.
    /// * The client receives a guild create event through the gateway.
    pub members: Option<Vec<Member>>,
    /// A list of resolved roles in the guild.
    ///
    /// This is only available during the following events:
    /// * Fetching the guild directly
    /// * The client retrieves the response after a request to join a guild through an invite
    /// * The client receives a ready event containing all guild data through the gateway.
    /// * The client receives a guild create event through the gateway.
    pub roles: Option<Vec<Role>>,
    /// A list of resolved channels in the guild.
    ///
    /// This is only available during the following events:
    /// * Fetching the guild directly
    /// * The client retrieves the response after a request to join a guild through an invite
    /// * The client receives a ready event containing all guild data through the gateway.
    /// * The client receives a guild create event through the gateway.
    pub channels: Option<Vec<GuildChannel>>,
}

bitflags::bitflags! {
    /// Represents extra metadata and features about a guild in a bitmask.
    #[derive(Default)]
    pub struct GuildFlags: u32 {
        /// The guild is a public guild.
        const PUBLIC = 1 << 0;
        /// The guild is a verified or official guild.
        const VERIFIED = 1 << 1;
        /// The guild has a vanity invite URL.
        const VANITY_URL = 1 << 2;
    }
}

serde_for_bitflags!(u32: GuildFlags);
