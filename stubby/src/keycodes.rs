//! QMK/HID keycode decoding + the tabbed assignment palette (Keychron-style).
//!
//! Sources:
//! - basic HID 0x00–0xFF (letters, digits, symbols, F-keys, nav, mods, media)
//! - quantum layer range 0x4000–0x52FF (MO/TO/TG/TT/DF/OSL/LT)
//! - QK_KB 0x7E00+ → the definition's `customKeycodes` names (Keychron macros)
//! Anything else falls back to hex — set/get still work by raw value.

pub const KC_NO: u16 = 0x0000;
pub const KC_TRNS: u16 = 0x0001;

/// Display label for a keycode. `custom` = definition customKeycodes names.
pub fn name_for(kc: u16, custom: &[String]) -> String {
    match kc {
        0x0000 => String::new(),
        0x0001 => "▽".into(),
        0x0002..=0x00FF => basic(kc as u8)
            .map(str::to_string)
            .unwrap_or_else(|| format!("0x{kc:04X}")),
        0x7E00..=0x7E3F => {
            let n = (kc - 0x7E00) as usize;
            custom.get(n).cloned().unwrap_or_else(|| format!("KB{n}"))
        }
        _ => quantum(kc).unwrap_or_else(|| format!("0x{kc:04X}")),
    }
}

/// Quantum layer/mod keycodes → labels.
fn quantum(kc: u16) -> Option<String> {
    Some(match kc {
        0x5200..=0x521F => format!("TO({})", kc - 0x5200),
        0x5220..=0x523F => format!("MO({})", kc - 0x5220),
        0x5240..=0x525F => format!("DF({})", kc - 0x5240),
        0x5260..=0x527F => format!("TG({})", kc - 0x5260),
        0x5280..=0x529F => format!("OSL({})", kc - 0x5280),
        0x52C0..=0x52DF => format!("TT({})", kc - 0x52C0),
        0x4000..=0x4FFF => {
            let layer = (kc >> 8) & 0x0F;
            let inner = basic((kc & 0xFF) as u8).unwrap_or("?");
            format!("LT{layer}\n{inner}")
        }
        _ => return None,
    })
}

// ---- assignment palette (tabbed) --------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Basic,
    Media,
    Special,
    Layer,
    Custom,
}

pub const TABS: &[(Tab, &str)] = &[
    (Tab::Basic, "Basic"),
    (Tab::Media, "Media"),
    (Tab::Special, "Special"),
    (Tab::Layer, "Layer"),
    (Tab::Custom, "Custom"),
];

/// Entries `(label, keycode)` shown in a tab.
pub fn entries(tab: Tab, layers: u8, custom: &[String]) -> Vec<(String, u16)> {
    let simple = |kcs: &[u16]| -> Vec<(String, u16)> {
        kcs.iter().map(|&k| (name_for(k, custom), k)).collect()
    };
    match tab {
        Tab::Basic => {
            let mut v: Vec<u16> = Vec::new();
            v.extend(0x04u16..=0x1D); // A-Z
            v.extend(0x1Eu16..=0x27); // 1-0
            v.extend([0x29, 0x2A, 0x2B, 0x2C, 0x28, 0x39]); // esc bksp tab space enter caps
            v.extend(0x2Du16..=0x38); // symbols
            v.extend(0x3Au16..=0x45); // F1-F12
            v.extend(0x49u16..=0x52); // ins/home/pgup/del/end/pgdn/arrows
            v.extend(0xE0u16..=0xE7); // modifiers
            simple(&v)
        }
        Tab::Media => simple(&[
            0xA8, 0xA9, 0xAA, 0xAE, 0xAB, 0xAC, 0xAD, 0xB2, 0xB3, 0xB1, 0xA5, 0xA6, 0xA7,
        ]),
        Tab::Special => vec![("NO ∅".into(), KC_NO), ("TRNS ▽".into(), KC_TRNS)],
        Tab::Layer => {
            let mut v = Vec::new();
            for l in 0..layers as u16 {
                v.push((format!("MO({l})"), 0x5220 + l));
                v.push((format!("TO({l})"), 0x5200 + l));
                v.push((format!("TG({l})"), 0x5260 + l));
                v.push((format!("TT({l})"), 0x52C0 + l));
                v.push((format!("DF({l})"), 0x5240 + l));
                v.push((format!("OSL({l})"), 0x5280 + l));
            }
            v
        }
        Tab::Custom => custom
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), 0x7E00 + i as u16))
            .collect(),
    }
}

fn basic(kc: u8) -> Option<&'static str> {
    const LETTERS: &[&str; 26] = &[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    Some(match kc {
        0x04..=0x1D => LETTERS[(kc - 0x04) as usize],
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
        // media / consumer
        0xA5 => "Power",
        0xA6 => "Sleep",
        0xA7 => "Wake",
        0xA8 => "Mute",
        0xA9 => "Vol+",
        0xAA => "Vol-",
        0xAB => "Next",
        0xAC => "Prev",
        0xAD => "Stop",
        0xAE => "Play",
        0xB1 => "Mail",
        0xB2 => "Calc",
        0xB3 => "PC",
        // modifiers
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
