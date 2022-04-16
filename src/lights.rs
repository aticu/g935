//! Configuration structs and stuff for headset lighting

use crate::{AsBytes, FromBytes};

/// Describes which light to configure
#[derive(Debug, Clone, Copy)]
pub enum Light {
    /// The logo light
    Logo,
    /// The main lights on the side
    Side,
}

/// Configuration for the light effect
#[derive(Debug, Clone, Copy)]
pub enum Effect {
    /// Settings for the off effect
    Off,
    /// Settings for the static color effect
    Static {
        /// Red value
        red: u8,
        /// Blue value
        green: u8,
        /// Green value
        blue: u8,
    },
    /// Settings for the breathing effect
    Breathing {
        /// Red value
        red: u8,
        /// Green value
        green: u8,
        /// Blue value
        blue: u8,
        /// The rate of the breathing effect
        rate: u16,
        /// Light brightness
        brightness: u8,
    },
    /// Settings for the color cycle effect
    ColorCycle {
        /// The rate of the cycle effect
        rate: u16,
        /// Light brightness
        brightness: u8,
    },
}

impl Default for Effect {
    fn default() -> Self {
        Effect::Off
    }
}

/// Profile type (default or not)
#[derive(Debug, Clone, Copy)]
pub enum ProfileType {
    /// Temporarily set (until next power-on)
    Temporary,
    /// Permanently store setting in device (don't apply now)
    Permanent,
}

/// Headset light configuration
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Which light to configure
    pub light: Light,
    /// Configuration for the effect
    pub effect: Effect,
    /// Profile type - unknown exactly how this works, but 2 seems to be the "device profile" and 0 non-default
    pub profile_type: ProfileType,
}

impl AsBytes for Config {
    fn as_bytes(&self) -> Vec<u8> {
        let mut params = vec![0u8; 13];

        params[0] = match self.light {
            Light::Logo => 0x00,
            Light::Side => 0x01,
        };

        params[1] = match self.effect {
            Effect::Off => 0x00,
            Effect::Static { .. } => 0x01,
            Effect::Breathing { .. } => 0x02,
            Effect::ColorCycle { .. } => 0x03,
        };

        match self.effect {
            Effect::Off => (),
            Effect::Static { red, green, blue } => {
                params[2] = red;
                params[3] = green;
                params[4] = blue;
            }
            Effect::Breathing {
                red,
                green,
                blue,
                rate,
                brightness,
            } => {
                params[2] = red;
                params[3] = green;
                params[4] = blue;
                params[5..7].copy_from_slice(&rate.to_be_bytes());
                params[8] = brightness;
            }
            Effect::ColorCycle { rate, brightness } => {
                params[7..9].copy_from_slice(&rate.to_be_bytes());
                params[9] = brightness;
            }
        }

        params[12] = match self.profile_type {
            ProfileType::Temporary => 0,
            ProfileType::Permanent => 2,
        };

        params
    }
}

impl FromBytes for Config {
    fn from_bytes(bytes: &[u8]) -> Self {
        assert!(
            bytes[0] <= 1,
            "Light index is out of range: was {}",
            bytes[0]
        );
        assert!(
            bytes[1] <= 3,
            "Light effect is out of range: was {}",
            bytes[1]
        );
        assert!(
            bytes[12] == 0 || bytes[12] == 2,
            "Light profile type was out of range: was {}",
            bytes[12]
        );

        Self {
            light: match bytes[0] {
                0 => Light::Logo,
                1 => Light::Side,
                _ => unreachable!(),
            },
            effect: match bytes[1] {
                0 => Effect::Off,
                1 => Effect::Static {
                    red: bytes[2],
                    green: bytes[3],
                    blue: bytes[4],
                },
                2 => Effect::Breathing {
                    red: bytes[2],
                    green: bytes[3],
                    blue: bytes[4],
                    rate: u16::from_be_bytes(bytes[5..7].try_into().unwrap()),
                    brightness: bytes[8],
                },
                3 => Effect::ColorCycle {
                    rate: u16::from_be_bytes(bytes[7..9].try_into().unwrap()),
                    brightness: bytes[9],
                },
                _ => unreachable!(),
            },
            profile_type: match bytes[12] {
                0 => ProfileType::Temporary,
                2 => ProfileType::Permanent,
                _ => unreachable!(),
            },
        }
    }
}
