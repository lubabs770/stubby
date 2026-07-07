//! stubby — Slint (native, no Electron) Keychron launcher.
//!
//! Material-styled chrome + custom keycap components rendering the live keymap.
//! Board render, click-to-select, layer switching, reload, and the bottom tabbed
//! keycode picker with live assignment. Reuses via/kle/keycodes unchanged.

slint::include_modules!();

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};
use stubby::{
    keycodes::{self, Tab},
    kle,
    via::Via,
};

const DEF_JSON: &str = include_str!("v4_ansi.json");
const U: f32 = 62.0; // key unit (px)
const GAP: f32 = 6.0;
const COLS: usize = 16; // picker grid columns

/// Accent presets (seed colours). Values are 0xRRGGBB.
struct Accent {
    name: &'static str,
    rgb: u32,
}

const ACCENTS: &[Accent] = &[
    Accent { name: "Slint",  rgb: 0x2379f4 },
    Accent { name: "Purple", rgb: 0x8b5cf0 },
    Accent { name: "Red",    rgb: 0xe04848 },
    Accent { name: "Green",  rgb: 0x2fae55 },
    Accent { name: "Orange", rgb: 0xe8801f },
    Accent { name: "Teal",   rgb: 0x1aa89a },
    Accent { name: "Pink",   rgb: 0xe0559a },
];

