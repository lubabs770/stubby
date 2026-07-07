# stubby

A **native, non-Electron** Keychron launcher for Linux. Talks to the keyboard
directly over raw HID using the **VIA protocol** — no browser, no WebHID, no
Chromium. Built on Keychron's own open-source releases:

- [`Keychron/keyboards`](https://github.com/Keychron/keyboards) — VIA device
  definitions (GPL-3.0). Bundled: `stubby/src/v4_ansi.json`.
- [`Keychron/qmk_firmware`](https://github.com/Keychron/qmk_firmware) — source of
  truth for the HID command bytes.

## This machine's keyboard

- **Keychron V4 ANSI**, USB `3434:0340`, 5×14 matrix.
- Raw-HID (VIA) interface = usage page `0xFF60` → currently `/dev/hidraw1`.

## Stack

Rust + [`hidapi`](https://crates.io/crates/hidapi) for transport, `egui` for the
GUI (added once transport is proven). Single static binary.

## Step 1 — grant HID access (one-time, needs sudo)

`/dev/hidraw*` is root-only by default. Install the udev rule, then replug the
keyboard (or reload rules):

```sh
sudo cp ~/stubby/99-stubby-keychron.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Tip: from the Claude Code prompt you can run this with a leading `!`.

## Step 2 — build & run the transport probe

```sh
cd ~/stubby/stubby
cargo run --bin stubby-probe
```

Expected: it enumerates the V4's interfaces, flags the raw-HID one, and prints
the protocol version, firmware version, uptime, and dynamic layer count. That
proves the wire protocol works end-to-end.

## Roadmap

- [x] Identify board, definition, and raw-HID interface
- [x] Milestone 1: VIA transport probe (`stubby-probe`)
- [ ] Read/write dynamic keymap buffer (remap keys)
- [ ] Parse `v4_ansi.json` layout → render key grid
- [ ] egui GUI: click a key, pick a keycode, write it live
- [ ] Lighting controls (V4 has RGB backlight)
