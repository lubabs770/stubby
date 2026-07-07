//! stubby-probe — milestone 1: prove we can talk to the Keychron V4 over raw HID
//! using the VIA protocol (the same wire protocol the web launcher uses).
//!
//! No GUI, no browser, no Electron. Just: open the 0xFF60 raw-HID interface and
//! ask the firmware a few questions. If this round-trips, the whole native
//! launcher is buildable on top of it.

use hidapi::HidApi;

// Keychron V4
const VID: u16 = 0x3434;
const PID: u16 = 0x0340;

// QMK raw-HID interface identifiers (from qmk/tmk usb_descriptor)
const RAW_USAGE_PAGE: u16 = 0xFF60;
const RAW_USAGE: u16 = 0x61;

// VIA command IDs (from Keychron/qmk_firmware quantum/via.h)
const ID_GET_PROTOCOL_VERSION: u8 = 0x01;
const ID_GET_KEYBOARD_VALUE: u8 = 0x02;
const ID_DYNAMIC_KEYMAP_GET_LAYER_COUNT: u8 = 0x11;

// sub-commands for ID_GET_KEYBOARD_VALUE
const ID_UPTIME: u8 = 0x01;
const ID_FIRMWARE_VERSION: u8 = 0x04;

const REPORT_LEN: usize = 32;

fn main() {
    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hidapi init failed: {e}");
            std::process::exit(1);
        }
    };

    // 1) Enumerate: show every interface the V4 exposes and flag the raw-HID one.
    println!("== enumerating {VID:04x}:{PID:04x} ==");
    let mut raw_path = None;
    for d in api.device_list() {
        if d.vendor_id() == VID && d.product_id() == PID {
            let is_raw = d.usage_page() == RAW_USAGE_PAGE && d.usage() == RAW_USAGE;
            println!(
                "  iface#{:<2} usage_page=0x{:04x} usage=0x{:02x} {} path={:?}",
                d.interface_number(),
                d.usage_page(),
                d.usage(),
                if is_raw { "<-- RAW HID (VIA)" } else { "" },
                d.path()
            );
            if is_raw {
                raw_path = Some(d.path().to_owned());
            }
        }
    }

    let path = match raw_path {
        Some(p) => p,
        None => {
            eprintln!(
                "\nno 0xFF60 raw-HID interface found.\n\
                 - is the keyboard plugged in?\n\
                 - permission denied? install the udev rule (see ../README.md) and replug."
            );
            std::process::exit(2);
        }
    };

    let dev = match api.open_path(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("\nopen failed: {e}\n(almost certainly a permissions issue — install the udev rule)");
            std::process::exit(3);
        }
    };

    println!("\n== talking VIA ==");

    // 2) protocol version
    let r = xfer(&dev, &[ID_GET_PROTOCOL_VERSION]);
    let proto = u16::from_be_bytes([r[1], r[2]]);
    println!("  protocol version : {proto}");

    // 3) firmware version (u32 BE at bytes 2..6)
    let r = xfer(&dev, &[ID_GET_KEYBOARD_VALUE, ID_FIRMWARE_VERSION]);
    let fw = u32::from_be_bytes([r[2], r[3], r[4], r[5]]);
    println!("  firmware version : {fw} (0x{fw:08x})");

    // 4) uptime (ms, u32 BE at bytes 2..6)
    let r = xfer(&dev, &[ID_GET_KEYBOARD_VALUE, ID_UPTIME]);
    let uptime = u32::from_be_bytes([r[2], r[3], r[4], r[5]]);
    println!("  uptime           : {:.1}s", uptime as f64 / 1000.0);

    // 5) layer count
    let r = xfer(&dev, &[ID_DYNAMIC_KEYMAP_GET_LAYER_COUNT]);
    println!("  dynamic layers   : {}", r[1]);

    println!("\nOK — transport works. Next: read/write the keymap buffer, then the GUI.");
}

/// Send a VIA command and return the 32-byte reply.
/// hidapi wants a leading report-ID byte (0x00 for unnumbered reports), so the
/// write buffer is 1 + 32 bytes; the reply is 32 bytes.
fn xfer(dev: &hidapi::HidDevice, cmd: &[u8]) -> [u8; REPORT_LEN] {
    let mut out = vec![0u8; 1 + REPORT_LEN]; // [report_id=0, payload...]
    out[1..1 + cmd.len()].copy_from_slice(cmd);
    if let Err(e) = dev.write(&out) {
        eprintln!("write failed: {e}");
        std::process::exit(4);
    }
    let mut buf = [0u8; REPORT_LEN];
    match dev.read_timeout(&mut buf, 1000) {
        Ok(0) => {
            eprintln!("read timed out (no reply to cmd 0x{:02x})", cmd[0]);
            std::process::exit(5);
        }
        Ok(_) => buf,
        Err(e) => {
            eprintln!("read failed: {e}");
            std::process::exit(6);
        }
    }
}
