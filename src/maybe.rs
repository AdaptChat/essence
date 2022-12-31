use serde::{de::Deserialize, ser::Serialize, Deserializer, Serializer};
#[cfg(feature = "openapi")]
use utoipa::{
    openapi::{OneOfBuilder, RefOr, Schema},
    ToSchema,
};

/// A serde value that distinguishes between null and missing values.
///
/// # Note
/// When used as a field in a serializable type (although not needed for deserialization), the
/// attribute `#[serde(default, skip_serializing_if = "Maybe::is_absent")]` must be placed on the
/// field.
///
/// When used as a field in a deserialization, the attribute `#[serde(default)]` must be placed on
/// the field.
#[derive(Clone, Debug, Default)]
pub enum Maybe<T> {
    /// The field is absent.
    #[default]
    Absent,
    /// The field is present but is set to `null`.
    Null,
    /// The field is present and is set to a value, `T`.
    Value(T),
}

impl<T> Maybe<T> {
    /// Returns `true` if the value is `Absent`.
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// Maps the inner value of `Maybe` to a new value.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Maybe<U> {
        match self {
            Self::Value(v) => Maybe::Value(f(v)),
            Self::Null => Maybe::Null,
            Self::Absent => Maybe::Absent,
        }
    }

    /// Turns this into an `Option`, but if the value is `Absent`, the given fallback value is used
    /// instead.
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_option_or_if_absent(self, fallback: Option<T>) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            Self::Null => None,
            Self::Absent => fallback,
        }
    }

    /// Turns this into an `Option`.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        self.into()
    }
}

impl<T> From<Option<T>> for Maybe<T> {
    fn from(opt: Option<T>) -> Self {
        opt.map_or(Self::Null, Self::Value)
    }
}

impl<T> From<Maybe<T>> for Option<T> {
    fn from(maybe: Maybe<T>) -> Self {
        match maybe {
            Maybe::Value(v) => Some(v),
            _ => None,
        }
    }
}

impl<'de, T> Deserialize<'de> for Maybe<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::deserialize(deserializer).map(Into::into)
    }
}

impl<T: Serialize> Serialize for Maybe<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Value(v) => v.serialize(serializer),
            Self::Absent => Err(serde::ser::Error::custom(
                "Maybe fields need to be annotated with \
                    `#[serde(default, skip_serializing_if = \"Maybe::is_absent\")]`",
            )),
        }
    }
}

#[cfg(feature = "openapi")]
impl<T: ToSchema> ToSchema for Maybe<T> {
    fn schema() -> RefOr<Schema> {
        RefOr::T(Schema::OneOf(
            OneOfBuilder::new().item(T::schema()).nullable(true).build(),
        ))
    }
}
