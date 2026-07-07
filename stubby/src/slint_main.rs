//! stubby — Slint (native, no Electron) Keychron launcher.
//!
//! Material-styled chrome + custom keycap components rendering the live keymap.
//! Board render, click-to-select, layer switching, reload, and the bottom tabbed
//! keycode picker with live assignment. Reuses via/kle/keycodes unchanged.

slint::include_modules!();

use std::cell::RefCell;
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

/// A named colour theme. Values are 0xRRGGBB.
struct ThemeDef {
    name: &'static str,
    bg: u32,
    panel: u32,
    cap: u32,
    cap_hover: u32,
    cap_text: u32,
    text: u32,
    dim: u32,
    accent: u32,
    ok: u32,
}

const THEMES: &[ThemeDef] = &[
    ThemeDef { name: "Dark",             bg: 0x15161a, panel: 0x1d1f24, cap: 0x3a3e47, cap_hover: 0x484d58, cap_text: 0xffffff, text: 0xededf3, dim: 0x93989f, accent: 0xe04848, ok: 0x4cc46a },
    ThemeDef { name: "Light",            bg: 0xd9dce3, panel: 0xe8eaef, cap: 0xf3f5f8, cap_hover: 0xffffff, cap_text: 0x24272d, text: 0x22252b, dim: 0x64697d, accent: 0xe04848, ok: 0x2c9c4c },
    ThemeDef { name: "Nord",             bg: 0x2e3440, panel: 0x3b4252, cap: 0x434c5e, cap_hover: 0x4c566a, cap_text: 0xeceff4, text: 0xeceff4, dim: 0x9aa4b8, accent: 0x88c0d0, ok: 0xa3be8c },
    ThemeDef { name: "Dracula",          bg: 0x282a36, panel: 0x21222c, cap: 0x44475a, cap_hover: 0x565872, cap_text: 0xf8f8f2, text: 0xf8f8f2, dim: 0x8a8fa3, accent: 0xbd93f9, ok: 0x50fa7b },
    ThemeDef { name: "Gruvbox",          bg: 0x282828, panel: 0x1d2021, cap: 0x3c3836, cap_hover: 0x504945, cap_text: 0xebdbb2, text: 0xebdbb2, dim: 0xa89984, accent: 0xfe8019, ok: 0xb8bb26 },
    ThemeDef { name: "Solarized Dark",   bg: 0x002b36, panel: 0x073642, cap: 0x0a4a58, cap_hover: 0x0f5a6a, cap_text: 0x93a1a1, text: 0xeee8d5, dim: 0x839496, accent: 0x268bd2, ok: 0x859900 },
    ThemeDef { name: "Catppuccin Mocha", bg: 0x1e1e2e, panel: 0x181825, cap: 0x313244, cap_hover: 0x45475a, cap_text: 0xcdd6f4, text: 0xcdd6f4, dim: 0xa6adc8, accent: 0xcba6f7, ok: 0xa6e3a1 },
    ThemeDef { name: "Catppuccin Latte", bg: 0xeff1f5, panel: 0xe6e9ef, cap: 0xffffff, cap_hover: 0xdce0e8, cap_text: 0x4c4f69, text: 0x4c4f69, dim: 0x6c6f85, accent: 0x8839ef, ok: 0x40a02b },
    ThemeDef { name: "Tokyo Night",      bg: 0x1a1b26, panel: 0x16161e, cap: 0x2a2e42, cap_hover: 0x3b4261, cap_text: 0xc0caf5, text: 0xc0caf5, dim: 0x9aa5ce, accent: 0x7aa2f7, ok: 0x9ece6a },
    ThemeDef { name: "Rosé Pine",        bg: 0x191724, panel: 0x1f1d2e, cap: 0x26233a, cap_hover: 0x393552, cap_text: 0xe0def4, text: 0xe0def4, dim: 0x908caa, accent: 0xebbcba, ok: 0x9ccfd8 },
];

fn col(v: u32) -> slint::Color {
    slint::Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn apply_theme(app: &AppWindow, i: usize) {
    let t = &THEMES[i.min(THEMES.len() - 1)];
    app.set_t_bg(col(t.bg));
    app.set_t_panel(col(t.panel));
    app.set_t_cap(col(t.cap));
    app.set_t_cap_hover(col(t.cap_hover));
    app.set_t_cap_text(col(t.cap_text));
    app.set_t_text(col(t.text));
    app.set_t_dim(col(t.dim));
    app.set_t_ok(col(t.ok));
    app.set_accent(col(t.accent));
    app.set_current_theme(i as i32);
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

    let theme_names: Vec<SharedString> = THEMES.iter().map(|t| t.name.into()).collect();
    app.set_theme_names(ModelRc::new(VecModel::from(theme_names)));
    apply_theme(&app, 0);
    {
        let weak = app.as_weak();
        app.on_theme_selected(move |i| {
            if let Some(a) = weak.upgrade() {
                apply_theme(&a, i.max(0) as usize);
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
