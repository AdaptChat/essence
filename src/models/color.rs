use serde::{Deserialize, Serialize};
use sqlx::postgres::PgTypeInfo;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// A single color stop in a linear gradient.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct GradientStop {
    /// The position of the stop in the gradient, between 0 and 1.
    pub position: f32,
    /// The color of the stop.
    pub color: u32,
}

/// A variation of an extended color that represents a linear gradient. Note that gradients are
/// strictly linear and are provided in this format to allow for better consistency and ease of
/// implementation in clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Gradient {
    /// The angle of the gradient, in radians.
    pub angle: f32,
    /// The color stops of the gradient.
    pub stops: Vec<GradientStop>,
}

/// A color that can either be solid or a linear gradient. Individual colors are specified as
/// integers between 0 and 16777215.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtendedColor {
    /// A solid color.
    Solid(u32),
    /// A linear gradient of colors.
    Gradient(Gradient),
}

#[cfg(feature = "db")]
#[derive(sqlx::Type, Copy, Clone, Debug)]
#[sqlx(type_name = "gradient_stop")]
pub(crate) struct DbGradientStop {
    position: f32,
    color: i32,
}

impl sqlx::postgres::PgHasArrayType for DbGradientStop {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("gradient_stop[]")
    }

    fn array_compatible(_: &PgTypeInfo) -> bool {
        true
    }
}

#[cfg(feature = "db")]
#[derive(sqlx::Type, Clone, Debug)]
#[sqlx(type_name = "gradient_type")]
pub(crate) struct DbGradient {
    angle: f32,
    stops: Vec<DbGradientStop>,
}

impl ExtendedColor {
    /// Constructs an extended color from either a solid or gradient entry in the database.
    #[must_use]
    #[cfg(feature = "db")]
    pub(crate) fn from_db(color: Option<i32>, gradient: Option<&DbGradient>) -> Option<Self> {
        match (color, gradient) {
            (_, Some(gradient)) => {
                let stops = gradient
                    .stops
                    .iter()
                    .map(|s| GradientStop {
                        position: s.position,
                        color: s.color as u32,
                    })
                    .collect();

                Some(Self::Gradient(Gradient {
                    angle: gradient.angle,
                    stops,
                }))
            }
            (Some(color), _) => Some(Self::Solid(color as u32)),
            _ => None,
        }
    }

    #[must_use]
    #[cfg(feature = "db")]
    pub(crate) fn to_db(&self) -> (Option<i32>, Option<DbGradient>) {
        match self {
            Self::Solid(color) => (Some(*color as i32), None),
            Self::Gradient(gradient) => {
                let stops = gradient
                    .stops
                    .iter()
                    .map(|s| DbGradientStop {
                        position: s.position,
                        color: s.color as i32,
                    })
                    .collect();

                (
                    None,
                    Some(DbGradient {
                        angle: gradient.angle,
                        stops,
                    }),
                )
            }
        }
    }
}
