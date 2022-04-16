//! A wrapper around the device read/write interface.

use std::{collections::VecDeque, fmt};

use hidapi::HidDevice;

/// Implements the communication with the hardware.
pub(crate) struct Device {
    /// The raw inner `HidDevice` of this device.
    device: HidDevice,
    /// The buffer for unhandled messages.
    msg_buffer: VecDeque<Vec<u8>>,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Device").finish_non_exhaustive()
    }
}

impl Device {
    /// Creates a new device from the underlying device.
    pub(crate) fn new(device: hidapi::HidDevice) -> Self {
        Self {
            device,
            msg_buffer: VecDeque::new(),
        }
    }

    /// Writes the given `data` to the device.
    fn write(&mut self, data: &[u8]) -> anyhow::Result<usize> {
        log::trace!("writing {:02x?}", data);

        Ok(self.device.write(data)?)
    }

    /// Reads from the device into the given buffer, returning a slice to the read data.
    fn read(&mut self, timeout: i32) -> anyhow::Result<Vec<u8>> {
        let mut buf = [0; 1024];

        let len = self.device.read_timeout(&mut buf, timeout)?;
        let result = buf[0..len].to_vec();

        if len != 0 {
            log::trace!("read {:02x?}", result);
        }

        Ok(result)
    }

    /// Sends a request to the device, returning the reply.
    pub(crate) fn request(&mut self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.write(data)?;

        let start = std::time::Instant::now();

        loop {
            let result = self.read(500)?;

            if result.len() < 4 || result[0..4] != data[0..4] {
                log::debug!("buffering unrequested message for later");

                self.msg_buffer.push_back(result.to_vec());
            } else if result.len() == 0 {
                return Err(anyhow::anyhow!("request timed out"));
            } else {
                return Ok(result);
            }

            if start.elapsed() > std::time::Duration::from_secs(2) {
                return Err(anyhow::anyhow!("request timed out"));
            }
        }
    }

    /// Returns the next unrequested message sent by the device if there is one.
    pub(crate) fn next_unrequested_msg(&mut self, timeout: i32) -> Option<Vec<u8>> {
        if let Some(msg) = self.msg_buffer.pop_front() {
            log::debug!(
                "returning an unrequested message from the buffer instead of reading it fresh"
            );

            return Some(msg);
        }

        self.read(timeout).ok().map(|slice| slice.to_vec())
    }
}
