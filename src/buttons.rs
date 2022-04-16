//! Code for interacting with buttons.

use crate::FromBytes;

/// A complete map of the state of all buttons.
#[derive(Debug, Default, Clone, Copy)]
pub struct ButtonState {
    /// The state of the buttons.
    pub(crate) buttons: Buttons,
    /// The state of the wheel.
    pub(crate) wheel: Wheel,
    /// The state of the microphone arm.
    pub(crate) mic_arm: MicArm,
    /// Whether the mute button was pressed during the recording of this state.
    pub(crate) mute_button: bool,
}

impl ButtonState {
    /// Returns `true` if the microphone was flipped up.
    pub fn mic_flipped_up(&self, old: &ButtonState) -> bool {
        old.mic_arm == MicArm::Down && self.mic_arm == MicArm::Up
    }

    /// Returns `true` if the microphone was flipped down.
    pub fn mic_flipped_down(&self, old: &ButtonState) -> bool {
        old.mic_arm == MicArm::Up && self.mic_arm == MicArm::Down
    }

    /// Returns `true` if the G1 key was pressed.
    pub fn g1_pressed(&self, old: &ButtonState) -> bool {
        !old.buttons.g1 && self.buttons.g1
    }

    /// Returns `true` if the G1 key was released.
    pub fn g1_released(&self, old: &ButtonState) -> bool {
        old.buttons.g1 && !self.buttons.g1
    }

    /// Returns `true` if the G2 key was pressed.
    pub fn g2_pressed(&self, old: &ButtonState) -> bool {
        !old.buttons.g2 && self.buttons.g2
    }

    /// Returns `true` if the G2 key was released.
    pub fn g2_released(&self, old: &ButtonState) -> bool {
        old.buttons.g2 && !self.buttons.g2
    }

    /// Returns `true` if the G3 key was pressed.
    pub fn g3_pressed(&self, old: &ButtonState) -> bool {
        !old.buttons.g3 && self.buttons.g3
    }

    /// Returns `true` if the G3 key was released.
    pub fn g3_released(&self, old: &ButtonState) -> bool {
        old.buttons.g3 && !self.buttons.g3
    }

    /// Returns `true` if the scroll wheel is being scrolled down.
    pub fn scroll_down(&self) -> bool {
        self.wheel.down
    }

    /// Returns `true` if the scroll wheel is being scrolled up.
    pub fn scroll_up(&self) -> bool {
        self.wheel.up
    }

    /// Returns `true` if the mute button is being pressed.
    pub fn mute_button_pressed(&self) -> bool {
        self.mute_button
    }

    /// Returns `true` if the scrolling ended.
    pub fn scroll_end(&self, old: &ButtonState) -> bool {
        (old.scroll_down() || old.scroll_up()) && !self.scroll_down() && !self.scroll_up()
    }
}

/// Contains a bool for each button, to show if it is pressed
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Buttons {
    /// If g1 button is pressed
    pub(crate) g1: bool,
    /// If g2 button is pressed
    pub(crate) g2: bool,
    /// If g3 button is pressed
    pub(crate) g3: bool,
}

impl FromBytes for Buttons {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            g1: bytes[4] & 1 != 0,
            g2: bytes[4] & 2 != 0,
            g3: bytes[4] & 4 != 0,
        }
    }
}

/// Contains a bool for each direction of the wheel, to show if it is active
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Wheel {
    /// If the wheel is currently scrolling up
    pub(crate) up: bool,
    /// If the wheel is currently scrolling down
    pub(crate) down: bool,
}

impl FromBytes for Wheel {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            up: bytes[1] & 1 != 0,
            down: bytes[1] & 2 != 0,
        }
    }
}

/// The state of the microphone arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicArm {
    /// The microphone arm is currently flipped up.
    Up,
    /// The microphone arm is currently flipped down.
    Down,
}

impl FromBytes for MicArm {
    fn from_bytes(bytes: &[u8]) -> Self {
        match bytes[1] {
            0x10 => Self::Up,
            0x20 => Self::Down,
            _ => {
                log::error!("unexpected microphone arm state, defaulting to UP");

                Self::Up
            }
        }
    }
}

impl Default for MicArm {
    fn default() -> Self {
        MicArm::Up
    }
}
