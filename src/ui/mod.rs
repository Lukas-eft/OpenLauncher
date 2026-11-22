use std::time::Instant;

use crate::models::{AppSettings, Game, NewGameInput, Platform, UpdateGameInput};

pub mod command_palette;
pub mod detail_modal;
pub mod game_form_modal;
pub mod game_grid;
pub mod icons;
pub mod settings_modal;
pub mod sidebar;
pub mod titlebar;
pub mod toasts;
pub mod topbar;
pub mod widgets;

/// Aktiver Bibliotheksfilter in Sidebar und TopBar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarFilter {
    All,
    Favorites,
    Recent,
    Platform(Platform),
    Tag(String),
}

impl SidebarFilter {
    pub fn title(&self) -> String {
        match self {
            Self::All => "All Games".to_string(),
            Self::Favorites => "Favorites".to_string(),
            Self::Recent => "Recently Played".to_string(),
            Self::Platform(platform) => crate::format::platform_label(*platform).to_string(),
            Self::Tag(tag) => format!("# {tag}"),
        }
    }
}

/// Sort option for the game grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOption {
    TitleAsc,
    PlaytimeDesc,
    LastPlayedDesc,
    AddedDesc,
}

impl SortOption {
    pub const ALL: [Self; 4] = [
        Self::TitleAsc,
        Self::PlaytimeDesc,
        Self::LastPlayedDesc,
        Self::AddedDesc,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::TitleAsc => "Name (A–Z)",
            Self::PlaytimeDesc => "Playtime",
            Self::LastPlayedDesc => "Last Played",
            Self::AddedDesc => "Recently Added",
        }
    }
}

/// Tabs im Dialog zum Hinzufügen eines Spiels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddModalTab {
    Emulator,
    Discover,
    Manual,
}

/// Darstellungsvariante für Toast-Benachrichtigungen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastVariant {
    Success,
    Info,
    Error,
}

/// Einzelne schwebende Toast-Meldung.
#[derive(Debug, Clone)]
pub struct ToastItem {
    pub id: u64,
    pub variant: ToastVariant,
    pub title: String,
    pub description: Option<String>,
    pub created_at: Instant,
}

/// Aktionen, die von UI-Komponenten an `OpenLauncherApp` gemeldet werden.
#[derive(Debug, Clone)]
pub enum AppAction {
    SelectFilter(SidebarFilter),
    SetSort(SortOption),
    ScanSteam,
    OpenAddModal(Option<AddModalTab>),
    OpenEditModal(Game),
    OpenDetailModal(String),
    CloseDetailModal,
    OpenSettings,
    CloseSettings,
    TogglePalette,
    ClosePalette,
    LaunchGame(String),
    ToggleFavorite(String),
    DeleteGame(String),
    SaveNewGame(NewGameInput),
    SaveUpdatedGame(UpdateGameInput),
    BulkImportGames(Vec<NewGameInput>),
    SaveSettings(AppSettings),
    DismissToast(u64),
    LoadEmulatorPresets,
    ScanEmulatorFolder {
        preset_id: String,
        emulator_path: Option<String>,
        rom_dir: String,
        fullscreen: bool,
    },
    ScanPcStores,
    ScanPcFolder(String),
    PickEmulatorBinary,
    PickEmulatorRomDir {
        preset_name: String,
    },
    PickSingleRom {
        preset_name: String,
        extensions: Vec<String>,
    },
    PickPcGamesFolder,
    PickExecutableForInspect,
    PickWorkingDir,
    PickLocalCover,
    PickSteamDir,
}
