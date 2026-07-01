# Nothing Linux

## Install

Nothing Linux currently supports **Nothing Ear (2024), model B171**, on Linux. There is not yet a packaged `.deb`, Flatpak, or AppImage, so the installer builds the application once and installs it for your user account.

### Linux Mint 22 / Ubuntu 24.04

1. Install the required system packages:

   ```sh
   sudo apt update
   sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev libdbus-1-dev libbluetooth-dev desktop-file-utils appstream git curl
   ```

2. Install Rust using the official installer:

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

   Choose the default installation when prompted.

3. Download and install Nothing Linux:

   ```sh
   git clone https://github.com/Dospacite/NothingLinux.git
   cd NothingLinux
   cargo run -p xtask -- install-user
   ```

4. Open **Nothing Linux** from the application menu. If it does not appear immediately, sign out and back in once.

The application and launcher are installed under `~/.local`; administrator access is not used after the system packages are installed.

## First Use

1. Pair and connect the earbuds from the desktop Bluetooth settings.
2. Put at least one earbud outside the case.
3. Open Nothing Linux. It discovers the paired B171 device automatically and shows **Connected** when synchronization finishes.
4. Use the navigation bar to open Overview, Noise Control, Equalizer, Controls, and More.

Closing the window can leave Nothing Linux running in the system tray for battery monitoring and quick ANC controls. Use **Quit** from the tray or press `Ctrl+Q` to stop it. **Start at login** is available on the More screen.

## Features

### Device Status

- Live left, right, and case battery levels.
- Dynamic left/right **in ear** and **not worn** status.
- Connection, synchronization, firmware, and charging state.
- Local system tray with battery status, open, ANC, and quit actions.

### Noise Control

- Noise cancellation with High, Mid, Low, and Adaptive strength.
- Transparency and Off modes.
- Immediate visual updates with device readback confirmation.

### Equalizer

- Simple and Advanced tabs following the Nothing X editing workflow.
- Simple three-band Bass, Mid, and Treble controls with synchronized sliders and circular knobs.
- Balanced, More Bass, More Treble, Voice, and Custom presets.
- Read-only response graphs; all editing is performed with sliders and knobs.
- Eight-band advanced EQ with gain, frequency, and Q controls.
- Up to 20 local advanced-EQ profiles with add, delete, reset, undo, and redo.
- Rapid slider changes remain stable without stale device responses moving the controls.

### Earbud Controls

- Per-earbud mappings for double pinch, triple pinch, pinch and hold, and double pinch and hold.
- Supported actions include track navigation, voice assistant, noise control, volume, and None.

### More Controls

- Bass Enhance enable and level.
- In-ear detection and low-lag mode.
- Default, LHDC, and LDAC codec selection.
- Dual Connection toggle and paired-device connection list.
- Find My Earbuds controls with worn-state safety checks.
- Ear-tip fit test start and cancel controls.

### Reliable Updates

Controls update visually as soon as they are changed. Nothing Linux then reads the setting back from the earbuds. If the device reports a different value, returns an error, disconnects, or does not confirm within three seconds, the control returns to the last confirmed value. A spinner in the top bar indicates pending device updates.

## Privacy and Compatibility

Nothing Linux communicates directly with BlueZ over the Nothing vendor RFCOMM service. It does not use an account, telemetry, cloud service, or network connection. Configuration and EQ profiles are stored locally in XDG user directories, and diagnostics redact device addresses and serial numbers.

Write commands are enabled only after the device identifies as B171. Unknown models remain read-only. Firmware flashing, factory reset, Mimi/Hearing ID enrollment, and cloud services are intentionally excluded.

This project is independent and is not affiliated with Nothing Technology Limited. It does not contain code, fonts, images, or other resources from Nothing X.

## Update or Remove

To update an existing source installation:

```sh
cd NothingLinux
git pull
cargo run -p xtask -- install-user
```

To remove the application while keeping local settings and EQ profiles:

```sh
cd NothingLinux
cargo run -p xtask -- uninstall-user
```

## Development

Run directly from the repository:

```sh
cargo run --release -p nothing-linux
```

Run the complete validation suite:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
desktop-file-validate data/io.github.nothinglinux.nothinglinux.desktop
appstreamcli validate --pedantic data/io.github.nothinglinux.nothinglinux.metainfo.xml
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution requirements and [docs/protocol.md](docs/protocol.md) for the independently documented protocol and safety boundary.
