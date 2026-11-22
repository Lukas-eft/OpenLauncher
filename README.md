<div align="center">

<img src="assets/banner.svg" alt="OpenLauncher Banner" width="100%" />

<br />
<br />

[![Rust](https://img.shields.io/badge/Rust-2021-09090B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![egui](https://img.shields.io/badge/UI-eframe%20%2F%20egui-18181B?style=for-the-badge)](https://github.com/emilk/egui)
[![SQLite](https://img.shields.io/badge/Storage-SQLite%20WAL-18181B?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-27272A?style=for-the-badge)](https://github.com/Lukas-eft/OpenLauncher)
[![License: MIT](https://img.shields.io/badge/License-MIT-60a5fa?style=for-the-badge)](#license)

**A lightweight, native Rust game launcher for Windows and Linux.**
**Keep your Steam, Epic, GOG, Minecraft, and emulator games in one place with local SQLite playtime tracking.**

[Features](#-features) •
[Supported Platforms](#-supported-platforms--emulators) •
[Releases](#-releases) •
[Quick Start](#-getting-started) •
[Architecture](#%EF%B8%8F-architecture) •
[Disclaimer](#%EF%B8%8F-disclaimer)

</div>

---

## ✨ Features

- **Pure Rust Desktop App** — Built directly with `eframe` / `egui` without any WebView, Electron, or Node.js runtime.
- **Dark Monochrome Interface** — Borderless window with a 2:3 cover grid, favorites, tags, and detail view.
- **Automatic Store & Library Discovery** — Scans and imports games from:
  - **Steam** (parses `libraryfolders.vdf` / `appmanifest_*.acf`, resolves local `appcache/librarycache` covers & hero banners before CDN fallback)
  - **Epic Games Store** (reads `.item` manifests, pulls `DieselGameBoxTall` covers & `DieselGameBox` banners from `catcache.bin`, launches via `com.epicgames.launcher://`)
  - **GOG Galaxy** (reads Windows Registry entries)
  - **Minecraft** (detects **Minecraft Launcher** / Java Edition & **Minecraft for Windows** Bedrock UWP packages with logos and `shell:AppsFolder` launch)
- **Emulator Support** — Built-in presets for **Cemu**, **Dolphin**, **RPCS3**, **PCSX2**, **Ryujinx**, **Yuzu / Eden**, **PPSSPP**, **DuckStation**, **Citra / Azahar**, and **mGBA**:
  - Auto-detects emulator executables, Cemu `settings.xml` / `games` / `mlc01` paths, and `meta.xml` titles
  - Skips Wii U updates/DLCs (`0005000e` / `0005000c`) and converts Cemu `.tga` cover art (`bootTvTex.tga` / `iconTex.tga`) to managed `.png` covers
- **Background Playtime Tracking** — Tracks active game processes (including child/handoff processes and store URI launches) and saves sessions in SQLite (`WAL` mode).
- **Quick Search (`Ctrl+K`) & System Tray** — Fuzzy command palette with keyboard navigation (`↑↓`, `Enter`, `Shift+Enter`) and optional minimize-to-tray so playtime tracking stays active.

---

## 🎮 Supported Platforms & Emulators

| Category | Sources / Presets | Cover Resolution |
| :--- | :--- | :--- |
| **PC Stores** | Steam, Epic Games, GOG Galaxy, Minecraft | Local Steam `librarycache`, Epic `catcache.bin`, UWP logos, nearby `cover.*` |
| **PC Folders** | Custom folders (`D:\Games`, `.desktop`, Unreal `Binaries/Win64`) | Nearby `cover.*` / `boxart.*` / `poster.*` |
| **Emulators** | Cemu, Dolphin, RPCS3, PCSX2, Ryujinx, Yuzu/Eden, PPSSPP, DuckStation, Citra/Azahar, mGBA | Cemu `meta/*.tga` (auto-converted to PNG), nearby cover files |

---

## 📦 Releases

Pre-built Windows binaries (`openlauncher.exe`) are available on the [GitHub Releases](https://github.com/Lukas-eft/OpenLauncher/releases) page.

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain, edition 2021)
- Windows 10/11 (recommended) or Linux

### Build & Run

```bash
# Clone the repository
git clone https://github.com/Lukas-eft/OpenLauncher.git
cd OpenLauncher

# Run unit tests
cargo test

# Run linter
cargo clippy --all-targets -- -D warnings

# Build optimized release binary
cargo build --release
```

The compiled executable will be at:

```text
target/release/openlauncher.exe
```

---

## 🗂️ Architecture

```text
src/
├── main.rs            # Borderless eframe viewport bootstrap & Windows DWM styling
├── app.rs             # Application state, async workers, file dialogs & modal routing
├── db.rs              # SQLite schema, CRUD, bulk import, tags & play_sessions (WAL)
├── discovery.rs       # Epic/GOG/Minecraft/PC folder & emulator ROM/executable scanning
├── steam.rs           # VDF/ACF parser & local Steam librarycache asset resolver
├── covers.rs          # Managed cover storage, TGA->PNG conversion & URI resolution
├── launcher.rs        # Safe process, Steam, Epic URI & shell:AppsFolder launching + tracker
├── fuzzy.rs           # Fuzzy search scoring for the Ctrl+K command palette
├── tray.rs            # System tray icon, context menu & Win32 window restore
├── theme.rs           # Monochrome color palette & surface helpers
└── ui/                # Titlebar, Sidebar, TopBar, GameGrid, DetailModal, Forms, Settings
```

---

## ⚠️ Disclaimer

**OpenLauncher** is an independent, non-commercial personal hobby and educational project built from scratch to explore native desktop UI development and local game library management in Rust.

- **No Affiliation or Commercial Intent:** This project is not affiliated with, endorsed by, or connected to Valve / Steam, Epic Games, GOG, Mojang / Microsoft, Nintendo, or any other launcher, store, or emulator developer. It is purely a personal learning project and does not aim to compete with or replace any existing commercial or open-source product.
- **Takedown & Removal Requests:** All code in this repository was written independently. However, if you are a rights holder and believe any part of this repository infringes on your intellectual property, trademarks, or rights—or if you would like any content modified or taken down—please open an [Issue](https://github.com/Lukas-eft/OpenLauncher/issues) or contact the repository owner via GitHub, and it will be addressed or removed promptly.

---

## License

This project is released under the [MIT License](LICENSE).
