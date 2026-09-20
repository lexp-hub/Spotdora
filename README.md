<div align="center">
  <img src="data/hicolor/scalable/apps/dev.lex.Spotdora.svg" alt="Spotdora" width="128" height="128" />
  <h1>Spotdora</h1>
  <p align="center">
    <strong>A native GTK4/Libadwaita Spotify client optimized for Fedora Workstation</strong>
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/Spotify-Client-1DB954?style=flat-square&logo=spotify&logoColor=white" alt="Spotify Client" />
    <img src="https://img.shields.io/badge/Fedora-Workstation_40+-51A2DA?style=flat-square&logo=fedora&logoColor=white" alt="Fedora" />
    <img src="https://img.shields.io/badge/Toolkit-GTK4_&_Libadwaita-4A90D9?style=flat-square&logo=gnome&logoColor=white" alt="GTK4 & Libadwaita" />
    <img src="https://img.shields.io/badge/Display-Native_Wayland-4E9A06?style=flat-square&logo=wayland&logoColor=white" alt="Wayland" />
    <img src="https://img.shields.io/badge/Audio-PipeWire-009688?style=flat-square" alt="PipeWire" />
    <img src="https://img.shields.io/badge/Packaging-Native_RPM-CC342D?style=flat-square&logo=redhat" alt="RPM" />
    <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License" />
  </p>
</div>

<br>

Spotdora is an open-source native Spotify client written in Rust and GTK4/Libadwaita, tailored specifically for Fedora Workstation (GNOME, Wayland, and PipeWire). Built on top of librespot, it provides a fast, responsive Spotify experience with deep desktop integration and out-of-the-box system optimizations.

> [!NOTE]
> **A Spotify Premium account is required.**

## Features

- **PipeWire & WirePlumber Integration**: Automatic audio stream metadata classification (`media.role=music`, application name, and icon), seamlessly integrating into GNOME Shell Quick Settings and GNOME Control Center volume mixers. Supports optimized GStreamer `pipewiresink` pipelines.
- **Native Wayland & Libadwaita**: Crisp, hardware-accelerated rendering with full fractional scaling and smooth touchpad gestures on GNOME Wayland. Adheres to GNOME Human Interface Guidelines (HIG) with automatic dark/light theme switching.
- **GNOME Shell & MPRIS Integration**: Complete MPRIS D-Bus implementation with `PlayPause`, `Next`, `Previous`, `Seek`, `Stop`, `OpenURI`, and `Quit` support. Includes GNOME Shell Dash/Dock right-click Quick Actions.
- **High Performance Rust Engine**: Powered by librespot with release-profile optimizations (**ThinLTO**, single codegen unit, stripped symbols, and panic-abort) for instant cold startup and minimal RAM usage.
- **Secret Service Credential Storage**: Secure, automatic credential management out of the box via GNOME Keyring / FreeDesktop Secret Service.
- **Native Fedora RPM Packaging**: Production-ready RPM spec file with AppStream metadata for GNOME Software and an optional `systemd --user` service unit.

## Installation

### Using DNF (RPM)

Install the RPM package using DNF:

```sh
sudo dnf install ./dist/spotdora-0.5.1-1.fc44.x86_64.rpm
```

### Quick Build & Install Script

Use the built-in helper script to install build dependencies and compile locally:

```sh
# Install dependencies, build release binary, and install to ~/.local/bin
./build-aux/build-fedora.sh --all
```

## Building from Source

### Prerequisites

Install build tools and required development libraries:

```sh
sudo dnf install gcc meson ninja-build cargo rust \
    gtk4-devel libadwaita-devel blueprint-compiler \
    openssl-devel alsa-lib-devel pulseaudio-libs-devel \
    pipewire-pulseaudio pipewire-gstreamer desktop-file-utils libappstream-glib
```

### Build

```sh
git clone https://github.com/lexp-hub/Spotdora.git
cd Spotdora
meson setup target -Dbuildtype=release -Doffline=false --prefix="$HOME/.local"
ninja -C target
ninja install -C target
```

Ensure `~/.local/bin` is in your `$PATH`.

### Build RPM Package

To package Spotdora into a native RPM for Fedora:

```sh
./build-aux/build-fedora.sh --rpm
```

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Space</kbd> | Play / Pause |
| <kbd>N</kbd> | Next Track |
| <kbd>P</kbd> | Previous Track |
| <kbd>Ctrl</kbd> + <kbd>F</kbd> | Search |
| <kbd>Alt</kbd> + <kbd>←</kbd> | Navigate Back |
| <kbd>Ctrl</kbd> + <kbd>Q</kbd> | Quit Application |

## Development

- Run development build: `ninja -C target run`
- Run unit tests: `meson test -C target --verbose`
- Check lints: `cargo clippy`

## License

MIT License. See [LICENSE](LICENSE).
