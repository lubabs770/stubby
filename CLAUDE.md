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

- `stubby/src/via.rs` — raw-HID VIA transport (get/set keycode, layer count, reset)
- `stubby/src/kle.rs` — KLE layout decoder
- `stubby/src/keycodes.rs` — keycode labels + assignment palette
- `stubby/src/main.rs` — egui GUI (bin `stubby`)
- `stubby/src/probe.rs` — minimal transport probe (bin `stubby-probe`)
- `stubby/src/v4_ansi.json` — vendored VIA definition (GPL-3.0, from `Keychron/keyboards`)

The cargo project root is `stubby/` (a subdirectory of the repo).

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
