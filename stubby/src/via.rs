//! VIA protocol over raw HID — the same wire protocol the Keychron web launcher
//! uses. Command IDs from Keychron/qmk_firmware `quantum/via.h`.

use hidapi::{HidApi, HidDevice};

pub const VID: u16 = 0x3434; // Keychron
pub const PID: u16 = 0x0340; // V4

const RAW_USAGE_PAGE: u16 = 0xFF60;
const RAW_USAGE: u16 = 0x61;
const REPORT_LEN: usize = 32;

// VIA command IDs
const ID_GET_PROTOCOL_VERSION: u8 = 0x01;
const ID_DYNAMIC_KEYMAP_RESET: u8 = 0x06;
const ID_DYNAMIC_KEYMAP_GET_KEYCODE: u8 = 0x04;
const ID_DYNAMIC_KEYMAP_SET_KEYCODE: u8 = 0x05;
const ID_DYNAMIC_KEYMAP_GET_LAYER_COUNT: u8 = 0x11;

pub struct Via {
    dev: HidDevice,
}

impl Via {
    /// Open the raw-HID (0xFF60) interface of the keyboard.
    pub fn open() -> Result<Via, String> {
        let api = HidApi::new().map_err(|e| format!("hidapi init: {e}"))?;
        let path = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VID
                    && d.product_id() == PID
                    && d.usage_page() == RAW_USAGE_PAGE
                    && d.usage() == RAW_USAGE
            })
            .map(|d| d.path().to_owned())
            .ok_or_else(|| {
                format!("no raw-HID interface for {VID:04x}:{PID:04x} (keyboard unplugged, or udev rule not applied)")
            })?;
        let dev = api
            .open_path(&path)
            .map_err(|e| format!("open {path:?}: {e} (permissions? see README udev rule)"))?;
        Ok(Via { dev })
    }

    fn xfer(&self, cmd: &[u8]) -> Result<[u8; REPORT_LEN], String> {
        let mut out = vec![0u8; 1 + REPORT_LEN]; // leading report-id 0
        out[1..1 + cmd.len()].copy_from_slice(cmd);
        self.dev.write(&out).map_err(|e| format!("write: {e}"))?;
        let mut buf = [0u8; REPORT_LEN];
        match self.dev.read_timeout(&mut buf, 1000) {
            Ok(0) => Err(format!("timeout on cmd 0x{:02x}", cmd[0])),
            Ok(_) => Ok(buf),
            Err(e) => Err(format!("read: {e}")),
        }
    }

    pub fn protocol_version(&self) -> Result<u16, String> {
        let r = self.xfer(&[ID_GET_PROTOCOL_VERSION])?;
        Ok(u16::from_be_bytes([r[1], r[2]]))
    }

    pub fn layer_count(&self) -> Result<u8, String> {
        let r = self.xfer(&[ID_DYNAMIC_KEYMAP_GET_LAYER_COUNT])?;
        Ok(r[1])
    }

    /// Reset ALL dynamic layers to the firmware's default keymap.
    pub fn reset_keymap(&self) -> Result<(), String> {
        self.xfer(&[ID_DYNAMIC_KEYMAP_RESET])?;
        Ok(())
    }

    pub fn get_keycode(&self, layer: u8, row: u8, col: u8) -> Result<u16, String> {
        let r = self.xfer(&[ID_DYNAMIC_KEYMAP_GET_KEYCODE, layer, row, col])?;
        // reply echoes [id, layer, row, col, kc_hi, kc_lo]
        Ok(u16::from_be_bytes([r[4], r[5]]))
    }

    pub fn set_keycode(&self, layer: u8, row: u8, col: u8, kc: u16) -> Result<(), String> {
        let [hi, lo] = kc.to_be_bytes();
        self.xfer(&[ID_DYNAMIC_KEYMAP_SET_KEYCODE, layer, row, col, hi, lo])?;
        Ok(())
    }
}
