# Contributing to Nothing Linux

Nothing Linux is an unofficial, local-first Linux desktop controller for Nothing Ear 2024 (`B171`). Contributions are welcome when they preserve that safety boundary.

## Ground rules

- Do not add telemetry, account login, cloud calls, firmware flashing, factory reset, or brand assets from Nothing Technology Limited.
- Do not enable write commands for unsupported devices or unverified payloads.
- Keep `nothing-core` independent from GTK so protocol and session logic remains testable without a display server.
- Prefer small, reviewable pull requests with protocol evidence, screenshots, or test output when relevant.

## Development setup

Install Rust and native dependencies:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libdbus-1-dev libbluetooth-dev desktop-file-utils appstream
```

Run the standard checks before opening a pull request:

```sh
cargo fmt --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
desktop-file-validate data/io.github.nothinglinux.nothinglinux.desktop
appstreamcli validate --pedantic data/io.github.nothinglinux.nothinglinux.metainfo.xml
```

## Protocol work

Protocol changes need independent evidence. Include the model, firmware version, sanitized frame examples, and whether the command is read-only or mutating. Redact Bluetooth addresses, serial numbers, and other identifiers.

Unknown models must remain read-only unless their writes have been independently verified.
