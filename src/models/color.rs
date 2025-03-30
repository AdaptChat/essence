use serde::{Deserialize, Serialize};
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

impl Gradient {
    /// Validates the gradient by ensuring that the stops are sorted by position and that the
    /// positions are between 0 and 1.
    pub fn validate(&self) -> crate::Result<()> {
        if !(0.0..std::f32::consts::TAU).contains(&self.angle) {
            return Err(crate::Error::InvalidField {
                field: "angle".to_string(),
                message: "Gradient angle must be in radians, between 0 and 2 * PI".to_string(),
            });
        }

        if self.stops.is_empty() {
            return Err(crate::Error::InvalidField {
                field: "stops".to_string(),
                message: "Gradient must have at least one stop".to_string(),
            });
        }

        if self.stops.len() > 8 {
            return Err(crate::Error::InvalidField {
                field: "stops".to_string(),
                message: "Gradient may only have at most 8 stops".to_string(),
            });
        }

        let mut last = 0.0;
        for stop in &self.stops {
            if stop.position < 0.0 || stop.position > 1.0 {
                return Err(crate::Error::InvalidField {
                    field: "stops".to_string(),
                    message: "Gradient stop position must be between 0 and 1".to_string(),
                });
            }

            if stop.position < last {
                return Err(crate::Error::InvalidField {
                    field: "stops".to_string(),
                    message: "Gradient stops must be sorted by position".to_string(),
                });
            }

            last = stop.position;
        }

        Ok(())
    }
}

/// A color that can either be solid or a linear gradient. Individual colors are specified as
/// integers between 0 and 16777215.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtendedColor {
    /// A solid color.
    Solid {
        /// The color of the solid color.
        color: u32,
    },
    /// A linear gradient of colors.
    Gradient(Gradient),
}

impl ExtendedColor {
    /// Validates the color if it is a gradient by ensuring that it is valid.
    pub fn validate(&self) -> crate::Result<()> {
        match self {
            Self::Solid { .. } => Ok(()),
            Self::Gradient(gradient) => gradient.validate(),
        }
    }
}

#[cfg(feature = "db")]
#[derive(sqlx::Type, Copy, Clone, Debug)]
#[sqlx(type_name = "gradient_stop")]
pub(crate) struct DbGradientStop {
    position: f32,
    color: i32,
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
            (Some(color), _) => Some(Self::Solid {
                color: color as u32,
            }),
            _ => None,
        }
    }

    #[must_use]
    #[cfg(feature = "db")]
    pub(crate) fn to_db(&self) -> (Option<i32>, Option<DbGradient>) {
        match self {
            Self::Solid { color } => (Some(*color as i32), None),
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
