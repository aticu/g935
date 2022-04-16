//! Code for interacting with the power state of the device.

/// Represents the current power state of the headset.
#[derive(Debug, Clone, Copy)]
pub enum PowerState {
    /// The headset is currently connected.
    Connected,
    /// The headset is turned off.
    Disconnected,
}
