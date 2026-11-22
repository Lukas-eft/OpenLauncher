# OpenLauncher

Leichtgewichtiger, 100 % nativer Rust-Game-Launcher für Windows & Linux im **Schwarz/Grau-Liquid-Glass-Design** (`eframe` / `egui`) mit lokaler **SQLite**-Datenbank — komplett ohne Browser, Node.js, TypeScript oder WebView.

## Features

- **100 % reine Rust-Desktop-App:** Kompiliert als eigenständige `openlauncher.exe` (`eframe`/`egui`) mit randlosem Liquid-Glass-Fenster, eigener Titelleiste, Resize-Zonen und System-Tray-Unterstützung.
- **Steam Auto-Scan (ohne API-Key):** Liest `libraryfolders.vdf` und `appmanifest_*.acf` unter Windows (Registry + Standardpfade) und Linux automatisch aus und startet Steam-Spiele direkt über die lokale `steam.exe` (`-applaunch <appid>`).
- **PC-Stores, Ordner-Scan & Emulatoren:** Automatische Erkennung für Steam, Epic Games, GOG Galaxy, lokale Spieleordner (inkl. Unreal-Engine-Erkennung) sowie 11 vorkonfigurierte Emulator-Presets (Cemu mit `settings.xml` & `meta.xml`, Dolphin, RPCS3, PCSX2, Ryujinx, Yuzu/Eden, PPSSPP, DuckStation, Citra/Azahar, mGBA, Eigener Emulator).
- **Hintergrund-Playtime-Tracking:** Überwacht gestartete Prozesse und Kindprozesse sicher ohne Shell-Aufrufe und speichert Gesamtspielzeit, „Zuletzt gespielt“ sowie einzelne Sessions in SQLite.
- **Liquid Glass UI:** Tiefschwarzer Obsidian-Hintergrund (`#0a0a0c`), frosted Charcoal-Glass-Panels, silberweiße Highlights, `Ctrl+K`-Command-Palette mit Fuzzy-Suche und Detail-Ansicht mit Session-Verlauf.

## Voraussetzungen

- **Rust** Toolchain (`rustup`, `cargo`, `clippy`)
- **Windows:** Microsoft Visual Studio C++ Build Tools (MSVC)

## Bauen, testen & starten

```powershell
# Unit-Tests (SQLite, VDF/ACF-Parser, Discovery/Cemu, Launcher, Covers, Format, Fuzzy)
cargo test

# Rust Linter (ohne Warnungen)
cargo clippy --all-targets -- -D warnings

# Als native App starten
cargo run

# Eigenständige Release-.exe bauen (target\release\openlauncher.exe)
cargo build --release
```