fn col(v: u32) -> slint::Color {
    slint::Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn rgb_to_hsl(v: u32) -> (f64, f64, f64) {
    let r = ((v >> 16) & 0xff) as f64 / 255.0;
    let g = ((v >> 8) & 0xff) as f64 / 255.0;
    let b = (v & 0xff) as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < 1e-9 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    ((h + 360.0) % 360.0, s, l)
}

fn tone(h: f64, s: f64, l: f64) -> slint::Color {
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    slint::Color::from_rgb_u8(
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

/// Derive a Material-You-ish tonal palette from a seed accent + light/dark.
fn apply_palette(app: &AppWindow, dark: bool, accent: u32) {
    let (h, s0, _) = rgb_to_hsl(accent);
    let ss = (s0 * 0.5).clamp(0.05, 0.35); // muted surface tint
    app.set_accent(col(accent));
    app.set_t_ok(col(0x4cc46a));
    app.set_dark(dark);
    if dark {
        app.set_t_bg(tone(h, ss * 0.5, 0.075));
        app.set_t_panel(tone(h, ss * 0.5, 0.115));
        app.set_t_cap(tone(h, ss * 0.6, 0.185));
        app.set_t_cap_hover(tone(h, ss * 0.6, 0.26));
        app.set_t_cap_text(tone(h, ss * 0.3, 0.90));
        app.set_t_text(tone(h, ss * 0.25, 0.94));
        app.set_t_dim(tone(h, ss * 0.3, 0.60));
    } else {
        app.set_t_bg(tone(h, ss * 0.5, 0.88));
        app.set_t_panel(tone(h, ss * 0.4, 0.95));
        app.set_t_cap(tone(h, ss * 0.3, 0.985));
        app.set_t_cap_hover(tone(h, ss * 0.5, 0.91));
        app.set_t_cap_text(tone(h, ss * 0.5, 0.20));
        app.set_t_text(tone(h, ss * 0.5, 0.13));
        app.set_t_dim(tone(h, ss * 0.4, 0.45));
    }
}

struct State {
    via: Option<Via>,
    keys: Vec<kle::Key>,
    custom: Vec<String>,
    keymap: HashMap<(u8, u8), u16>,
    layer: u8,
    tab: Tab,
    selected: Option<usize>,
}

impl State {
    fn read_layer(&mut self) {
        self.keymap.clear();
        let Some(via) = &self.via else { return };
        for k in &self.keys {
            if let Ok(kc) = via.get_keycode(self.layer, k.row, k.col) {
                self.keymap.insert((k.row, k.col), kc);
            }
        }
    }

    fn rows(&self) -> Vec<KeyData> {
        self.keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let kc = self.keymap.get(&(k.row, k.col)).copied().unwrap_or(0);
                KeyData {
                    kx: k.x * U,
                    ky: k.y * U,
                    kw: k.w * U - GAP,
                    kh: k.h * U - GAP,
                    label: keycodes::name_for(kc, &self.custom).into(),
                    selected: self.selected == Some(i),
                    index: i as i32,
                }
            })
            .collect()
    }

    fn sel_label(&self) -> String {
        match self.selected {
            Some(i) => {
                let k = &self.keys[i];
                let kc = self.keymap.get(&(k.row, k.col)).copied().unwrap_or(0);
                format!("selected [{},{}] = {}", k.row, k.col, keycodes::name_for(kc, &self.custom))
            }
            None => "click a key to select".into(),
        }
    }

    /// Picker entries for the current tab, chunked into rows of COLS.
    fn picker_rows(&self, layer_count: u8) -> ModelRc<ModelRc<PickData>> {
        let entries = keycodes::entries(self.tab, layer_count, &self.custom);
        let rows: Vec<ModelRc<PickData>> = entries
            .chunks(COLS)
            .map(|chunk| {
                let cells: Vec<PickData> = chunk
                    .iter()
                    .map(|(label, kc)| PickData {
                        label: label.as_str().into(),
                        kc: *kc as i32,
                    })
                    .collect();
                ModelRc::new(VecModel::from(cells))
            })
            .collect();
        ModelRc::new(VecModel::from(rows))
    }
}

fn tab_from_index(i: i32) -> Tab {
    keycodes::TABS
        .get(i as usize)
        .map(|(t, _)| *t)
        .unwrap_or(Tab::Basic)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def: serde_json::Value = serde_json::from_str(DEF_JSON)?;
    let keys = kle::parse(&def["layouts"]["keymap"]);
    let kb_name = def["name"].as_str().unwrap_or("Keyboard").to_string();
    let custom: Vec<String> = def["customKeycodes"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|k| k["name"].as_str().unwrap_or("?").to_string())
                .collect()
        })
        .unwrap_or_default();

    let (via, layer_count, status) = match Via::open() {
        Ok(v) => {
            let lc = v.layer_count().unwrap_or(1);
            let p = v.protocol_version().unwrap_or(0);
            (Some(v), lc, format!("VIA proto {p}"))
        }
        Err(e) => (None, 1, format!("OFFLINE — {e}")),
    };

    let max_x = keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
    let max_y = keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);

    let mut state = State {
        via,
        keys,
        custom,
        keymap: HashMap::new(),
        layer: 0,
        tab: Tab::Basic,
        selected: None,
    };
    state.read_layer();
    let connected = state.via.is_some();
    let state = Rc::new(RefCell::new(state));

    let app = AppWindow::new()?;
    app.set_kb_name(kb_name.into());
    app.set_connected(connected);
    app.set_layer_count(layer_count as i32);
    app.set_status(status.into());
    app.set_board_w(max_x * U + GAP);
    app.set_board_h(max_y * U + GAP);
    app.set_sel_label(state.borrow().sel_label().into());

    let tab_names: Vec<SharedString> = keycodes::TABS.iter().map(|(_, n)| (*n).into()).collect();
    app.set_tab_names(ModelRc::new(VecModel::from(tab_names)));
    app.set_picker_rows(state.borrow().picker_rows(layer_count));

    let accent_names: Vec<SharedString> = ACCENTS.iter().map(|a| a.name.into()).collect();
    app.set_accent_names(ModelRc::new(VecModel::from(accent_names)));

    let ui_dark = Rc::new(Cell::new(true));
    let ui_accent = Rc::new(Cell::new(0usize));
    apply_palette(&app, ui_dark.get(), ACCENTS[ui_accent.get()].rgb);
    {
        let weak = app.as_weak();
        let ui_dark = ui_dark.clone();
        let ui_accent = ui_accent.clone();
        app.on_accent_selected(move |i| {
            let i = i.max(0) as usize % ACCENTS.len();
            ui_accent.set(i);
            if let Some(a) = weak.upgrade() {
                apply_palette(&a, ui_dark.get(), ACCENTS[i].rgb);
            }
        });
    }
    {
        let weak = app.as_weak();
        let ui_dark = ui_dark.clone();
        let ui_accent = ui_accent.clone();
        app.on_toggle_dark(move || {
            ui_dark.set(!ui_dark.get());
            if let Some(a) = weak.upgrade() {
                apply_palette(&a, ui_dark.get(), ACCENTS[ui_accent.get()].rgb);
            }
        });
    }

    let board = Rc::new(VecModel::from(state.borrow().rows()));
    app.set_keys(ModelRc::from(board.clone()));

    // click a key → select
    {
        let state = state.clone();
        let board = board.clone();
        let weak = app.as_weak();
        app.on_key_clicked(move |idx| {
            let mut s = state.borrow_mut();
            s.selected = Some(idx as usize);
            board.set_vec(s.rows());
            if let Some(a) = weak.upgrade() {
                a.set_sel_label(s.sel_label().into());
            }
        });
    }
    // switch layer
    {
        let state = state.clone();
        let board = board.clone();
        let weak = app.as_weak();
        app.on_layer_clicked(move |l| {
            let mut s = state.borrow_mut();
            s.layer = l as u8;
            s.selected = None;
            s.read_layer();
            board.set_vec(s.rows());
            if let Some(a) = weak.upgrade() {
                a.set_current_layer(l);
                a.set_sel_label(s.sel_label().into());
            }
        });
    }
    // switch picker tab
    {
        let state = state.clone();
        let weak = app.as_weak();
        app.on_tab_clicked(move |i| {
            let mut s = state.borrow_mut();
            s.tab = tab_from_index(i);
            if let Some(a) = weak.upgrade() {
                a.set_current_tab(i);
                a.set_picker_rows(s.picker_rows(layer_count));
            }
        });
    }
    // pick a keycode → write to the selected key
    {
        let state = state.clone();
        let board = board.clone();
        let weak = app.as_weak();
        app.on_pick_clicked(move |kc| {
            let mut s = state.borrow_mut();
            if let Some(i) = s.selected {
                let (row, col) = (s.keys[i].row, s.keys[i].col);
                let ok = s
                    .via
                    .as_ref()
                    .map(|v| v.set_keycode(s.layer, row, col, kc as u16).is_ok())
                    .unwrap_or(false);
                if ok {
                    s.keymap.insert((row, col), kc as u16);
                }
                board.set_vec(s.rows());
                if let Some(a) = weak.upgrade() {
                    a.set_sel_label(s.sel_label().into());
                }
            }
        });
    }
    // reload from board
    {
        let state = state.clone();
        let board = board.clone();
        app.on_reload(move || {
            let mut s = state.borrow_mut();
            s.read_layer();
            board.set_vec(s.rows());
        });
    }

    app.run()?;
    Ok(())
}
