# stubby — project instructions

Native (non-Electron) Keychron launcher for Linux. Talks to the keyboard
directly over raw HID using the **VIA protocol** — no browser, no WebHID, no
Electron. Built on Keychron's open-source releases
([`Keychron/keyboards`](https://github.com/Keychron/keyboards) VIA definitions,
[`Keychron/qmk_firmware`](https://github.com/Keychron/qmk_firmware) for the
protocol).

## Account / context

**Personal.** GitHub repo `lubabs770/stubby` (public), remote named `stubby`.
Commit author email MUST be the personal noreply
`246544701+lubabs770@users.noreply.github.com` — never a real address (the repo
is public). Never mix with the work `shmuelnewman`/careflow context.

## Hardware / runtime

- Keychron **V4 ANSI**, USB `3434:0340`, 5×14 matrix.
- Raw-HID (VIA) interface = usage page `0xFF60` (the app finds it by usage page,
  not by a fixed `/dev/hidrawN`).
- Requires the udev rule installed so hidraw is group-accessible (`wheel`),
  otherwise "permission denied":
  `sudo cp 99-stubby-keychron.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=hidraw`
  (logind `uaccess` alone does NOT work for this keyboard's hidraw on this box —
  that's why the rule uses explicit `GROUP="wheel", MODE="0660"`).

## Layout

- `stubby/src/via.rs` — raw-HID VIA transport: keymap get/set, layer count,
  reset, and lighting over the VIA3 custom channel 3 (qmk_rgb_matrix — the
  wire protocol the V4's proto-12 firmware actually uses, not qmk_rgblight)
- `stubby/src/kle.rs` — KLE layout decoder
- `stubby/src/keycodes.rs` — keycode labels + assignment palette
- `stubby/src/slint_main.rs` + `stubby/ui/app.slint` — the app (bin `stubby`,
  Slint with Material style set in `build.rs`)
- `stubby/src/main.rs` — earlier egui prototype (bin `stubby-egui`, kept for
  reference; not maintained)
- `stubby/src/probe.rs` — minimal transport probe (bin `stubby-probe`)
- `stubby/src/v4_ansi.json` — vendored VIA definition (GPL-3.0, from `Keychron/keyboards`)

The cargo project root is `stubby/` (a subdirectory of the repo).

## Workflow — run and verify the UI yourself

- **Launch the app yourself** after building; don't hand the user a run
  command. `pkill -x stubby` (exact match — `pkill -f` matches its own command
  line and kills the shell), then run `stubby/target/debug/stubby` in the
  background.
- **Screenshot your UI work and look at it before handing over.** Get window
  geometry from `hyprctl clients -j` (match on title containing `V4`; the
  window class is empty), then `grim -g "X,Y WxH" out.png`. Re-query geometry
  for every shot — the window moves/resizes as the user interacts and stale
  coords crop the window wrong.
- Env helpers for screenshots: `STUBBY_PAGE=1` opens on the Lighting page,
  `STUBBY_DARK=0` starts in light mode.
- You can't float/resize windows from the CLI on this box: `hyprctl dispatch`
  is Lua-flavored (wants `hl.dsp.*` dispatcher objects) and `hyprctl keyword`
  rejects non-legacy parsers. To frame a shot, grim a computed sub-region
  instead.
- Icons: **Material Symbols Rounded** is installed system-wide; use ligature
  names (`"dark_mode"`, `"refresh"`) in a Text with that font-family. Emoji do
  NOT render in Slint's text stack.
- Slint layout gotcha: a fixed `width:`/`height:` on a layout child also caps
  the parent layout's max size, and fixed heights act as window minimums (a
  drag-to-grow control can enter a window-resize feedback loop). Prefer
  `preferred-*` sizes, or float fixed-size content centered inside an
  unconstrained container.

## BUILD RULE — do not full/clean-build on this machine

This runs on a thinkpad. A cold `cargo build` compiles ~250 crates (eframe,
winit, wgpu-adjacent stack) and takes minutes — do not do it locally.

- **OK locally:** incremental `cargo build` / `cargo run` while iterating (a
  warm rebuild of just `stubby` is a few seconds). `cargo check` for type-checks.
- **NEVER locally:** `cargo clean` followed by a rebuild; `--release` builds;
  building fresh artifacts for distribution.
- **Release/artifact builds run on GitHub CI**, not here. To produce a binary:
  push a tag `vX.Y.Z` (or use the manual dispatch), which runs
  `.github/workflows/release.yml` on a runner (`cargo build --release --locked`)
  and attaches `stubby-linux-x86_64` to the GitHub Release.
  - Tag trigger: `git tag vX.Y.Z && git push stubby vX.Y.Z`
  - Manual trigger: `gh workflow run release.yml`
  - Watch it: `gh run watch` / `gh run list --workflow=release.yml`

If a full/release build is ever genuinely needed, offload it to CI and pull the
artifact — do not compile it on this machine.
