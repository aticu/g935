//! Respresents a configuration of the headset.

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use crate::{buttons::ButtonState, lights, Headset, PowerState};

/// A wrapper that simply hides its inner type in `Debug` implementations.
///
/// This is useful for types which do not implement `Debug`.
pub(crate) struct OpaqueDebug<T> {
    /// The wrapped value.
    inner: T,
}

impl<T> fmt::Debug for OpaqueDebug<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "..")
    }
}

impl<T> Deref for OpaqueDebug<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for OpaqueDebug<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<T> for OpaqueDebug<T> {
    fn from(inner: T) -> Self {
        OpaqueDebug { inner }
    }
}

/// A field in the config that tracks whether it was changed.
#[derive(Debug)]
pub(crate) struct ConfigField<T> {
    /// The value of the config field.
    val: T,
    /// Whether the config field was changed.
    dirty: bool,
}

impl<T: Default> Default for ConfigField<T> {
    fn default() -> Self {
        Self {
            val: T::default(),
            // Start dirty for the initial synchronization.
            dirty: true,
        }
    }
}

impl<T> ConfigField<T> {
    /// Sets the value of the config field.
    fn set(&mut self, val: T) {
        self.val = val;
        self.force_sync();
    }

    /// Sets the dirty flag to force synchronization.
    fn force_sync(&mut self) {
        self.dirty = true;
    }

    /// Checks if the config field needs to be synchronized and clears the dirty flag.
    fn needs_sync(&mut self) -> bool {
        let old_dirty = self.dirty;
        self.dirty = false;
        old_dirty
    }
}

impl<T> Deref for ConfigField<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<T> DerefMut for ConfigField<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

/// The type of a handler for button presses.
pub type ButtonHandler = Box<dyn FnMut(&mut Config, &mut Headset, ButtonState)>;

/// A handler for changes in the power state of the headset.
pub type PowerStateChangeHandler = Box<dyn FnMut(&mut Config, &mut Headset, PowerState)>;

/// The type of a handler for periodic updates.
pub type PeriodicHandler = Box<dyn FnMut(&mut Config, &mut Headset)>;

/// The configuration for running the software.
#[derive(Debug, Default)]
pub struct Config {
    /// The handler for the buttons press.
    pub(crate) button_handler: ConfigField<Option<OpaqueDebug<ButtonHandler>>>,
    /// The handler for the power state change.
    pub(crate) power_state_change_handler:
        ConfigField<Option<OpaqueDebug<PowerStateChangeHandler>>>,
    /// The handler for periodic callbacks.
    pub(crate) periodic_handler: ConfigField<Option<OpaqueDebug<PeriodicHandler>>>,
    /// The light effect to use for the side lights.
    pub(crate) side_light_effect: ConfigField<lights::Effect>,
    /// The light effect to use for the logo lights.
    pub(crate) logo_light_effect: ConfigField<lights::Effect>,
}

impl Config {
    /// Syncs the current configuration with
    pub(crate) fn sync_configuration(&mut self, headset: &mut Headset) -> anyhow::Result<()> {
        if self.button_handler.needs_sync() {
            headset.enable_buttons(self.button_handler.is_some())?;
        }

        if self.power_state_change_handler.needs_sync() {}

        if self.side_light_effect.needs_sync() {
            headset.set_lights(&lights::Config {
                light: lights::Light::Side,
                effect: *self.side_light_effect,
                profile_type: lights::ProfileType::Temporary,
            })?;
        }

        if self.logo_light_effect.needs_sync() {
            headset.set_lights(&lights::Config {
                light: lights::Light::Logo,
                effect: *self.logo_light_effect,
                profile_type: lights::ProfileType::Temporary,
            })?;
        }

        Ok(())
    }

    /// Explicitly sets the configuration to dirty to enable a re-synchronization.
    ///
    /// This is for example useful after a device restart.
    pub(crate) fn set_dirty(&mut self) {
        self.button_handler.force_sync();
        self.power_state_change_handler.force_sync();
        self.periodic_handler.force_sync();
        self.side_light_effect.force_sync();
        self.logo_light_effect.force_sync();
    }

    /// Calls the configured button handler, if it exists.
    pub(crate) fn call_button_handler(&mut self, headset: &mut Headset, button_state: ButtonState) {
        if let Some(mut button_handler) = self.button_handler.take() {
            // Clear the dirty flag in case it was set to check for changes to the handler itself
            self.button_handler.dirty = false;

            button_handler(self, headset, button_state);

            if !self.button_handler.dirty {
                *self.button_handler = Some(button_handler);
            }
        }
    }

    /// Sets the handler for button presses.
    pub fn set_button_handler(&mut self, handler: Option<ButtonHandler>) {
        self.button_handler
            .set(handler.map(|handler| OpaqueDebug { inner: handler }));
    }

    /// Calls the configured power state change handler, if it exists.
    pub(crate) fn call_power_state_change_handler(
        &mut self,
        headset: &mut Headset,
        power_state: PowerState,
    ) {
        if let Some(mut power_state_change_handler) = self.power_state_change_handler.take() {
            // Clear the dirty flag in case it was set to check for changes to the handler itself
            self.power_state_change_handler.dirty = false;

            power_state_change_handler(self, headset, power_state);

            if !self.power_state_change_handler.dirty {
                *self.power_state_change_handler = Some(power_state_change_handler);
            }
        }
    }

    /// Sets the handler for power state changes.
    pub fn set_power_state_change_handler(&mut self, handler: Option<PowerStateChangeHandler>) {
        self.power_state_change_handler
            .set(handler.map(|handler| OpaqueDebug { inner: handler }));
    }

    /// Calls the configured periodic handler, if it exists.
    pub(crate) fn call_periodic_handler(&mut self, headset: &mut Headset) {
        if let Some(mut periodic_handler) = self.periodic_handler.take() {
            // Clear the dirty flag in case it was set to check for changes to the handler itself
            self.periodic_handler.dirty = false;

            periodic_handler(self, headset);

            if !self.periodic_handler.dirty {
                *self.periodic_handler = Some(periodic_handler);
            }
        }
    }

    /// Sets the handler for periodic updates.
    pub fn set_periodic_handler(&mut self, handler: Option<PeriodicHandler>) {
        self.periodic_handler
            .set(handler.map(|handler| OpaqueDebug { inner: handler }));
    }

    /// Sets the effect for the side light.
    pub fn set_side_light_effect(&mut self, effect: lights::Effect) {
        self.side_light_effect.set(effect);
    }

    /// Sets the effect for the logo light.
    pub fn set_logo_light_effect(&mut self, effect: lights::Effect) {
        self.logo_light_effect.set(effect);
    }
}
