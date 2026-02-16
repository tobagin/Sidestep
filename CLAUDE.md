# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

The app is built and run as a Flatpak:

```bash
# Build & install development Flatpak (includes SDK setup)
./build.sh --dev

# Run development build
flatpak run io.github.tobagin.Sidestep.Dev

# Build & install production Flatpak
./build.sh
flatpak run io.github.tobagin.Sidestep
```

Direct cargo build works for type-checking but won't produce a runnable binary (gresources won't be compiled, config.rs paths won't be set by meson):
```bash
cargo check
```

## Project Overview

Sidestep is a GTK4/Libadwaita desktop app (Rust, Flatpak) that guides users through unlocking Android bootloaders and installing mobile Linux distributions. It targets GNOME and uses a wizard-style flow.

## Architecture

### Module Layers

- **`hardware/`** — Device communication: `DeviceDetector` polls USB via tokio background threads, wraps `adb` and `fastboot` CLI tools. Binary paths configurable via `ADB_PATH`/`FASTBOOT_PATH` env vars.
- **`models/`** — Data types: `Device`, `Distro`, `DeviceDatabase`. Device database loaded from YAML files in `data/devices/{manufacturer}/{codename}/`.
- **`flashing/`** — Installation engine: download (`reqwest` + progress), decompress (XZ/GZIP), verify (SHA256), flash (fastboot commands). `ubports.rs` is the Ubuntu Touch installer implementation.
- **`pages/`** — GTK composite template widgets for each wizard screen. UI defined in Blueprint language (`.blp` files in `data/ui/pages/`).
- **`wizard/`** — `WizardController` state machine managing the install flow.
- **`wizards/`** — Wizard implementations (e.g., `install_wizard.rs`).
- **`utils/`** — YAML parser for device database.

### Application Flow

```
main.rs → SidestepApplication → SidestepWindow (NavigationView)
  → WaitingPage (USB polling) → DeviceDetailsPage → DistroSelectionPage
  → FlashingPage (download/decompress/verify/flash) → SuccessPage
```

### GTK/GObject Patterns

All custom widgets use the GObject subclass pattern:
- Inner `mod imp` with `#[derive(gtk::CompositeTemplate)]` struct
- `#[glib::object_subclass]` impl block
- `glib::wrapper!` macro for the public type
- Blueprint `.blp` templates compiled to `.ui` XML via `blueprint-compiler`
- Custom signals (e.g., `"device-selected"`, `"distro-selected"`, `"installation-complete"`) for inter-widget communication

### Key Gotcha: AdwPreferencesGroup

Do NOT use `first_child()`/`remove()` loops to clear rows from `AdwPreferencesGroup`. The `first_child()` returns internal GtkBox structure, not added rows. Instead, track added rows in a `Vec` and remove them by reference.

## Device Database

Device configs live in `data/devices/{manufacturer}/{codename}/`:
- `info.yml` — Device specs and metadata
- `distros.yml` — Available distributions
- `installers/{distro}.yml` — Distro-specific installer config

Currently supported: `google/sargo` (Pixel 3a), `oneplus/enchilada` (OnePlus 6), `microsoft/zeta` (Surface Duo).

## Android Flashing Notes

- Platform-tools pinned to **33.0.3** (fastboot 36.0.2 has a regression with `--disable-verity --disable-verification`)
- vbmeta flags format: `fastboot flash vbmeta --disable-verity --disable-verification file.img`
- userdata: must use `fastboot format:ext4 userdata` (not `erase`)

## Key Dependencies

- **gtk4 0.10** (v4_12), **libadwaita 0.8** (v1_6) — UI framework
- **tokio** — Async runtime for device polling and subprocess management
- **reqwest 0.11** (rustls-tls) — HTTP downloads
- **serde + serde_yaml** — Device database parsing
- **flate2 / xz2** — Image decompression
- **sha2** — Checksum verification

## Configuration

- `config.rs` — Build-time constants (APP_ID, VERSION, paths). Values are hardcoded defaults; meson overwrites them during Flatpak builds.
- `meson_options.txt` — `profile` option: `default` (production) or `development`
- Logging: `RUST_LOG=sidestep=debug` (default filter: `sidestep=info`)
