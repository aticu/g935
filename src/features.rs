//! Deals with features.

use crate::device::Device;

/// Represents a feature on the device.
#[derive(Debug)]
pub(crate) struct Feature {
    /// The index of the feature.
    index: u8,
}

impl Feature {
    /// Makes a request on the feature.
    pub(crate) fn request(&self, device: &mut Device, body: &[u8]) -> anyhow::Result<Vec<u8>> {
        assert!(
            body.len() <= 17,
            "feature request can be at most 17 bytes large"
        );

        let mut data = [0; 20];
        data[0] = 0x11;
        data[1] = 0xff;
        data[2] = self.index;

        data[3..body.len() + 3].copy_from_slice(body);

        let response = device.request(&data)?;

        Ok(response.to_vec())
    }
}

impl PartialEq<u8> for Feature {
    fn eq(&self, other: &u8) -> bool {
        self.index == *other
    }
}

impl PartialEq<&u8> for Feature {
    fn eq(&self, other: &&u8) -> bool {
        self.index == **other
    }
}

impl PartialEq<Feature> for u8 {
    fn eq(&self, other: &Feature) -> bool {
        *self == other.index
    }
}

impl PartialEq<Feature> for &u8 {
    fn eq(&self, other: &Feature) -> bool {
        **self == other.index
    }
}

/// Resolves the feature with the given ID.
fn resolve_feature(
    root_feature: &Feature,
    device: &mut Device,
    feature: u16,
) -> anyhow::Result<Feature> {
    let feat_bytes = feature.to_be_bytes();

    let response = root_feature.request(device, &[0x01, feat_bytes[0], feat_bytes[1]])?;

    Ok(Feature { index: response[4] })
}

macro_rules! feature_map {
    ($(#[$meta:meta])* $vis:vis struct $name:ident { $($(#[$feature_meta:meta])* $feature:ident: $num:expr),* $(,)? }) => {
        $(#[$meta])*
        $vis struct $name {
            $(
                $(#[$feature_meta])*
                $vis $feature: Feature,
            )*
        }

        impl $name {
            /// Initializes the feature map from the given `Device`.
            $vis fn initialize(device: &mut Device) -> anyhow::Result<Self> {
                let root = Feature { index: 0 };

                Ok(Self {
                    $(
                        $feature: resolve_feature(&root, device, $num)?,
                    )*
                })
            }
        }
    };
}

feature_map! {
    /// A map of the feature indices of the specific device.
    #[derive(Debug)]
    pub(crate) struct FeatureMap {
        /// The root feature used for discovering other features.
        root: 0x0000,
        /// The feature used to read battery levels and charging status.
        battery: 0x1f20,
        // /// The feature used for information about the device and firmware.
        // devinfo: 0x0002,
        /// The feature used to read the device name.
        devname: 0x0005,
        /// The feature that allows access to the GKey buttons.
        gkey: 0x8010,
        /// The feature that controls the LEDs.
        lights: 0x8070,
        // /// The feature that controls side tones.
        // sidetone: 0x8300,
        // /// The feature that controls the equalizer.
        // eq: 0x8310,
    }
}
