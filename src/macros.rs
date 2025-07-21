#[macro_export]
macro_rules! serde_for_bitflags {
    (@bincode for $t:ty) => {
        #[cfg(feature = "db")]
        impl bincode::Encode for $t {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), bincode::error::EncodeError> {
                bincode::Encode::encode(&self.bits(), encoder)
            }
        }

        #[cfg(feature = "db")]
        impl bincode::Decode<()> for $t {
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
    (@serde($repr:ty) $tgt:ty => $openapi_format:ident; $minmax:expr_2021) => {
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
        serde_for_bitflags!(@bincode for $tgt);
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
    (i32: $t:ty) => { serde_for_bitflags!(@serde_signed(i32) $t => Int32); };
    (i16: $t:ty) => { serde_for_bitflags!(@serde_signed(i16) $t => Int32); };
    (i64: $t:ty) => { serde_for_bitflags!(@serde_signed(i64) $t => Int64); };
}
