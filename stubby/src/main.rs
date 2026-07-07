//! stubby — a native (no Electron/browser) Keychron launcher.
//!
//! Keychron-style Keymap screen: themeable palette (dark/light + user accent),
//! collapsible nav rail, shaded keycaps with embossed legends, centered board,
//! and a drag-resizable bottom keycode picker.

use std::collections::HashMap;

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rangef, Rect, Rounding, Sense, Stroke, Vec2};
use stubby::{
    keycodes::{self, Tab},
    kle,
    via::Via,
};

const DEF_JSON: &str = include_str!("v4_ansi.json");

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "stubby — Keychron V4",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

// ---- theme ------------------------------------------------------------------

#[derive(Clone)]
struct Theme {
    bg: Color32,
    panel: Color32,
    rail: Color32,
    cap_skirt: Color32,
    cap_top: Color32,
    cap_top_hover: Color32,
    cap_edge: Color32,
    sheen: Color32,
    sel_skirt: Color32,
    sel_top: Color32,
    accent: Color32,
    text: Color32,
    text_dim: Color32,
    ok: Color32,
    shadow: Color32,
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Theme {
    fn new(dark: bool, accent: Color32) -> Self {
        if dark {
            Theme {
                bg: rgb(0x15, 0x16, 0x1a),
                panel: rgb(0x1d, 0x1f, 0x24),
                rail: rgb(0x11, 0x12, 0x15),
                cap_skirt: rgb(0x24, 0x27, 0x2d),
                cap_top: rgb(0x3a, 0x3e, 0x47),
                cap_top_hover: rgb(0x48, 0x4d, 0x58),
                cap_edge: Color32::from_black_alpha(110),
                sheen: Color32::from_white_alpha(16),
                sel_skirt: accent.linear_multiply(0.5),
                sel_top: accent,
                accent,
                text: rgb(0xed, 0xef, 0xf3),
                text_dim: rgb(0x93, 0x98, 0xa2),
                ok: rgb(0x4c, 0xc4, 0x6a),
                shadow: Color32::from_black_alpha(160),
            }
        } else {
            Theme {
                bg: rgb(0xd9, 0xdc, 0xe3),
                panel: rgb(0xe8, 0xea, 0xef),
                rail: rgb(0xcf, 0xd3, 0xdb),
                cap_skirt: rgb(0xac, 0xb2, 0xbd),
                cap_top: rgb(0xe4, 0xe7, 0xed),
                cap_top_hover: rgb(0xf0, 0xf2, 0xf7),
                cap_edge: Color32::from_black_alpha(38),
                sheen: Color32::from_white_alpha(120),
                sel_skirt: accent.linear_multiply(0.7),
                sel_top: accent,
                accent,
                text: rgb(0x22, 0x25, 0x2b),
                text_dim: rgb(0x64, 0x69, 0x73),
                ok: rgb(0x2c, 0x9c, 0x4c),
                shadow: Color32::from_white_alpha(170),
            }
        }
    }

    fn apply(&self, ctx: &egui::Context, dark: bool) {
        let mut v = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.override_text_color = Some(self.text);
        v.panel_fill = self.bg;
        v.window_fill = self.panel;
        v.extreme_bg_color = self.rail;
        v.selection.bg_fill = self.accent.linear_multiply(0.4);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent);
        ctx.set_visuals(v);
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Screen {
    Keymap,
    Lighting,
    Settings,
}

struct App {
    keys: Vec<kle::Key>,
    keymap: HashMap<(u8, u8), u16>,
    custom: Vec<String>,
    via: Option<Via>,
    layer: u8,
    layer_count: u8,
    selected: Option<(u8, u8)>,
    screen: Screen,
    tab: Tab,
    kb_name: String,
    status: String,
    dark: bool,
    accent: Color32,
    rail_collapsed: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let def: serde_json::Value = serde_json::from_str(DEF_JSON).expect("bundled def parses");
        let keys = kle::parse(&def["layouts"]["keymap"]);
        let kb_name = def["name"].as_str().unwrap_or("Keyboard").to_string();
        let custom = def["customKeycodes"]
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
                let proto = v.protocol_version().unwrap_or(0);
                (Some(v), lc, format!("VIA proto {proto}"))
            }
            Err(e) => (None, 1, format!("OFFLINE — {e}")),
        };

