use crate::models::{Bot, Devices, PartialGuild};
use crate::serde_for_bitflags;
#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

bitflags::bitflags! {
    /// A bitmask of flags which affect how the user's client should behave.
    #[derive(Default)]
    pub struct ClientFlags: i32 {
        /// Whether the user wants to receive push notifications.
        const PUSH_NOTIFICATIONS = 1 << 0;
        /// Whether the user wants to always show guilds in the sidebar.
        const ALWAYS_SHOW_GUILDS_IN_SIDEBAR = 1 << 1;
    }
}

serde_for_bitflags!(i32: ClientFlags);

bitflags::bitflags! {
    /// A bitmask of onboarding and tutorial steps that a user has completed.
    #[derive(Default)]
    pub struct UserOnboardingFlags: i64 {
        // Learn Adapt
        const CUSTOMIZE_YOUR_PROFILE = 1 << 0;
        const CONNECT_WITH_FRIENDS = 1 << 1;
        const DISCOVER_COMMUNITIES = 1 << 2;
    }
}

serde_for_bitflags!(i64: UserOnboardingFlags);

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(untagged)]
pub enum GuildPositioningEntry {
    /// A guild ID representing a guild in the sidebar.
    Guild(u64),
    /// A folder containing guilds.
    Folder {
        /// The name of the folder.
        name: String,
        /// The IDs of the guilds in this folder, in the order they should be displayed.
        guild_order: Vec<u64>,
        /// The hue this icon should be displayed with, represented as an integer between 0 and 255.
        hue: u8,
    },
}

/// Represents settings concerned with a front-facing client.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct ClientSettings {
    /// A bitmask of flags which affect how the user's client should behave.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub flags: ClientFlags,
    /// Onboarding flags that indicate which onboarding steps the user has completed.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub onboarding_flags: UserOnboardingFlags,
    /// An IETF BCP 47 compliant language tag, representing the user's preferred locale.
    pub locale: String,
    /// Represents the ordering of guilds shown in the client, including folders.
    /// This is a list of either guild IDs or guild folder objects.
    pub guild_order: Vec<GuildPositioningEntry>,
    /// Represents the ordering of DM channels shown in the client. Any DM channels not in this
    /// list are considered "hidden DMs".
    pub dm_channel_order: Vec<u64>,
    /// The theme the user has selected for their client.
    pub theme: ThemeReference,
    /// A list of plugins the user has enabled in their client.
    pub plugins: Vec<Plugin>,
}

/// A set of three preset themes the user can select from.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "lowercase")]
pub enum PresetTheme {
    /// Light theme
    Light,
    /// Dim theme
    #[default]
    Dim,
    /// Dark theme
    Dark,
}

/// A custom theme.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct CustomTheme {
    /// The ID of the theme.
    pub id: u64,
    /// The name of the theme.
    pub name: String,
}

/// Represents a theme a user has selected for their client.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum ThemeReference {
    /// A preset theme.
    Preset {
        /// The name of the preset theme.
        /// Currently supported values are: "light", "dim", and "dark".
        name: PresetTheme,
    },
    /// A reference to a published, discoverable theme on the marketplace.
    /// The client should fetch the theme from discovery when this is set.
    Published {
        /// The ID of the custom theme.
        id: u64,
        /// The desired version of the theme.
        version: String,
        /// A resolved snapshot of the marketplace entry associated with the theme, if available.
        details: Option<MarketplaceEntry>,
        /// A resolved snapshot of the theme at the time this data is sent, if available.
        /// This is always sent in "identify" payloads from harmony.
        snapshot: Option<CustomTheme>,
    },
    /// A custom theme which the user has created or imported themselves.
    Custom(CustomTheme),
}

impl Default for ThemeReference {
    fn default() -> Self {
        Self::Preset {
            name: PresetTheme::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PluginCompatibility {
    /// A bitmask of devices this plugin is compatible with.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub devices: Devices,
    /// A list of client implementations this plugin is compatible with. If empty or `None`, it
    /// will be assumed that only offical Adapt clients are supported.
    ///
    /// Known client implementations include:
    /// - `adapt-web` for the official web and desktop client
    pub implementations: Option<Vec<String>>,
    /// The minimum client version this plugin is compatible with, if any. This is arbitrarily
    /// formatted and is mainly up to your client implementation to decide how to handle.
    pub version: Option<String>,
}

impl Default for PluginCompatibility {
    fn default() -> Self {
        Self {
            devices: Devices::all(),
            implementations: Some(vec!["adapt-web".to_string()]),
            version: None,
        }
    }
}

/// Represents a plugin.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Plugin {
    /// The ID of the plugin.
    pub id: u64,
    /// The name of the plugin.
    pub name: String,
    /// Restricts the types of clients this plugin can run on.
    pub compatibility: PluginCompatibility,
    /// The manifest used to load and run the plugin. The format of the manifest varies
    /// based on client implementation, and it is unchecked in the backend. The plugin manifest
    /// may not exceed 256 KB.
    pub manifest: String,
}

/// Represents anything that is "discoverable".
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type", content = "resolved", rename_all = "lowercase")]
pub enum DiscoverableEntity {
    /// A guild.
    Guild(Option<PartialGuild>),
    /// A bot.
    Bot(Option<Bot>),
    /// A custom theme.
    Theme(Option<CustomTheme>),
    /// A plugin.
    Plugin(Option<Plugin>),
}

/// Represents an entry in discovery.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct DiscoveryEntry {
    /// The actual entity this entry represents.
    #[serde(flatten)]
    pub entity: DiscoverableEntity,
    /// The latest revision of this entry.
    pub revision: DiscoveryRevision,
}

/// Represents a revision of a listing in discovery.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct DiscoveryRevision {
    /// The revision ID of this entry, which also encodes when this entry was last updated.
    pub revision_id: u64,
    /// The ID of the user who created the entry.
    pub author_id: u64,
    /// A brief description for the entry.
    pub brief: String,
    /// A long description for the entry. This may contain markdown.
    pub description: String,
    /// The version of this entry.
    pub version: String,
    /// If this is a marketplace listing, optional additional metadata about the entry.
    #[serde(flatten)]
    pub marketplace: Option<MarketplaceEntry>,
}

/// Represents a marketplace entry for a discoverable entity (themes and plugins).
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct MarketplaceEntry {
    /// The ID of the marketplace entry.
    pub id: u64,
    /// The name of the entry.
    pub name: String,
    /// A URL representing the icon for the entry.
    pub icon: String,
    /// A URL representing the optional banner for the entry.
    pub banner: Option<String>,
    /// The number of times this entry has been used or installed.
    pub uses: u64,
    /// The number of upvotes this entry has received.
    pub upvotes: u64,
}
