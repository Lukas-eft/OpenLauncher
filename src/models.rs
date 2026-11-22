use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Unterstützte Herkunfts- und Startplattformen eines Spiels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Steam,
    Manual,
    Emulator,
}

impl Platform {
    /// Gibt die kanonische Kleinschreibungs-Repräsentation für SQLite zurück.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Manual => "manual",
            Self::Emulator => "emulator",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Platform {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "steam" => Ok(Self::Steam),
            "manual" => Ok(Self::Manual),
            "emulator" => Ok(Self::Emulator),
            other => Err(AppError::InvalidInput(format!(
                "Unbekannte Plattform: {other}"
            ))),
        }
    }
}

/// Persistierter Spieleintrag inklusive zugeordneter Tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub steam_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub install_dir: Option<String>,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub cover_url: Option<String>,
    pub cover_is_local: bool,
    pub hero_url: Option<String>,
    pub is_favorite: bool,
    pub playtime_seconds: i64,
    pub last_played_at: Option<String>,
    pub added_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

/// Eingabedaten zum manuellen Anlegen eines Spiels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGameInput {
    pub title: String,
    pub platform: Platform,
    pub steam_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub install_dir: Option<String>,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub cover_url: Option<String>,
    pub cover_is_local: bool,
    pub hero_url: Option<String>,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

/// Eingabedaten zum Bearbeiten eines bestehenden Spiels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGameInput {
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub steam_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub install_dir: Option<String>,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub cover_url: Option<String>,
    pub cover_is_local: bool,
    pub hero_url: Option<String>,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

/// Einzelne aufgezeichnete Spielsession in SQLite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaySession {
    pub id: i64,
    pub game_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: i64,
}

/// Persistierte Benutzereinstellungen der Anwendung.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub minimize_to_tray: bool,
    pub custom_steam_path: Option<String>,
    pub auto_scan_steam_on_startup: bool,
}

/// Geparstes Steam-Spiel aus einer `appmanifest_*.acf`-Datei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamApp {
    pub app_id: String,
    pub name: String,
    pub install_dir: PathBuf,
    pub cover_url: String,
    pub hero_url: String,
}

/// Zusammenfassung eines Steam-Bibliotheksscans für das Frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamScanResult {
    pub scanned_libraries: usize,
    pub found_games: usize,
    pub added_games: usize,
    pub updated_games: usize,
    pub warnings: Vec<String>,
}

/// Status eines aktuell überwachten laufenden Spiels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveGameStatus {
    pub game_id: String,
    pub title: String,
    pub started_at: String,
    pub pid: Option<u32>,
}

/// Event-Payload beim Beenden einer überwachten Spielsession.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningGameEvent {
    pub game_id: String,
    pub session_id: i64,
    pub duration_seconds: i64,
    pub total_playtime_seconds: i64,
    pub last_played_at: String,
}

/// Vorschau-Daten aus einer Linux-`.desktop`-Datei für das Formular.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopEntryPreview {
    pub title: String,
    pub executable_path: String,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub icon: Option<String>,
}

/// Vorkonfiguriertes Emulator-Profil inklusive erkannter Pfade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorPresetInfo {
    pub id: String,
    pub name: String,
    pub system: String,
    pub extensions: Vec<String>,
    pub default_args_template: String,
    pub fullscreen_args_template: String,
    pub default_tags: Vec<String>,
    pub detected_exe_path: Option<String>,
    pub detected_rom_dirs: Vec<String>,
}

/// Automatisch gefundenes Spiel aus Emulatoren, Ordnern oder PC-Stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredGame {
    pub title: String,
    pub platform: Platform,
    pub source: String,
    pub steam_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub install_dir: Option<String>,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub cover_path: Option<String>,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    pub tags: Vec<String>,
    pub already_imported: bool,
}

/// Ergebnis einer intelligenten Einzeldatei-Analyse (`.exe`, ROM, `.desktop`, Emulator).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFileInspection {
    pub title: String,
    pub platform: Platform,
    pub detected_emulator_id: Option<String>,
    pub is_emulator_binary: bool,
    pub is_rom_file: bool,
    pub executable_path: Option<String>,
    pub rom_path: Option<String>,
    pub install_dir: Option<String>,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub cover_path: Option<String>,
    pub tags: Vec<String>,
}

/// Ergebnis eines Sammel-Imports erkannter Spiele.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportResult {
    pub added_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}
