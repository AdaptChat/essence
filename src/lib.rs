#![cfg_attr(feature = "db", feature(once_cell))]
#![cfg_attr(any(feature = "auth", feature = "db"), feature(is_some_and))]
#![allow(
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown
)]

#[cfg(any(feature = "auth", feature = "token-parsing"))]
pub mod auth;
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
pub use permissions::calculate_permissions;

#[macro_export]
macro_rules! serde_for_bitflags {
    (@openapi for $t:ty => $format:ident) => {
        #[cfg(feature = "openapi")]
        impl utoipa::ToSchema for $t {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
                utoipa::openapi::RefOr::T(
                    utoipa::openapi::ObjectBuilder::new()
                        .schema_type(utoipa::openapi::SchemaType::Integer)
                        .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(
                            utoipa::openapi::KnownFormat::$format,
                        )))
                        .build()
                        .into(),
                )
            }
        }
    };
    (u32: $t:ty) => {
        use ::std::result::Result;

        impl serde::Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_u32(self.bits)
            }
        }

        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = u32::deserialize(deserializer)?;

                Self::from_bits(raw).ok_or(serde::de::Error::custom(format!(
                    "invalid bitflags value: {} (expected an integer between {} and {})",
                    raw,
                    0,
                    Self::all().bits(),
                )))
            }
        }

        serde_for_bitflags!(@openapi for $t => Int32);
    };
    (i64: $t:ty) => {
        impl serde::Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_i64(self.bits)
            }
        }

        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = i64::deserialize(deserializer)?;

                let max = Self::all().bits();
                let (min, max) = if max > 0 { (0, max) } else { (i64::MIN, i64::MAX) };

                Self::from_bits(raw).ok_or(serde::de::Error::custom(format!(
                    "invalid bitflags value: {} (expected an integer between {} and {})",
                    raw,
                    min,
                    max,
                )))
            }
        }

        serde_for_bitflags!(@openapi for $t => Int64);
    };
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