        let accent = rgb(0xe0, 0x48, 0x48);
        Theme::new(true, accent).apply(&cc.egui_ctx, true);

        let mut app = App {
            keys,
            keymap: HashMap::new(),
            custom,
            via,
            layer: 0,
            layer_count,
            selected: None,
            screen: Screen::Keymap,
            tab: Tab::Basic,
            kb_name,
            status,
            dark: true,
            accent,
            rail_collapsed: false,
        };
        app.read_layer();
        app
    }

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
                self.status = format!("set [{r},{c}] = {}", keycodes::name_for(kc, &self.custom));
            }
            Err(e) => self.status = format!("write failed: {e}"),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let th = Theme::new(self.dark, self.accent);
        th.apply(ctx, self.dark);

        self.nav_rail(ctx, &th);
        self.top_bar(ctx, &th);
        if self.screen == Screen::Keymap {
            self.picker(ctx, &th);
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.screen {
            Screen::Keymap => self.keymap_screen(ui, &th),
            Screen::Lighting => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Lighting — coming in pass 2 (RGB effects, brightness, hue)")
                            .color(th.text_dim),
                    );
                });
            }
            Screen::Settings => self.settings_screen(ui, &th),
        });
    }
}

impl App {
    fn nav_rail(&mut self, ctx: &egui::Context, th: &Theme) {
        let width = if self.rail_collapsed { 56.0 } else { 140.0 };
        egui::SidePanel::left("rail")
            .exact_width(width)
            .resizable(false)
            .frame(egui::Frame::none().fill(th.rail).inner_margin(egui::Margin::symmetric(8.0, 14.0)))
            .show(ctx, |ui| {
                let items = [
                    (Screen::Keymap, "⌨", "Keymap"),
                    (Screen::Lighting, "☀", "Lighting"),
                    (Screen::Settings, "⚙", "Settings"),
                ];
                if self.rail_collapsed {
                    ui.vertical_centered(|ui| {
                        if ui
                            .add(egui::Button::new(egui::RichText::new("☰").size(18.0)).frame(false))
                            .on_hover_text("expand")
                            .clicked()
                        {
                            self.rail_collapsed = false;
                        }
                        ui.add_space(18.0);
                        for (screen, icon, label) in items {
                            let active = self.screen == screen;
                            let col = if active { th.text } else { th.text_dim };
                            let rich = egui::RichText::new(icon).size(18.0).color(col);
                            if ui
                                .add_sized([38.0, 32.0], egui::SelectableLabel::new(active, rich))
                                .on_hover_text(label)
                                .clicked()
                            {
                                self.screen = screen;
                            }
                            ui.add_space(4.0);
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("◫ stubby").strong().size(17.0).color(th.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new(egui::RichText::new("«").size(16.0)).frame(false))
                                .on_hover_text("collapse")
                                .clicked()
                            {
                                self.rail_collapsed = true;
                            }
                        });
                    });
                    ui.add_space(16.0);
                    for (screen, icon, label) in items {
                        let active = self.screen == screen;
                        let col = if active { th.text } else { th.text_dim };
                        let rich = egui::RichText::new(format!("{icon}  {label}")).size(15.0).color(col);
                        if ui
                            .add_sized([120.0, 32.0], egui::SelectableLabel::new(active, rich))
                            .clicked()
                        {
                            self.screen = screen;
                        }
                        ui.add_space(2.0);
                    }
                }
            });
    }

    fn top_bar(&mut self, ctx: &egui::Context, th: &Theme) {
        egui::TopBottomPanel::top("topbar")
            .frame(egui::Frame::none().fill(th.panel).inner_margin(egui::Margin::symmetric(16.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.kb_name).strong().size(16.0).color(th.text));
                    ui.add_space(10.0);
                    let (dot, col) = if self.via.is_some() {
                        ("● connected", th.ok)
                    } else {
                        ("● offline", th.accent)
                    };
                    ui.label(egui::RichText::new(dot).color(col).size(13.0));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(&self.status).color(th.text_dim).size(12.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚠ reset all").clicked() {
                            if let Some(via) = &self.via {
                                match via.reset_keymap() {
                                    Ok(()) => {
                                        self.status = "reset all layers to default".into();
                                        self.selected = None;
                                    }
                                    Err(e) => self.status = format!("reset failed: {e}"),
                                }
                            }
                            self.read_layer();
                        }
                        if ui.button("⟳ reload").clicked() {
                            self.read_layer();
                        }
                        ui.add_space(6.0);
                        let icon = if self.dark { "☀" } else { "🌙" };
                        if ui.button(egui::RichText::new(icon).size(14.0)).clicked() {
                            self.dark = !self.dark;
                        }
                        ui.color_edit_button_srgba(&mut self.accent).on_hover_text("accent colour");
                    });
                });
            });
    }

    fn settings_screen(&mut self, ui: &mut egui::Ui, th: &Theme) {
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.add_space(28.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Appearance").strong().size(18.0).color(th.text));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Theme").color(th.text_dim));
                    ui.add_space(12.0);
                    if ui.selectable_label(self.dark, "Dark").clicked() {
                        self.dark = true;
                    }
                    if ui.selectable_label(!self.dark, "Light").clicked() {
                        self.dark = false;
                    }
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Accent colour").color(th.text_dim));
                    ui.add_space(12.0);
                    ui.color_edit_button_srgba(&mut self.accent);
                    for (name, c) in [
                        ("red", rgb(0xe0, 0x48, 0x48)),
                        ("orange", rgb(0xe8, 0x8a, 0x2e)),
                        ("green", rgb(0x40, 0xb8, 0x60)),
                        ("blue", rgb(0x3d, 0x8b, 0xf0)),
                        ("purple", rgb(0x9a, 0x5c, 0xf0)),
                    ] {
                        if ui.add(swatch_button(c)).on_hover_text(name).clicked() {
                            self.accent = c;
                        }
                    }
                });
                ui.add_space(24.0);
                ui.label(egui::RichText::new("Device").strong().size(18.0).color(th.text));
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {} layers · {}",
                        self.kb_name, self.layer_count, self.status
                    ))
                    .color(th.text_dim),
                );
            });
        });
    }

    fn keymap_screen(&mut self, ui: &mut egui::Ui, th: &Theme) {
        let u = 62.0;
        let gap = 6.0;
        let max_x = self.keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
        let max_y = self.keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);
        let board_w = max_x * u + gap;
        let board_h = max_y * u + gap;
        let col_w = 44.0; // layer column
        let block_w = col_w + 20.0 + board_w;

        let avail = ui.available_size();
        // center vertically (bias slightly toward top) and horizontally
        let vpad = ((avail.y - board_h) * 0.42).max(10.0);
        ui.add_space(vpad);

        ui.horizontal(|ui| {
            let hpad = ((avail.x - block_w) * 0.5).max(8.0);
            ui.add_space(hpad);

            // layer column
            let mut switch = None;
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("LAYER").color(th.text_dim).size(11.0));
                ui.add_space(6.0);
                for l in 0..self.layer_count {
                    let (r, painter) = ui.allocate_painter(Vec2::splat(col_w), Sense::click());
                    let sel = self.layer == l;
                    keycap(&painter, r.rect, &l.to_string(), sel, r.hovered(), 16.0, th);
                    if r.clicked() {
                        switch = Some(l);
                    }
                    ui.add_space(6.0);
                }
            });
            if let Some(l) = switch {
                self.layer = l;
                self.selected = None;
                self.read_layer();
            }

            ui.add_space(20.0);
            // board
            ui.vertical(|ui| {
                let (resp, painter) =
                    ui.allocate_painter(Vec2::new(board_w, board_h), Sense::hover());
                let origin = resp.rect.min;
                let mut click = None;
                for k in &self.keys {
                    let rect = Rect::from_min_size(
                        origin + Vec2::new(k.x * u, k.y * u),
                        Vec2::new(k.w * u - gap, k.h * u - gap),
                    );
                    let kr = ui.interact(rect, resp.id.with((k.row, k.col)), Sense::click());
                    let sel = self.selected == Some((k.row, k.col));
                    let kc = self.keymap.get(&(k.row, k.col)).copied().unwrap_or(0);
                    keycap(&painter, rect, &keycodes::name_for(kc, &self.custom), sel, kr.hovered(), 13.0, th);
                    if kr.clicked() {
                        click = Some((k.row, k.col));
                    }
                }
                if let Some(sel) = click {
                    self.selected = Some(sel);
                }
            });
        });
    }

    fn picker(&mut self, ctx: &egui::Context, th: &Theme) {
        egui::TopBottomPanel::bottom("picker")
            .resizable(true)
            .default_height(320.0)
            .height_range(Rangef::new(170.0, 620.0))
            .frame(egui::Frame::none().fill(th.panel).inner_margin(egui::Margin::same(12.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for &(tab, label) in keycodes::TABS {
                        let active = self.tab == tab;
                        let txt = egui::RichText::new(label)
                            .size(14.0)
                            .color(if active { th.text } else { th.text_dim });
                        if ui.add(egui::SelectableLabel::new(active, txt)).clicked() {
                            self.tab = tab;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match self.selected {
                        Some((r, c)) => {
                            let cur = self.keymap.get(&(r, c)).copied().unwrap_or(0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "selected [{r},{c}] = {}",
                                    keycodes::name_for(cur, &self.custom)
                                ))
                                .color(th.accent)
                                .size(12.0),
                            );
                        }
                        None => {
                            ui.label(egui::RichText::new("click a key to select").color(th.text_dim).size(12.0));
                        }
                    });
                });
                ui.add_space(8.0);

                let entries = keycodes::entries(self.tab, self.layer_count, &self.custom);
                let mut assign = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let cell = Vec2::new(60.0, 46.0);
                    let gap = 7.0;
                    let avail = ui.available_width();
                    let cols = ((avail + gap) / (cell.x + gap)).floor().max(1.0) as usize;
                    let rows = entries.len().div_ceil(cols);
                    let (resp, painter) =
                        ui.allocate_painter(Vec2::new(avail, rows as f32 * (cell.y + gap)), Sense::hover());
                    let origin = resp.rect.min;
                    for (i, (label, kc)) in entries.iter().enumerate() {
                        let (cx, cy) = (i % cols, i / cols);
                        let rect = Rect::from_min_size(
                            origin + Vec2::new(cx as f32 * (cell.x + gap), cy as f32 * (cell.y + gap)),
                            cell,
                        );
                        let kr = ui.interact(rect, resp.id.with(i), Sense::click());
                        keycap(&painter, rect, label, false, kr.hovered(), 12.0, th);
                        if kr.clicked() {
                            assign = Some(*kc);
                        }
                    }
                });
                if let Some(kc) = assign {
                    self.assign(kc);
                }
            });
    }
}

