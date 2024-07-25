//! Programmatic access to the G935 headset.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]

mod battery;
mod buttons;
pub mod config;
mod device;
mod features;
pub mod lights;
mod power_state;

use buttons::{Buttons, MicArm, Wheel};
use config::Config;
use device::Device;
use features::FeatureMap;

pub use crate::{
    battery::{BatteryStatus, ChargingStatus},
    buttons::ButtonState,
    power_state::PowerState,
};

/// Convert a struct that implements this trait to bytes
trait AsBytes {
    /// Convert a struct that implements this trait to bytes
    fn as_bytes(&self) -> Vec<u8>;
}

/// Convert a series of bytes to a struct that implements this trait
trait FromBytes {
    /// Convert a series of bytes to a struct that implements this trait
    fn from_bytes(bytes: &[u8]) -> Self;
}

/// Represents an open connection to the headset.
#[derive(Debug)]
pub struct Headset {
    /// The inner device used for communication.
    device: Device,
    /// The features of the headset.
    features: FeatureMap,
}

impl Headset {
    /// Opens a connection to the headset.
    pub fn open() -> anyhow::Result<Self> {
        let api = hidapi::HidApi::new()?;
        let mut device = Device::new(api.open(0x046d, 0x0a87)?);

        let features = features::FeatureMap::initialize(&mut device)?;

        log::debug!("read feature map: {:?}", features);

        let mut headset = Self { device, features };

        let (ver1, ver2) = headset.get_protocol_version()?;
        if (ver1, ver2) != (4, 2) {
            log::warn!("this code was tested with protocol version 4.2, found protocol version {ver1}.{ver2} instead");
        } else {
            log::debug!("found protocol version {ver1}.{ver2}");
        }

        let name = headset.get_device_name()?;

        log::info!("connected to device {name:?}");

        Ok(headset)
    }

    /// Returns the protocol version used by the headset.
    fn get_protocol_version(&mut self) -> anyhow::Result<(u8, u8)> {
        let response = self
            .features
            .root
            .request(&mut self.device, &[0x11, 0x00, 0x00, 0xaf])?;

        if response[6] != 0xaf {
            log::error!(
                "ping response did not match the request: was {:#04x}",
                response[6]
            );
        }

        Ok((response[4], response[5]))
    }

    /// Returns the device name of the headset.
    fn get_device_name(&mut self) -> anyhow::Result<String> {
        let len = self.features.devname.request(&mut self.device, &[0x01])?[4];

        let mut name = String::new();
        let part_count = ((len - 1) / 16) + 1;

        for i in 0..part_count {
            let rest_len = len as usize - name.len();

            let response = &self
                .features
                .devname
                .request(&mut self.device, &[0x11, i])?[4..4 + std::cmp::min(rest_len, 16)];

            name += std::str::from_utf8(response)?;
        }

        Ok(name)
    }

    /// Sets the button status.
    fn enable_buttons(&mut self, enable: bool) -> anyhow::Result<()> {
        log::debug!("{} buttons", if enable { "enabling" } else { "disabling" });

        let response = self
            .features
            .gkey
            .request(&mut self.device, &[0x21, enable as u8])?;

        if response[4] != enable as u8 {
            log::error!(
                "enable buttons response did not match the request: expected {}, found {}",
                enable as u8,
                response[4]
            );
        }

        Ok(())
    }

    /// Set light configuration.
    pub fn set_lights(&mut self, lights: &lights::Config) -> anyhow::Result<lights::Config> {
        log::debug!("setting lights to {lights:?}");

        let mut request = lights.as_bytes();
        request.insert(0, 0x31);

        self.features
            .lights
            .request(&mut self.device, &request)
            .map(|bytes| lights::Config::from_bytes(&bytes[4..]))
    }

    /// Get battery status and level.
    pub fn get_battery_status(&mut self) -> anyhow::Result<BatteryStatus> {
        self.features
            .battery
            .request(&mut self.device, &[0x01])
            .map(|bytes| BatteryStatus::from_bytes(&bytes[4..]))
    }

    /// Repeatedly queries the device, running config handlers as the respective events occur.
    pub fn run_with_config(&mut self, mut config: Config) {
        if let Err(err) = config.sync_configuration(self) {
            log::error!("failed initial config synchronization: {err}");
        }

        let mut button_state = ButtonState::default();
        let mut power_state;

        const TIMEOUT_IN_MS: i32 = 500;
        const RESET_TIME_IN_SEC: i32 = 20;
        const RESET_COUNTER_AFTER: i32 = RESET_TIME_IN_SEC * 1000 / TIMEOUT_IN_MS;

        let mut counter = 0;

        loop {
            match self.device.next_unrequested_msg(TIMEOUT_IN_MS).as_deref() {
                Some([]) => {
                    // Read timed out, but reset the buttons periodically to survive sleeps
                    counter += 1;
                    if counter > RESET_COUNTER_AFTER {
                        counter = 0;
                        // this is a terrible hack to make it work after reboots, but I cannot be
                        // bothered to figure out a better method to detect the unresponsiveness of
                        // the button handlers right now, so it will have to do
                        //
                        // the correct method probably involved regularly querying whether the
                        // buttons are enabled
                        self.enable_buttons(config.button_handler.is_some()).ok();
                        self.set_lights(&lights::Config {
                            light: lights::Light::Side,
                            effect: *config.side_light_effect,
                            profile_type: lights::ProfileType::Temporary,
                        })
                        .ok();
                        self.set_lights(&lights::Config {
                            light: lights::Light::Logo,
                            effect: *config.logo_light_effect,
                            profile_type: lights::ProfileType::Temporary,
                        })
                        .ok();
                    }
                }
                Some(bytes @ [0x08, 0x10 | 0x20]) => {
                    button_state.mic_arm = MicArm::from_bytes(bytes);
                    log::debug!("mic arm state is {:?}", button_state.mic_arm);

                    config.call_button_handler(self, button_state);
                }
                Some([0x08, 0x01]) => {
                    log::debug!("mute button pressed");

                    config.call_button_handler(
                        self,
                        ButtonState {
                            mute_button: true,
                            ..button_state
                        },
                    );
                }
                Some(bytes @ [0x11, 0xff, feature, 0x00, ..]) if feature == self.features.gkey => {
                    button_state.buttons = Buttons::from_bytes(bytes);
                    log::debug!("button state is {:?}", button_state.buttons);

                    config.call_button_handler(self, button_state);
                }
                Some(bytes @ [0x01, _, 0x00, 0x00, 0x00]) => {
                    button_state.wheel = Wheel::from_bytes(bytes);
                    log::debug!("wheel state is {:?}", button_state.wheel);

                    config.call_button_handler(self, button_state);
                }
                Some([0x11, 0xff, feature, 0x00, rest @ ..])
                    if feature == self.features.battery =>
                {
                    if rest.iter().all(|&b| b == 0x00) {
                        power_state = PowerState::Disconnected;
                    } else {
                        // After the device reconnected, the config needs to be synced again
                        config.set_dirty();
                        power_state = PowerState::Connected;
                    }

                    config.call_power_state_change_handler(self, power_state);
                }
                Some(msg) => log::info!("unhandled message from device: {msg:02x?}"),
                None => (),
            }

            config.call_periodic_handler(self);

            if let Err(err) = config.sync_configuration(self) {
                log::error!("failed config re-synchronization: {err}");
            }
        }
    }
}
