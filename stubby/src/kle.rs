//! Minimal KLE (keyboard-layout-editor) decoder for VIA definitions.
//!
//! VIA definitions store the physical layout in KLE JSON: an array of rows, each
//! row an array of items. An item is either a string (a key, whose first label
//! line is "row,col") or an object that adjusts the cursor/size of the *next*
//! key (x/y offsets, w/h size). Enough for the V4 (no rotation, no layout opts).

use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Key {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub row: u8,
    pub col: u8,
}

/// Parse `definition["layouts"]["keymap"]` into positioned keys (units = 1u key).
pub fn parse(keymap: &Value) -> Vec<Key> {
    let mut keys = Vec::new();
    let rows = match keymap.as_array() {
        Some(r) => r,
        None => return keys,
    };

    let mut y = 0.0f32;
    for row in rows {
        let Some(items) = row.as_array() else { continue };
        let mut x = 0.0f32;
        let mut w = 1.0f32;
        let mut h = 1.0f32;

        for item in items {
            match item {
                Value::Object(props) => {
                    if let Some(v) = props.get("x").and_then(Value::as_f64) {
                        x += v as f32;
                    }
                    if let Some(v) = props.get("y").and_then(Value::as_f64) {
                        y += v as f32;
                    }
                    if let Some(v) = props.get("w").and_then(Value::as_f64) {
                        w = v as f32;
                    }
                    if let Some(v) = props.get("h").and_then(Value::as_f64) {
                        h = v as f32;
                    }
                }
                Value::String(label) => {
                    let first = label.split('\n').next().unwrap_or("");
                    if let Some((r, c)) = parse_matrix(first) {
                        keys.push(Key { x, y, w, h, row: r, col: c });
                    }
                    x += w;
                    w = 1.0;
                    h = 1.0;
                }
                _ => {}
            }
        }
        y += 1.0;
    }
    keys
}

fn parse_matrix(s: &str) -> Option<(u8, u8)> {
    let (r, c) = s.split_once(',')?;
    Some((r.trim().parse().ok()?, c.trim().parse().ok()?))
}
