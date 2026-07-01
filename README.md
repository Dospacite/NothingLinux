# Nothing Linux

## Install

Nothing Linux currently supports **Nothing Ear (2024), model B171**, on 64-bit Linux.

### Linux Mint 22 / Ubuntu 24.04

1. Download the latest `nothing-linux_VERSION_amd64.deb` from the [Releases page](https://github.com/Dospacite/NothingLinux/releases).
2. Open a terminal in the download folder and install it:

   ```sh
   sudo apt install ./nothing-linux_*_amd64.deb
   ```

3. Open **Nothing Linux** from the application menu.

### AppImage

The AppImage is useful when you do not want to install a system package:

```sh
chmod +x nothing-linux_*_amd64.AppImage
./nothing-linux_*_amd64.AppImage
```

### Fedora

Download the latest `nothing-linux-VERSION-1.x86_64.rpm` from the [Releases page](https://github.com/Dospacite/NothingLinux/releases), then install it with:

```sh
sudo dnf install ./nothing-linux-*.x86_64.rpm
```

### Install From Source

Install the build dependencies and Rust, clone this repository, then run the current-user installer:

```sh
sudo apt update
sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev libdbus-1-dev libbluetooth-dev desktop-file-utils appstream git curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
git clone https://github.com/Dospacite/NothingLinux.git
cd NothingLinux
cargo run -p xtask -- install-user
```

The source installer places the application under `~/.local` for the current user.

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

Install a newer DEB or RPM with the same `apt install` or `dnf install` command used above. To remove a system package while keeping local settings and EQ profiles:

```sh
sudo apt remove nothing-linux
# Fedora: sudo dnf remove nothing-linux
```

To update an existing source installation:

```sh
cd NothingLinux
git pull
cargo run -p xtask -- install-user
```

To remove a source installation:

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
