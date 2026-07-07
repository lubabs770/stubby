//! stubby — a native Keychron launcher core.
//!
//! `via`      — the raw-HID VIA transport (open device, get/set keycodes).
//! `kle`      — parse a VIA definition's KLE layout into positioned keys.
//! `keycodes` — map QMK/HID keycodes to human labels + an assignment palette.

pub mod keycodes;
pub mod kle;
pub mod via;
