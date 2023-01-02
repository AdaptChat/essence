use crate::snowflake::epoch_time;
use argon2_async::{set_config, Config};
use base64::{
    alphabet::URL_SAFE,
    decode_engine, encode_engine,
    engine::fast_portable::{FastPortable, NO_PAD},
};
use std::{sync::OnceLock, time::UNIX_EPOCH};

pub use argon2_async::{hash as hash_password, verify as verify_password};
pub use ring::rand::{SecureRandom, SystemRandom};
pub static RNG: OnceLock<SystemRandom> = OnceLock::new();

/// Configures and initializes the Argon2 hasher. This must be called before using the hasher.
pub async fn configure_hasher(secret_key: &'static [u8]) {
    let mut config = Config::new();

    config
        .set_secret_key(Some(secret_key))
        .set_memory_cost(4096)
        .set_iterations(64);

    set_config(config).await;
}

/// Returns a reference to the system RNG.
#[inline]
pub fn get_system_rng() -> &'static SystemRandom {
    RNG.get_or_init(SystemRandom::new)
}

const ENGINE: FastPortable = FastPortable::from(&URL_SAFE, NO_PAD);

/// Generates a new token for the given user ID.
///
/// # Token Format
/// ```text
/// MzkxMTM0MzUxMjc4MDg.MTg0NjAzMTg2.khHChSMQuhJ8hqj3QVp1HZjqjVlBRbXuxdsh7ri7FHU
/// ^ User ID           ^ Timestamp  ^ Random bytes
/// ```
///
/// Tokens are made of three sections, each separated by a period (`.`):
///
/// * Section 1 is the ID of the user that generated this token, cast as a string, and then encoded
/// using base64. (pseudocode: `to_base64(to_string(user_id))`)
/// * Section 2 is the timestamp of when the token was generated represented as milliseconds since
/// the Adapt epoch (see [`crate::snowflake::EPOCH_MILLIS`]), cast as a string, and then encoded
/// using base64. (pseudocode: `to_base64(to_string(unix_timestamp_millis - EPOCH_MILLIS))`)
/// * Section 3 is 32 random bytes encoded using base64.
///
/// # See Also
/// * [`TokenReader`] for a type that can decode tokens.
#[must_use]
pub fn generate_token(user_id: u64) -> String {
    let mut token = encode_engine(user_id.to_string().as_bytes(), &ENGINE);

    token.push('.');
    token.push_str(&encode_engine(epoch_time().to_string().as_bytes(), &ENGINE));
    token.push('.');
    token.push_str(&{
        let dest = &mut [0_u8; 32];
        get_system_rng().fill(dest).expect("could not fill bytes");

        encode_engine(dest, &ENGINE)
    });
    token
}

/// Reads information from a token.
pub struct TokenReader<'a>(&'a str, &'a str);

impl<'a> TokenReader<'a> {
    /// Creates a new token reader. Returns ``None`` if the token is invalid.
    #[inline]
    #[must_use]
    pub fn new(token: &'a str) -> Option<Self> {
        let mut split = token.splitn(3, '.');

        Some(Self(split.next()?, split.next()?))
    }

    /// Returns the user ID from the token. Returns ``None`` if the token is invalid.
    #[inline]
    #[must_use]
    pub fn user_id(&self) -> Option<u64> {
        decode_engine(self.0, &ENGINE)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|s| s.parse().ok())
    }

    /// Returns the timestamp from the token as a Unix timestamp in milliseconds.
    #[inline]
    #[must_use]
    pub fn timestamp_millis(&self) -> Option<u64> {
        decode_engine(self.1, &ENGINE)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|s| s.parse().ok())
    }

    /// Returns the timestamp from the token as a Unix timestamp in seconds.
    #[inline]
    #[must_use]
    pub fn timestamp_secs(&self) -> Option<u64> {
        self.timestamp_millis().map(|t| t / 1000)
    }

    /// Returns the timestamp from the token as a [`SystemTime`].
    #[inline]
    #[must_use]
    pub fn timestamp(&self) -> Option<std::time::SystemTime> {
        self.timestamp_millis()
            .map(std::time::Duration::from_millis)
            .map(|t| UNIX_EPOCH + t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = generate_token(39_113_435_127_808);

        assert!(token.starts_with("MzkxMTM0MzUxMjc4MDg."));
    }

    #[test]
    fn test_parse_token() {
        let token = "MzkxMTM0MzUxMjc4MDg.MTg0NjAzMTg2.khHChSMQuhJ8hqj3QVp1HZjqjVlBRbXuxdsh7ri7FHU";
        let reader = TokenReader::new(token).unwrap();

        assert_eq!(reader.user_id(), Some(39_113_435_127_808));
        assert_eq!(reader.timestamp_millis(), Some(184_603_186));
    }
}