/// Draw a shaded keycap: skirt (front lip) + top face + sheen + embossed legend.
fn keycap(
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    selected: bool,
    hovered: bool,
    size: f32,
    th: &Theme,
) {
    let (skirt, top) = if selected {
        (th.sel_skirt, th.sel_top)
    } else if hovered {
        (th.cap_skirt, th.cap_top_hover)
    } else {
        (th.cap_skirt, th.cap_top)
    };
    // skirt / front lip
    painter.rect(rect, Rounding::same(6.0), skirt, Stroke::NONE);
    // top face: inset, shifted up so the skirt shows as a front lip
    let top_rect = Rect::from_min_max(rect.min + Vec2::new(3.0, 2.0), rect.max - Vec2::new(3.0, 6.0));
    painter.rect(top_rect, Rounding::same(5.0), top, Stroke::new(1.0, th.cap_edge));
    // sheen: a lighter band across the upper part of the face (fake gradient)
    let sheen = if selected { Color32::from_white_alpha(28) } else { th.sheen };
    let hi = Rect::from_min_max(
        top_rect.min + Vec2::new(1.0, 1.0),
        Pos2::new(top_rect.max.x - 1.0, top_rect.min.y + top_rect.height() * 0.42),
    );
    painter.rect(
        hi,
        Rounding { nw: 5.0, ne: 5.0, sw: 0.0, se: 0.0 },
        sheen,
        Stroke::NONE,
    );

    if !label.is_empty() {
        let pos = top_rect.left_top() + Vec2::new(6.0, 4.0);
        let font = FontId::proportional(size);
        let (legend, shadow) = if selected {
            (Color32::WHITE, Color32::from_black_alpha(120))
        } else {
            (th.text, th.shadow)
        };
        painter.text(pos + Vec2::new(0.8, 1.0), Align2::LEFT_TOP, label, font.clone(), shadow);
        painter.text(pos, Align2::LEFT_TOP, label, font, legend);
    }
}

fn swatch_button(color: Color32) -> impl egui::Widget {
    move |ui: &mut egui::Ui| {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
        ui.painter().rect(rect, Rounding::same(4.0), color, Stroke::new(1.0, Color32::from_black_alpha(60)));
        resp
    }
}
