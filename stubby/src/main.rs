//! stubby — a native (no Electron/browser) Keychron launcher.
//!
//! Reads the live keymap off the board over raw HID, renders the physical layout
//! from the bundled VIA definition, and writes remaps back instantly on click.

use std::collections::HashMap;

use eframe::egui;
use stubby::{keycodes, kle, via::Via};

const DEF_JSON: &str = include_str!("v4_ansi.json");

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([980.0, 470.0]),
        ..Default::default()
    };
    eframe::run_native(
        "stubby — Keychron V4",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

struct App {
    keys: Vec<kle::Key>,
    keymap: HashMap<(u8, u8), u16>,
    via: Option<Via>,
    layer: u8,
    layer_count: u8,
    selected: Option<(u8, u8)>,
    filter: String,
    status: String,
}

impl App {
    fn new() -> Self {
        let def: serde_json::Value = serde_json::from_str(DEF_JSON).expect("bundled def parses");
        let keys = kle::parse(&def["layouts"]["keymap"]);

        let (via, layer_count, status) = match Via::open() {
            Ok(v) => {
                let lc = v.layer_count().unwrap_or(1);
                let proto = v.protocol_version().unwrap_or(0);
                (Some(v), lc, format!("connected · VIA proto {proto} · {lc} layers"))
            }
            Err(e) => (None, 1, format!("OFFLINE: {e}")),
        };

        let mut app = App {
            keys,
            keymap: HashMap::new(),
            via,
            layer: 0,
            layer_count,
            selected: None,
            filter: String::new(),
            status,
        };
        app.read_layer();
        app
    }

    /// Pull every key's keycode for the current layer off the board.
    fn read_layer(&mut self) {
        self.keymap.clear();
        let Some(via) = &self.via else { return };
        for k in &self.keys {
            match via.get_keycode(self.layer, k.row, k.col) {
                Ok(kc) => {
                    self.keymap.insert((k.row, k.col), kc);
                }
                Err(e) => {
                    self.status = format!("read {},{} failed: {e}", k.row, k.col);
                    break;
                }
            }
        }
    }

    /// Write a keycode to the selected key and update local state.
    fn assign(&mut self, kc: u16) {
        let Some((r, c)) = self.selected else {
            self.status = "select a key first".into();
            return;
        };
        let Some(via) = &self.via else {
            self.status = "not connected".into();
            return;
        };
        match via.set_keycode(self.layer, r, c, kc) {
            Ok(()) => {
                self.keymap.insert((r, c), kc);
                self.status = format!("set [{r},{c}] = {} on layer {}", keycodes::name_for(kc), self.layer);
            }
            Err(e) => self.status = format!("write failed: {e}"),
        }
    }
}

// palette
const KEY_FILL: egui::Color32 = egui::Color32::from_rgb(48, 50, 58);
const KEY_HOVER: egui::Color32 = egui::Color32::from_rgb(70, 74, 86);
const KEY_SEL: egui::Color32 = egui::Color32::from_rgb(90, 120, 200);
const KEY_BORDER: egui::Color32 = egui::Color32::from_rgb(90, 94, 104);
const KEY_TEXT: egui::Color32 = egui::Color32::from_rgb(225, 227, 233);

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("stubby");
                ui.separator();
                ui.label("Layer:");
                let mut switch = None;
                for l in 0..self.layer_count {
                    if ui.selectable_label(self.layer == l, format!("{l}")).clicked() {
                        switch = Some(l);
                    }
                }
                if let Some(l) = switch {
                    self.layer = l;
                    self.selected = None;
                    self.read_layer();
                }
                ui.separator();
                if ui.button("⟳ reload").clicked() {
                    self.read_layer();
                }
                if ui.button("⚠ reset all layers").clicked() {
                    if let Some(via) = &self.via {
                        match via.reset_keymap() {
                            Ok(()) => {
                                self.status = "reset all layers to firmware default".into();
                                self.selected = None;
                            }
                            Err(e) => self.status = format!("reset failed: {e}"),
                        }
                    } else {
                        self.status = "not connected".into();
                    }
                    self.read_layer();
                }
            });
            ui.label(&self.status);
        });

        egui::SidePanel::right("palette")
            .default_width(230.0)
            .show(ctx, |ui| {
                ui.heading("Assign");
                match self.selected {
                    Some((r, c)) => {
                        let cur = self.keymap.get(&(r, c)).copied().unwrap_or(0);
                        ui.label(format!("key [{r},{c}] = {} (0x{cur:04X})", keycodes::name_for(cur)));
                    }
                    None => {
                        ui.label("click a key to select it");
                    }
                }
                ui.add_space(4.0);
                ui.text_edit_singleline(&mut self.filter);
                ui.separator();

                let filter = self.filter.to_lowercase();
                let mut assign = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut last_group = "";
                    for (group, label, kc) in keycodes::palette() {
                        if !filter.is_empty() && !label.to_lowercase().contains(&filter) {
                            continue;
                        }
                        if group != last_group {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(group).weak().small());
                            last_group = group;
                        }
                        if ui.button(label).clicked() {
                            assign = Some(kc);
                        }
                    }
                });
                if let Some(kc) = assign {
                    self.assign(kc);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let u = 58.0;
            let gap = 5.0;
            let max_x = self.keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
            let max_y = self.keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);
            let desired = egui::vec2(max_x * u + gap, max_y * u + gap);

            let (resp, painter) = ui.allocate_painter(desired, egui::Sense::hover());
            let origin = resp.rect.min;

            let mut click = None;
            for k in &self.keys {
                let min = origin + egui::vec2(k.x * u, k.y * u);
                let size = egui::vec2(k.w * u - gap, k.h * u - gap);
                let rect = egui::Rect::from_min_size(min, size);
                let id = resp.id.with((k.row, k.col));
                let kr = ui.interact(rect, id, egui::Sense::click());

                let selected = self.selected == Some((k.row, k.col));
                let fill = if selected {
                    KEY_SEL
                } else if kr.hovered() {
                    KEY_HOVER
                } else {
                    KEY_FILL
                };
                painter.rect(rect, 4.0, fill, egui::Stroke::new(1.0, KEY_BORDER));

                let kc = self.keymap.get(&(k.row, k.col)).copied().unwrap_or(0);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    keycodes::name_for(kc),
                    egui::FontId::proportional(14.0),
                    KEY_TEXT,
                );

                if kr.clicked() {
                    click = Some((k.row, k.col));
                }
            }
            if let Some(sel) = click {
                self.selected = Some(sel);
            }
        });
    }
}
