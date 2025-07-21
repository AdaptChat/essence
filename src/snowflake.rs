//! Snowflake generation and parsing.
//!
//! # Snowflake bit format
//! Snowflakes are represented as unsigned 64-bit integers (`u64`). The bits
//! (from left to right, 0-indexed, `inclusive..exclusive`) are as follows:
//!
//! * Bits 0..46: Timestamp in milliseconds since `2022-12-25T00:00:00Z`. (See [`EPOCH`])
//! * Bits 46..51: The model type represented as an enumeration. (See [`ModelType`])
//! * Bits 51..56: The node or process ID that generated the snowflake.
//! * Bits 56..64: The incrementing counter for the snowflake.
//!
//! ```text
//! 1111111111111111111111111111111111111111111111_11111_11111_11111111
//! milliseconds from 2022-12-25T00:00:00Z         ^     ^     ^
//!                                                |     |     increment (0 to 255)
//!                                                |     node number (0 to 31)
//!                                                model number (0 to 31)
//! ```

use crate::models::ModelType;
use regex::Regex;
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering::Relaxed},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static INCREMENT: AtomicU8 = AtomicU8::new(0);

/// The snowflake epoch. This is ``2022-12-25T00:00:00Z`` as a Unix timestamp, in milliseconds.
pub const EPOCH_MILLIS: u64 = 1_671_926_400_000;

/// Returns the current time in milliseconds since the epoch.
#[inline]
#[must_use]
pub fn epoch_time() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before UNIX epoch")
        .as_millis() as u64;

    now.saturating_sub(EPOCH_MILLIS)
}

/// Generates a snowflake with the given model type and node ID.
///
/// # Safety
/// This assumes that `node_id < 32`. If this is not the case, bits will flow and overwrite
/// other fields, resulting in an invalid snowflake.
#[inline]
#[must_use]
pub unsafe fn generate_snowflake_unchecked(model_type: ModelType, node_id: u8) -> u64 {
    let increment = INCREMENT.fetch_add(1, Relaxed);

    (epoch_time() << 18) | ((model_type as u64) << 13) | ((node_id as u64) << 8) | increment as u64
}

/// Generates a snowflake with the given model type and node ID.
///
/// # Panics
/// * If `node_id >= 32`.
#[inline]
#[must_use]
pub fn generate_snowflake(model_type: ModelType, node_id: u8) -> u64 {
    assert!(node_id < 32, "node ID must be less than 32");

    unsafe { generate_snowflake_unchecked(model_type, node_id) }
}

/// Returns the given snowflake with its model type altered to the given one.
#[inline]
#[must_use]
pub const fn with_model_type(snowflake: u64, model_type: ModelType) -> u64 {
    snowflake & !(0b11111 << 13) | (model_type as u64) << 13
}

/// Extract all snowflake IDs surrounded by <@!? and >, called mentions, from a string.
#[must_use]
pub fn extract_mentions(s: &str) -> Vec<u64> {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    let regex = REGEX.get_or_init(|| Regex::new(r"<@!?(\d+)>").unwrap());
    regex
        .captures_iter(s)
        .map(|c| c.get(1).unwrap().as_str().parse().unwrap())
        .collect::<Vec<_>>()
}

/// Reads parts of a snowflake.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SnowflakeReader(u64);

impl SnowflakeReader {
    /// Creates a new snowflake reader from the given snowflake.
    #[inline]
    #[must_use]
    pub const fn new(snowflake: u64) -> Self {
        Self(snowflake)
    }

    /// Reads and returns the timestamp of the snowflake as a Unix timestamp in milliseconds.
    #[inline]
    #[must_use]
    pub const fn timestamp_millis(&self) -> u64 {
        self.0 >> 18
    }

    /// Reads and returns the timestamp of the snowflake as a Unix timestamp in seconds.
    #[inline]
    #[must_use]
    pub const fn timestamp_secs(&self) -> u64 {
        self.timestamp_millis() / 1000
    }

    /// Reads and returns the timestamp of the snowflake as a [`SystemTime`].
    #[inline]
    #[must_use]
    pub fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.timestamp_millis())
    }

    /// Reads and returns the model type of the snowflake.
    #[inline]
    #[must_use]
    pub const fn model_type(&self) -> ModelType {
        ModelType::from_u8(((self.0 >> 13) & 0b11111) as u8)
    }

    /// Reads and returns the node ID of the snowflake.
    #[inline]
    #[must_use]
    pub const fn node_id(&self) -> u8 {
        ((self.0 >> 8) & 0b11111) as u8
    }

    /// Reads and returns the increment of the snowflake.
    #[inline]
    #[must_use]
    pub const fn increment(&self) -> u8 {
        (self.0 & 0b1111_1111) as u8
    }
}

impl From<u64> for SnowflakeReader {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<i64> for SnowflakeReader {
    fn from(value: i64) -> Self {
        Self(value as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_snowflake() {
        let a = generate_snowflake(ModelType::User, 0);
        let b = generate_snowflake(ModelType::User, 0);

        assert_ne!(a, b);
        println!("{} != {}", a, b);
    }

    #[test]
    fn test_parse_snowflake() {
        let snowflake = generate_snowflake(ModelType::Channel, 6);
        let reader = SnowflakeReader::new(snowflake);

        assert_eq!(reader.model_type(), ModelType::Channel);
        assert_eq!(reader.node_id(), 6);
    }

    #[test]
    fn test_with_model_type() {
        let original = generate_snowflake(ModelType::User, 0);
        let original_reader = SnowflakeReader::new(original);

        let new = with_model_type(original, ModelType::Channel);
        let new_reader = SnowflakeReader::new(new);

        assert_eq!(
            original_reader.timestamp_millis(),
            new_reader.timestamp_millis()
        );
        assert_eq!(original_reader.node_id(), new_reader.node_id());
        assert_eq!(original_reader.increment(), new_reader.increment());

        assert_eq!(original_reader.model_type(), ModelType::User);
        assert_eq!(new_reader.model_type(), ModelType::Channel);
    }
}
