#![cfg_attr(feature = "db", feature(once_cell))]
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

#[macro_export]
macro_rules! bincode_for_bitflags {
    ($ty: ty) => {
        #[cfg(feature = "db")]
        impl bincode::Encode for $ty {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), bincode::error::EncodeError> {
                bincode::Encode::encode(&self.bits(), encoder)
            }
        }

        #[cfg(feature = "db")]
        impl bincode::Decode for $ty {
            fn decode<D: bincode::de::Decoder>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                Self::from_bits(bincode::Decode::decode(decoder)?).ok_or_else(|| {
                    bincode::error::DecodeError::OtherString(
                        "representation contains bits that do not correspond to a flag".to_string(),
                    )
                })
            }
        }
    };
}
#[macro_export]
macro_rules! serde_for_bitflags {
    (@openapi for $t:ty => $format:ident) => {
        #[cfg(feature = "utoipa")]
        impl utoipa::ToSchema<'static> for $t {
            fn schema() -> (&'static str, utoipa::openapi::RefOr<utoipa::openapi::Schema>) {
                (
                    stringify!($t),
                    utoipa::openapi::RefOr::T(
                        utoipa::openapi::ObjectBuilder::new()
                            .schema_type(utoipa::openapi::SchemaType::Integer)
                            .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(
                                utoipa::openapi::KnownFormat::$format,
                            )))
                            .build()
                            .into(),
                    )
                )
            }
        }
    };
    (@serde($repr:ty) $tgt:ty => $openapi_format:ident; $minmax:expr) => {
        impl serde::Serialize for $tgt {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.bits().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $tgt {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = <$repr as serde::Deserialize<'de>>::deserialize(deserializer)?;
                let (min, max) = $minmax;

                Self::from_bits(raw).ok_or(serde::de::Error::custom(format!(
                    "invalid bitflags value for {}: {} (expected an integer between {} and {})",
                    stringify!($tgt),
                    raw,
                    min,
                    max,
                )))
            }
        }

        serde_for_bitflags!(@openapi for $tgt => $openapi_format);
    };
    (@serde_signed($repr:ty) $tgt:ty => $openapi_format:ident) => {
        serde_for_bitflags!(
            @serde($repr) $tgt => $openapi_format;
            {
                const MAX: $repr = <$tgt>::all().bits();
                if MAX > 0 { (0, MAX) } else { (<$repr>::MIN, <$repr>::MAX) }
            }
        );
    };
    (@serde_unsigned($repr:ty) $tgt:ty => $openapi_format:ident) => {
        serde_for_bitflags!(
            @serde($repr) $tgt => $openapi_format;
            (0, <$tgt>::all().bits())
        );
    };

    (u32: $t:ty) => { serde_for_bitflags!(@serde_unsigned(u32) $t => Int32); };
    (i16: $t:ty) => { serde_for_bitflags!(@serde_signed(i16) $t => Int32); };
    (i64: $t:ty) => { serde_for_bitflags!(@serde_signed(i64) $t => Int64); };
}

#[macro_export]
macro_rules! builder_methods {
    ($($attr:ident: $t:ty => $name:ident $(+ $modifier:ident)?),+ $(,)?) => {
        $(
            #[doc = concat!("Changes the ``", stringify!($attr), "`` attribute.")]
            pub fn $name(&mut self, $attr: $t) -> &mut Self {
                self.$attr = $($modifier)? ($attr);
                self
            }
        )+
    };
}

pub async fn connect(db_url: &str, redis_url: &str) -> Result<()> {
    db::connect(db_url).await?;
    cache::connect(redis_url);

    Ok(())
}
