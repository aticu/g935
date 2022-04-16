//! Battery and charging status related things.

use std::fmt;

use crate::FromBytes;

/// See https://github.com/Sapd/HeadsetControl/blob/master/src/devices/logitech_g633_g933_935.c
fn estimate_battery_level(voltage: u16) -> f32 {
    if voltage <= 3525 {
        0.03 * (voltage as f32) - 101.0
    } else if voltage > 4030 {
        100.0
    } else {
        let voltage = voltage as f32;

        0.000_000_003_726_847_3 * voltage.powf(4.0) - 0.000_056_056_262 * voltage.powf(3.0)
            + 0.315_605_2 * voltage.powf(2.0)
            - 788.093_75 * voltage
            + 736_315.3
    }
}

/// The current status of charging
#[derive(Debug, Eq, Hash, PartialEq, Copy, Clone)]
pub enum ChargingStatus {
    /// Battery is discharging
    Discharging,
    /// Battery is charging
    Charging,
    /// Battery is full
    Full,
}

impl fmt::Display for ChargingStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChargingStatus::Discharging => write!(f, "discharging"),
            ChargingStatus::Charging => write!(f, "charging"),
            ChargingStatus::Full => write!(f, "full"),
        }
    }
}

/// Battery status
#[derive(Debug)]
pub struct BatteryStatus {
    /// Charging status
    pub charging_status: ChargingStatus,
    /// Battery voltage
    pub voltage: u16,
    /// Charge percentage
    pub charge: f32,
}

impl FromBytes for BatteryStatus {
    fn from_bytes(bytes: &[u8]) -> Self {
        let charging_status = match bytes[2] {
            1 => ChargingStatus::Discharging,
            3 => ChargingStatus::Charging,
            7 => ChargingStatus::Full,
            s => {
                log::error!(
                    "encountered unknown charging status {}, defaulting to discharging",
                    s
                );
                ChargingStatus::Discharging
            }
        };

        let voltage = u16::from_be_bytes(bytes[0..2].try_into().unwrap());

        BatteryStatus {
            charging_status,
            voltage,
            charge: estimate_battery_level(voltage),
        }
    }
}
