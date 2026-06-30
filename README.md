# Nothing Linux

Nothing Linux is an unofficial, fully local Rust/GTK4 controller focused on Nothing Ear 2024 (`B171`). It communicates directly with BlueZ over the vendor RFCOMM service and does not use `bluetoothctl`, an account, telemetry, or any network service.

This project is independent and is not affiliated with Nothing Technology Limited. It does not contain code, fonts, images, or other resources from Nothing X.

## Current scope

- Direct BlueZ discovery using the vendor service UUID, paired state, and a model query; no name or address-prefix matching.
- CRC16-framed channel-15 session with activation, synchronization, partial/multiple-frame parsing, acknowledgements, readback, timeouts, and bounded recovery. RFCOMM readiness is verified before the first write; a confirmed stale `ENOTCONN` link triggers one rate-limited BlueZ reconnect repair.
- Live battery, wear, firmware, ANC, EQ, gestures, Bass Enhance, in-ear detection, and low-lag state.
- Confirmed B171 writes for ANC, preset, three-band EQ, advanced EQ, gesture mapping, Bass Enhance, in-ear detection, low-lag mode, high-quality audio codec selection, dual connection, find earbuds, and the ear-tip fit test.
- Local advanced-EQ editor and profiles using the B171 eight-band payload.
- Unknown models never receive write commands.
- XDG configuration, atomic writes, up to 20 JSON EQ profiles, opt-in autostart, and redacted diagnostics.

Firmware flashing, factory reset, Mimi/Hearing ID enrollment, and cloud services are intentionally excluded.

## Build and run

Install Rust stable plus the GTK development packages. On Ubuntu 24.04 / Linux Mint 22:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libdbus-1-dev libbluetooth-dev desktop-file-utils appstream
```

Then:

```sh
cargo run --release -p nothing-linux
```

Pair and connect the earbuds through the desktop Bluetooth settings before starting the app. To install from source for the current user:

```sh
cargo run -p xtask -- install-user
```

This installs into `~/.local`. Remove the installed application files with `cargo run -p xtask -- uninstall-user`; user configuration remains intact.

## Development

```sh
cargo fmt --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
desktop-file-validate data/io.github.nothinglinux.nothinglinux.desktop
appstreamcli validate --pedantic data/io.github.nothinglinux.nothinglinux.metainfo.xml
```

See [docs/protocol.md](docs/protocol.md) for the independently documented protocol and safety boundary.
