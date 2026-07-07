//! QMK/HID keycode <-> label. Covers the basic HID range (0x00–0xE7) that the
//! standard VIA keymap uses. Unknown codes (layer taps, macros, RGB, etc.) fall
//! back to hex — set/get still work by raw value, we just don't name them yet.

/// Human label for a keycode.
pub fn name_for(kc: u16) -> String {
    match kc {
        0x0000 => "".into(),   // KC_NO
        0x0001 => "▽".into(),  // KC_TRANSPARENT
        _ => {
            if let Some(s) = basic(kc as u8).filter(|_| kc <= 0xFF) {
                s.into()
            } else {
                format!("0x{kc:04X}")
            }
        }
    }
}

/// A curated palette for the assignment panel: (group, label, keycode).
pub fn palette() -> Vec<(&'static str, &'static str, u16)> {
    let mut v = Vec::new();
    for kc in 0x04u16..=0x1D {
        v.push(("Letters", leak(basic(kc as u8).unwrap()), kc));
    }
    for kc in 0x1Eu16..=0x27 {
        v.push(("Digits", leak(basic(kc as u8).unwrap()), kc));
    }
    for (kc, _) in (0x2Du16..=0x38).map(|k| (k, ())) {
        if let Some(n) = basic(kc as u8) {
            v.push(("Symbols", leak(n), kc));
        }
    }
    for kc in 0x3Au16..=0x45 {
        v.push(("Function", leak(basic(kc as u8).unwrap()), kc));
    }
    for kc in [0x28u16, 0x29, 0x2A, 0x2B, 0x2C, 0x39] {
        v.push(("Edit", leak(basic(kc as u8).unwrap()), kc));
    }
    for kc in 0x49u16..=0x52 {
        if let Some(n) = basic(kc as u8) {
            v.push(("Nav", leak(n), kc));
        }
    }
    for kc in 0xE0u16..=0xE7 {
        v.push(("Modifiers", leak(basic(kc as u8).unwrap()), kc));
    }
    v.push(("Special", "TRNS ▽", 0x0001));
    v.push(("Special", "NO ∅", 0x0000));
    v
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn basic(kc: u8) -> Option<&'static str> {
    const LETTERS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    Some(match kc {
        0x04..=0x1D => return Some(letter(LETTERS[(kc - 0x04) as usize])),
        0x1E => "1",
        0x1F => "2",
        0x20 => "3",
        0x21 => "4",
        0x22 => "5",
        0x23 => "6",
        0x24 => "7",
        0x25 => "8",
        0x26 => "9",
        0x27 => "0",
        0x28 => "Enter",
        0x29 => "Esc",
        0x2A => "Bksp",
        0x2B => "Tab",
        0x2C => "Space",
        0x2D => "-",
        0x2E => "=",
        0x2F => "[",
        0x30 => "]",
        0x31 => "\\",
        0x33 => ";",
        0x34 => "'",
        0x35 => "`",
        0x36 => ",",
        0x37 => ".",
        0x38 => "/",
        0x39 => "Caps",
        0x3A => "F1",
        0x3B => "F2",
        0x3C => "F3",
        0x3D => "F4",
        0x3E => "F5",
        0x3F => "F6",
        0x40 => "F7",
        0x41 => "F8",
        0x42 => "F9",
        0x43 => "F10",
        0x44 => "F11",
        0x45 => "F12",
        0x46 => "PrtSc",
        0x47 => "ScrLk",
        0x48 => "Pause",
        0x49 => "Ins",
        0x4A => "Home",
        0x4B => "PgUp",
        0x4C => "Del",
        0x4D => "End",
        0x4E => "PgDn",
        0x4F => "→",
        0x50 => "←",
        0x51 => "↓",
        0x52 => "↑",
        0xE0 => "LCtrl",
        0xE1 => "LShft",
        0xE2 => "LAlt",
        0xE3 => "LGui",
        0xE4 => "RCtrl",
        0xE5 => "RShft",
        0xE6 => "RAlt",
        0xE7 => "RGui",
        _ => return None,
    })
}

fn letter(b: u8) -> &'static str {
    // static single-char strs for A-Z
    const S: &[&str; 26] = &[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    S[(b - b'A') as usize]
}
