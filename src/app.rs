use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Frame, Margin, ViewportCommand};
use rfd::FileDialog;

use crate::covers::{copy_cover_to_dir, ensure_managed_cover, remove_cover_if_managed};
use crate::db::{Database, UpsertOutcome};
use crate::discovery;
use crate::error::AppResult;
use crate::format::format_duration_exact;
use crate::fuzzy::parse_rfc3339_millis;
use crate::launcher::{PlaytimeTracker, TrackerEvent};
use crate::models::{
    ActiveGameStatus, AppSettings, DiscoveredGame, EmulatorPresetInfo, Game, PlaySession,
    SteamScanResult,
};
use crate::steam;
use crate::theme;
use crate::tray::{self, TrayCommand, TrayHandle};
use crate::ui::command_palette::CommandPaletteState;
use crate::ui::detail_modal::DetailModalState;
use crate::ui::game_form_modal::GameFormState;
use crate::ui::settings_modal::SettingsModalState;
use crate::ui::{
    command_palette, detail_modal, game_form_modal, game_grid, settings_modal, sidebar, titlebar,
    toasts, topbar, widgets, AddModalTab, AppAction, SidebarFilter, SortOption, ToastItem,
    ToastVariant,
};

enum AppMessage {
    SteamScanned {
        result: AppResult<SteamScanResult>,
        silent_if_empty: bool,
    },
    PresetsLoaded(Vec<EmulatorPresetInfo>),
    EmulatorFolderScanned {
        preset_name: String,
        result: AppResult<Vec<DiscoveredGame>>,
    },
    PcStoresScanned(Vec<DiscoveredGame>),
    PcFolderScanned(AppResult<Vec<DiscoveredGame>>),
    SessionsLoaded {
        game_id: String,
        sessions: Vec<PlaySession>,
    },
}

#[derive(Debug, Clone)]
enum PendingDialog {
    EmulatorBinary,
    EmulatorRomDir {
        preset_name: String,
    },
    SingleRom {
        preset_name: String,
        extensions: Vec<String>,
    },
    PcGamesFolder,
    ExecutableForInspect,
    WorkingDir,
    LocalCover,
    SteamDir,
}

/// Zentraler Zustand und UI-Controller der reinen Rust-Desktop-App OpenLauncher.
pub struct OpenLauncherApp {
    db: Arc<Database>,
    tracker: Arc<PlaytimeTracker>,
    covers_dir: PathBuf,
    _tray_handle: Option<TrayHandle>,
    force_quit: Arc<AtomicBool>,

    app_tx: Sender<AppMessage>,
    app_rx: Receiver<AppMessage>,
    tracker_tx: Sender<TrackerEvent>,
    tracker_rx: Receiver<TrackerEvent>,
    tray_rx: Receiver<TrayCommand>,

    games: Vec<Game>,
    tags: Vec<String>,
    settings: AppSettings,
    running_games: HashMap<String, ActiveGameStatus>,
    filter: SidebarFilter,
    sort: SortOption,
    is_scanning_steam: bool,

    detail_modal: Option<DetailModalState>,
    form_modal: Option<GameFormState>,
    settings_modal: Option<SettingsModalState>,
    command_palette: CommandPaletteState,
    toasts: Vec<ToastItem>,
    next_toast_id: u64,
    pending_dialog: Option<PendingDialog>,
}

impl OpenLauncherApp {
    /// Initialisiert Datenbank, Playtime-Tracker, Tray-Icon und Bibliotheksdaten.
    pub fn new(cc: &eframe::CreationContext<'_>, win32_hwnd: Option<isize>) -> AppResult<Self> {
        let app_data_dir = crate::covers::resolve_app_data_dir()?;
        let db_path = app_data_dir.join("openlauncher.sqlite");
        let covers_dir = app_data_dir.join("covers");

        let db = Arc::new(Database::open(&db_path)?);
        let tracker = Arc::new(PlaytimeTracker::new());
        let settings = db.get_settings().unwrap_or_default();
        let games = db.list_games().unwrap_or_default();
        let tags = db.list_all_tags().unwrap_or_default();

        let (app_tx, app_rx) = mpsc::channel();
        let (tracker_tx, tracker_rx) = mpsc::channel();
        let (tray_tx, tray_rx) = mpsc::channel();
        let force_quit = Arc::new(AtomicBool::new(false));

        let tray_handle = tray::setup_tray(
            cc.egui_ctx.clone(),
            tray_tx,
            Arc::clone(&force_quit),
            win32_hwnd,
        )
        .ok();

        let mut app = Self {
            db,
            tracker,
            covers_dir,
            _tray_handle: tray_handle,
            force_quit,
            app_tx,
            app_rx,
            tracker_tx,
            tracker_rx,
            tray_rx,
            games,
            tags,
            settings,
            running_games: HashMap::new(),
            filter: SidebarFilter::All,
            sort: SortOption::TitleAsc,
            is_scanning_steam: false,
            detail_modal: None,
            form_modal: None,
            settings_modal: None,
            command_palette: CommandPaletteState::default(),
            toasts: Vec::new(),
            next_toast_id: 1,
            pending_dialog: None,
        };

        if app.settings.auto_scan_steam_on_startup {
            app.start_steam_scan(true, &cc.egui_ctx);
        }

        Ok(app)
    }

    fn push_toast(&mut self, variant: ToastVariant, title: impl Into<String>, desc: Option<String>) {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        if self.toasts.len() >= 4 {
            let overflow = self.toasts.len() - 3;
            self.toasts.drain(0..overflow);
        }
        self.toasts.push(ToastItem {
            id,
            variant,
            title: title.into(),
            description: desc,
            created_at: Instant::now(),
        });
    }

    fn refresh_library(&mut self) {
        match self.db.list_games() {
            Ok(games) => self.games = games,
            Err(err) => self.push_toast(
                ToastVariant::Error,
                "Bibliothek konnte nicht geladen werden",
                Some(err.to_string()),
            ),
        }
        if let Ok(tags) = self.db.list_all_tags() {
            self.tags = tags;
        }
        let active = self.tracker.running_games();
        self.running_games.clear();
        for status in active {
            self.running_games.insert(status.game_id.clone(), status);
        }
    }

    fn start_steam_scan(&mut self, silent_if_empty: bool, ctx: &egui::Context) {
        if self.is_scanning_steam {
            return;
        }
        self.is_scanning_steam = true;
        let db = Arc::clone(&self.db);
        let tx = self.app_tx.clone();
        let repaint = ctx.clone();

        thread::spawn(move || {
            let result = (|| -> AppResult<SteamScanResult> {
                let settings = db.get_settings()?;
                let custom_path = settings.custom_steam_path.as_deref().map(Path::new);
                let scan = steam::scan_steam_libraries(custom_path);

                let mut added_games = 0usize;
                let mut updated_games = 0usize;
                let mut warnings = scan.warnings;

                for app in &scan.apps {
                    match db.upsert_steam_game(app) {
                        Ok(UpsertOutcome::Inserted) => added_games += 1,
                        Ok(UpsertOutcome::Updated) => updated_games += 1,
                        Err(err) => warnings.push(format!("{} ({}): {err}", app.name, app.app_id)),
                    }
                }

                Ok(SteamScanResult {
                    scanned_libraries: scan.scanned_libraries,
                    found_games: scan.apps.len(),
                    added_games,
                    updated_games,
                    warnings,
                })
            })();

            let _ = tx.send(AppMessage::SteamScanned {
                result,
                silent_if_empty,
            });
            repaint.request_repaint();
        });
    }

    fn load_sessions_async(&self, game_id: String, ctx: &egui::Context) {
        let db = Arc::clone(&self.db);
        let tx = self.app_tx.clone();
        let repaint = ctx.clone();
        thread::spawn(move || {
            let sessions = db.get_game_sessions(&game_id, 25).unwrap_or_default();
            let _ = tx.send(AppMessage::SessionsLoaded { game_id, sessions });
            repaint.request_repaint();
        });
    }

    fn drain_background_events(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.tray_rx.try_recv() {
            match cmd {
                TrayCommand::Show => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                TrayCommand::Quit => {
                    self.force_quit.store(true, Ordering::SeqCst);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        while let Ok(evt) = self.tracker_rx.try_recv() {
            match evt {
                TrackerEvent::Started(status) => {
                    self.running_games.insert(status.game_id.clone(), status);
                }
                TrackerEvent::Stopped(event) => {
                    self.running_games.remove(&event.game_id);
                    if let Some(game) = self.games.iter_mut().find(|g| g.id == event.game_id) {
                        game.playtime_seconds = event.total_playtime_seconds;
                        game.last_played_at = Some(event.last_played_at.clone());
                    }
                    if let Some(ref detail) = self.detail_modal {
                        if detail.game_id == event.game_id {
                            self.load_sessions_async(event.game_id.clone(), ctx);
                        }
                    }
                    if event.duration_seconds > 0 {
                        self.push_toast(
                            ToastVariant::Info,
                            "Session gespeichert",
                            Some(format!(
                                "Spielzeit aufgezeichnet: {}",
                                format_duration_exact(event.duration_seconds)
                            )),
                        );
                    }
                }
            }
        }

        while let Ok(msg) = self.app_rx.try_recv() {
            match msg {
                AppMessage::SteamScanned {
                    result,
                    silent_if_empty,
                } => {
                    self.is_scanning_steam = false;
                    match result {
                        Ok(res) => {
                            self.refresh_library();
                            if !silent_if_empty
                                || res.added_games > 0
                                || !res.warnings.is_empty()
                            {
                                let variant =
                                    if res.found_games == 0 && !res.warnings.is_empty() {
                                        ToastVariant::Info
                                    } else {
                                        ToastVariant::Success
                                    };
                                let suffix = if res.scanned_libraries == 1 { "" } else { "en" };
                                self.push_toast(
                                    variant,
                                    "Steam-Scan abgeschlossen",
                                    Some(format!(
                                        "{} neu hinzugefügt, {} aktualisiert ({} Spiele in {} Bibliothek{suffix}).",
                                        res.added_games,
                                        res.updated_games,
                                        res.found_games,
                                        res.scanned_libraries
                                    )),
                                );
                            }
                        }
                        Err(err) => {
                            self.push_toast(
                                ToastVariant::Error,
                                "Steam-Scan fehlgeschlagen",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
                AppMessage::PresetsLoaded(presets) => {
                    if let Some(ref mut form) = self.form_modal {
                        form.apply_loaded_presets(presets);
                    }
                }
                AppMessage::EmulatorFolderScanned {
                    preset_name,
                    result,
                } => {
                    if let Some(ref mut form) = self.form_modal {
                        match result {
                            Ok(items) => form.set_emu_results(items, &preset_name),
                            Err(err) => {
                                form.emu_scan_busy = false;
                                form.error = Some(err.to_string());
                            }
                        }
                    }
                }
                AppMessage::PcStoresScanned(items) => {
                    if let Some(ref mut form) = self.form_modal {
                        form.set_discover_results(
                            items,
                            "Keine installierten Steam-, Epic- oder GOG-Spiele gefunden.",
                        );
                    }
                }
                AppMessage::PcFolderScanned(result) => {
                    if let Some(ref mut form) = self.form_modal {
                        match result {
                            Ok(items) => form.set_discover_results(
                                items,
                                "In diesem Ordner wurden keine Spiele gefunden.",
                            ),
                            Err(err) => {
                                form.discover_busy = false;
                                form.error = Some(err.to_string());
                            }
                        }
                    }
                }
                AppMessage::SessionsLoaded { game_id, sessions } => {
                    if let Some(ref mut detail) = self.detail_modal {
                        if detail.game_id == game_id {
                            detail.sessions = sessions;
                            detail.loading_sessions = false;
                        }
                    }
                }
            }
        }
    }

    fn process_actions(&mut self, actions: Vec<AppAction>, ctx: &egui::Context) {
        for action in actions {
            match action {
                AppAction::SelectFilter(next) => {
                    if next == SidebarFilter::Recent {
                        self.sort = SortOption::LastPlayedDesc;
                    }
                    self.filter = next;
                }
                AppAction::SetSort(sort) => {
                    self.sort = sort;
                }
                AppAction::ScanSteam => {
                    self.start_steam_scan(false, ctx);
                }
                AppAction::OpenAddModal(tab) => {
                    let fallback_tab = match self.filter {
                        SidebarFilter::Platform(crate::models::Platform::Manual) => {
                            AddModalTab::Manual
                        }
                        SidebarFilter::Platform(crate::models::Platform::Emulator) => {
                            AddModalTab::Emulator
                        }
                        _ => AddModalTab::Discover,
                    };
                    self.form_modal = Some(GameFormState::new_add(tab.unwrap_or(fallback_tab)));
                    self.spawn_load_presets(ctx);
                }
                AppAction::OpenEditModal(game) => {
                    self.form_modal = Some(GameFormState::new_edit(&game));
                }
                AppAction::OpenDetailModal(game_id) => {
                    self.detail_modal = Some(DetailModalState::new(game_id.clone()));
                    self.load_sessions_async(game_id, ctx);
                }
                AppAction::CloseDetailModal => {
                    self.detail_modal = None;
                }
                AppAction::OpenSettings => {
                    self.settings_modal = Some(SettingsModalState::from(&self.settings));
                }
                AppAction::CloseSettings => {
                    self.settings_modal = None;
                }
                AppAction::TogglePalette => {
                    if self.command_palette.is_open {
                        self.command_palette.close();
                    } else {
                        self.command_palette.open();
                    }
                }
                AppAction::ClosePalette => {
                    self.command_palette.close();
                }
                AppAction::LaunchGame(game_id) => {
                    self.handle_launch_game(&game_id, ctx);
                }
                AppAction::ToggleFavorite(game_id) => {
                    match self.db.toggle_favorite(&game_id) {
                        Ok(updated) => {
                            if let Some(slot) = self.games.iter_mut().find(|g| g.id == game_id) {
                                *slot = updated;
                            }
                        }
                        Err(err) => {
                            self.push_toast(
                                ToastVariant::Error,
                                "Favorit konnte nicht aktualisiert werden",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
                AppAction::DeleteGame(game_id) => {
                    self.handle_delete_game(&game_id);
                }
                AppAction::SaveNewGame(mut input) => {
                    ensure_managed_cover(
                        &mut input.cover_url,
                        &mut input.cover_is_local,
                        &self.covers_dir,
                    );
                    match self.db.create_game(input) {
                        Ok(created) => {
                            self.refresh_library();
                            self.form_modal = None;
                            self.push_toast(
                                ToastVariant::Success,
                                "Spiel hinzugefügt",
                                Some(created.title),
                            );
                        }
                        Err(err) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.is_submitting = false;
                                form.error = Some(err.to_string());
                            }
                            self.push_toast(
                                ToastVariant::Error,
                                "Spiel konnte nicht erstellt werden",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
                AppAction::SaveUpdatedGame(mut input) => {
                    ensure_managed_cover(
                        &mut input.cover_url,
                        &mut input.cover_is_local,
                        &self.covers_dir,
                    );
                    match self.db.update_game(input) {
                        Ok(updated) => {
                            self.refresh_library();
                            self.form_modal = None;
                            self.push_toast(
                                ToastVariant::Success,
                                "Änderungen gespeichert",
                                Some(updated.title),
                            );
                        }
                        Err(err) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.is_submitting = false;
                                form.error = Some(err.to_string());
                            }
                            self.push_toast(
                                ToastVariant::Error,
                                "Änderungen konnten nicht gespeichert werden",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
                AppAction::BulkImportGames(mut items) => {
                    for item in &mut items {
                        ensure_managed_cover(
                            &mut item.cover_url,
                            &mut item.cover_is_local,
                            &self.covers_dir,
                        );
                    }
                    match self.db.import_games_bulk(items) {
                        Ok(res) => {
                            self.refresh_library();
                            let skip_info = if res.skipped_count > 0 {
                                format!(", {} bereits vorhanden", res.skipped_count)
                            } else {
                                String::new()
                            };
                            let desc = if !res.errors.is_empty() {
                                Some(format!("{} Fehler{skip_info}", res.errors.len()))
                            } else if !skip_info.is_empty() {
                                Some(skip_info.trim_start_matches(", ").to_string())
                            } else {
                                None
                            };
                            self.push_toast(
                                if res.added_count > 0 {
                                    ToastVariant::Success
                                } else {
                                    ToastVariant::Info
                                },
                                format!(
                                    "{} {} importiert",
                                    res.added_count,
                                    if res.added_count == 1 {
                                        "Spiel"
                                    } else {
                                        "Spiele"
                                    }
                                ),
                                desc,
                            );
                            if res.added_count > 0 {
                                self.form_modal = None;
                            } else if let Some(ref mut form) = self.form_modal {
                                form.is_submitting = false;
                            }
                        }
                        Err(err) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.is_submitting = false;
                                form.error = Some(err.to_string());
                            }
                            self.push_toast(
                                ToastVariant::Error,
                                "Import fehlgeschlagen",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
                AppAction::SaveSettings(next) => {
                    match self.db.save_settings(&next).and_then(|()| self.db.get_settings()) {
                        Ok(saved) => {
                            self.settings = saved;
                            self.settings_modal = None;
                            self.push_toast(ToastVariant::Success, "Einstellungen gespeichert", None);
                        }
                        Err(err) => {
                            if let Some(ref mut modal) = self.settings_modal {
                                modal.is_saving = false;
                            }
                            self.push_toast(
                                ToastVariant::Error,
                                "Einstellungen konnten nicht gespeichert werden",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
                AppAction::DismissToast(id) => {
                    self.toasts.retain(|t| t.id != id);
                }
                AppAction::LoadEmulatorPresets => {
                    self.spawn_load_presets(ctx);
                }
                AppAction::ScanEmulatorFolder {
                    preset_id,
                    emulator_path,
                    rom_dir,
                    fullscreen,
                } => {
                    let preset_name = self
                        .form_modal
                        .as_ref()
                        .and_then(|f| f.selected_preset())
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| preset_id.clone());
                    self.spawn_scan_emulator_folder(
                        preset_id,
                        preset_name,
                        emulator_path,
                        rom_dir,
                        fullscreen,
                        ctx,
                    );
                }
                AppAction::ScanPcStores => {
                    self.spawn_scan_pc_stores(ctx);
                }
                AppAction::ScanPcFolder(folder) => {
                    self.spawn_scan_pc_folder(folder, ctx);
                }
                AppAction::PickEmulatorBinary => {
                    self.pending_dialog = Some(PendingDialog::EmulatorBinary);
                }
                AppAction::PickEmulatorRomDir { preset_name } => {
                    self.pending_dialog = Some(PendingDialog::EmulatorRomDir { preset_name });
                }
                AppAction::PickSingleRom {
                    preset_name,
                    extensions,
                } => {
                    self.pending_dialog = Some(PendingDialog::SingleRom {
                        preset_name,
                        extensions,
                    });
                }
                AppAction::PickPcGamesFolder => {
                    self.pending_dialog = Some(PendingDialog::PcGamesFolder);
                }
                AppAction::PickExecutableForInspect => {
                    self.pending_dialog = Some(PendingDialog::ExecutableForInspect);
                }
                AppAction::PickWorkingDir => {
                    self.pending_dialog = Some(PendingDialog::WorkingDir);
                }
                AppAction::PickLocalCover => {
                    self.pending_dialog = Some(PendingDialog::LocalCover);
                }
                AppAction::PickSteamDir => {
                    self.pending_dialog = Some(PendingDialog::SteamDir);
                }
            }
        }
    }

    fn handle_launch_game(&mut self, game_id: &str, ctx: &egui::Context) {
        let game = match self.db.get_game(game_id) {
            Ok(g) => g,
            Err(err) => {
                self.push_toast(
                    ToastVariant::Error,
                    "Spiel konnte nicht gestartet werden",
                    Some(err.to_string()),
                );
                return;
            }
        };
        let title = game.title.clone();
        match self.tracker.launch_game(
            game,
            Arc::clone(&self.db),
            self.tracker_tx.clone(),
            Some(ctx.clone()),
        ) {
            Ok(status) => {
                self.running_games.insert(status.game_id.clone(), status);
                self.push_toast(
                    ToastVariant::Success,
                    format!("{title} wird gestartet"),
                    Some("Playtime-Tracking ist aktiv.".to_string()),
                );
            }
            Err(err) => {
                self.push_toast(
                    ToastVariant::Error,
                    format!("{title} konnte nicht gestartet werden"),
                    Some(err.to_string()),
                );
            }
        }
    }

    fn handle_delete_game(&mut self, game_id: &str) {
        if self.tracker.is_running(game_id) {
            self.push_toast(
                ToastVariant::Error,
                "Löschen fehlgeschlagen",
                Some("Ein aktuell laufendes Spiel kann nicht gelöscht werden".to_string()),
            );
            return;
        }
        let existing = match self.db.get_game(game_id) {
            Ok(g) => g,
            Err(err) => {
                self.push_toast(
                    ToastVariant::Error,
                    "Löschen fehlgeschlagen",
                    Some(err.to_string()),
                );
                return;
            }
        };
        if let Err(err) = self.db.delete_game(game_id) {
            self.push_toast(
                ToastVariant::Error,
                "Löschen fehlgeschlagen",
                Some(err.to_string()),
            );
            return;
        }
        if existing.cover_is_local {
            if let Some(ref cover_path) = existing.cover_url {
                remove_cover_if_managed(Path::new(cover_path), &self.covers_dir);
            }
        }
        self.detail_modal = None;
        self.refresh_library();
        self.push_toast(ToastVariant::Info, "Spiel entfernt", Some(existing.title));
    }

    fn spawn_load_presets(&self, ctx: &egui::Context) {
        let tx = self.app_tx.clone();
        let repaint = ctx.clone();
        thread::spawn(move || {
            let presets = discovery::list_emulator_presets();
            let _ = tx.send(AppMessage::PresetsLoaded(presets));
            repaint.request_repaint();
        });
    }

    fn spawn_scan_emulator_folder(
        &self,
        preset_id: String,
        preset_name: String,
        emulator_path: Option<String>,
        rom_dir: String,
        fullscreen: bool,
        ctx: &egui::Context,
    ) {
        let db = Arc::clone(&self.db);
        let tx = self.app_tx.clone();
        let repaint = ctx.clone();
        thread::spawn(move || {
            let result = db.find_existing_keys().and_then(|existing| {
                discovery::scan_emulator_folder(
                    &preset_id,
                    emulator_path.as_deref(),
                    Path::new(rom_dir.trim()),
                    fullscreen,
                    &existing,
                )
            });
            let _ = tx.send(AppMessage::EmulatorFolderScanned {
                preset_name,
                result,
            });
            repaint.request_repaint();
        });
    }

    fn spawn_scan_pc_stores(&self, ctx: &egui::Context) {
        let db = Arc::clone(&self.db);
        let tx = self.app_tx.clone();
        let repaint = ctx.clone();
        thread::spawn(move || {
            let items = match (db.get_settings(), db.find_existing_keys()) {
                (Ok(settings), Ok(existing)) => {
                    let custom = settings.custom_steam_path.as_deref().map(Path::new);
                    discovery::scan_pc_stores(custom, &existing)
                }
                _ => Vec::new(),
            };
            let _ = tx.send(AppMessage::PcStoresScanned(items));
            repaint.request_repaint();
        });
    }

    fn spawn_scan_pc_folder(&self, folder: String, ctx: &egui::Context) {
        let db = Arc::clone(&self.db);
        let tx = self.app_tx.clone();
        let repaint = ctx.clone();
        thread::spawn(move || {
            let result = db
                .find_existing_keys()
                .and_then(|existing| discovery::scan_pc_games_folder(Path::new(folder.trim()), &existing));
            let _ = tx.send(AppMessage::PcFolderScanned(result));
            repaint.request_repaint();
        });
    }

    fn execute_pending_dialog(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let Some(dialog) = self.pending_dialog.take() else {
            return;
        };

        match dialog {
            PendingDialog::EmulatorBinary => {
                let mut fd = FileDialog::new()
                    .set_parent(frame)
                    .set_title("Emulator-Programmdatei auswählen");
                if cfg!(target_os = "windows") {
                    fd = fd.add_filter("Programme", &["exe"]);
                }
                if let Some(path) = fd.pick_file() {
                    if let Some(ref mut form) = self.form_modal {
                        form.emu_exe_path = path.to_string_lossy().into_owned();
                        form.error = None;
                    }
                }
            }
            PendingDialog::EmulatorRomDir { preset_name } => {
                if let Some(dir) = FileDialog::new()
                    .set_parent(frame)
                    .set_title(format!("{preset_name}-Spieleordner auswählen"))
                    .pick_folder()
                {
                    if let Some(ref mut form) = self.form_modal {
                        let preset_id = form
                            .selected_preset()
                            .map(|p| p.id.clone())
                            .unwrap_or_else(|| "cemu".to_string());
                        let emu_exe = {
                            let t = form.emu_exe_path.trim();
                            if t.is_empty() {
                                None
                            } else {
                                Some(t.to_string())
                            }
                        };
                        let fullscreen = form.emu_fullscreen;
                        form.emu_scan_busy = true;
                        form.error = None;
                        self.spawn_scan_emulator_folder(
                            preset_id,
                            preset_name,
                            emu_exe,
                            dir.to_string_lossy().into_owned(),
                            fullscreen,
                            ctx,
                        );
                    }
                }
            }
            PendingDialog::SingleRom {
                preset_name,
                extensions,
            } => {
                let mut fd = FileDialog::new()
                    .set_parent(frame)
                    .set_title(format!("{preset_name}-Spiel auswählen"));
                if !extensions.is_empty() {
                    fd = fd.add_filter(format!("{preset_name}-Spiele"), &extensions);
                }
                if let Some(rom_path) = fd.pick_file() {
                    match discovery::inspect_game_file(&rom_path) {
                        Ok(info) => {
                            if let Some(ref mut form) = self.form_modal {
                                let rom_str = rom_path.to_string_lossy().into_owned();
                                form.apply_single_rom(&rom_str, info, &self.covers_dir);
                            }
                        }
                        Err(err) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.error = Some(err.to_string());
                            }
                        }
                    }
                }
            }
            PendingDialog::PcGamesFolder => {
                if let Some(dir) = FileDialog::new()
                    .set_parent(frame)
                    .set_title(r"Spiele-Ordner auswählen (z. B. D:\Games)")
                    .pick_folder()
                {
                    if let Some(ref mut form) = self.form_modal {
                        form.discover_busy = true;
                        form.error = None;
                    }
                    self.spawn_scan_pc_folder(dir.to_string_lossy().into_owned(), ctx);
                }
            }
            PendingDialog::ExecutableForInspect => {
                let fd = FileDialog::new()
                    .set_parent(frame)
                    .set_title("Spiel-Datei oder Emulator auswählen")
                    .add_filter(
                        "Spiele & Starter",
                        &[
                            "exe", "desktop", "appimage", "wua", "wux", "wud", "rpx", "nsp",
                            "xci", "rvz", "iso", "chd", "3ds", "gba", "bin", "pkg",
                        ],
                    );
                if let Some(file_path) = fd.pick_file() {
                    match discovery::inspect_game_file(&file_path) {
                        Ok(info) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.apply_inspected_file(info, &self.covers_dir);
                            }
                        }
                        Err(err) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.error = Some(err.to_string());
                            }
                        }
                    }
                }
            }
            PendingDialog::WorkingDir => {
                if let Some(dir) = FileDialog::new()
                    .set_parent(frame)
                    .set_title("Arbeitsverzeichnis auswählen")
                    .pick_folder()
                {
                    if let Some(ref mut form) = self.form_modal {
                        form.working_dir = dir.to_string_lossy().into_owned();
                    }
                }
            }
            PendingDialog::LocalCover => {
                let fd = FileDialog::new()
                    .set_parent(frame)
                    .set_title("Cover-Bild auswählen")
                    .add_filter("Bilder", &["png", "jpg", "jpeg", "webp", "gif", "avif"]);
                if let Some(img_path) = fd.pick_file() {
                    match copy_cover_to_dir(&img_path, &self.covers_dir) {
                        Ok(stored) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.cover_url = stored;
                                form.cover_is_local = true;
                                form.error = None;
                            }
                        }
                        Err(err) => {
                            if let Some(ref mut form) = self.form_modal {
                                form.error = Some(err.to_string());
                            }
                        }
                    }
                }
            }
            PendingDialog::SteamDir => {
                if let Some(dir) = FileDialog::new()
                    .set_parent(frame)
                    .set_title("Steam-Installationsordner auswählen")
                    .pick_folder()
                {
                    if let Some(ref mut settings_modal) = self.settings_modal {
                        settings_modal.custom_steam_path = dir.to_string_lossy().into_owned();
                    }
                }
            }
        }
    }

    fn filtered_and_sorted_games(&self) -> Vec<&Game> {
        let mut filtered: Vec<&Game> = self
            .games
            .iter()
            .filter(|game| match &self.filter {
                SidebarFilter::All => true,
                SidebarFilter::Favorites => game.is_favorite,
                SidebarFilter::Recent => game.last_played_at.is_some(),
                SidebarFilter::Platform(p) => game.platform == *p,
                SidebarFilter::Tag(tag) => game.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)),
            })
            .collect();

        filtered.sort_by(|a, b| match self.sort {
            SortOption::TitleAsc => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            SortOption::PlaytimeDesc => b
                .playtime_seconds
                .cmp(&a.playtime_seconds)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            SortOption::LastPlayedDesc => {
                let a_ts = parse_rfc3339_millis(a.last_played_at.as_deref()).unwrap_or(-1);
                let b_ts = parse_rfc3339_millis(b.last_played_at.as_deref()).unwrap_or(-1);
                b_ts.cmp(&a_ts)
                    .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            }
            SortOption::AddedDesc => {
                let a_ts = parse_rfc3339_millis(Some(&a.added_at)).unwrap_or(0);
                let b_ts = parse_rfc3339_millis(Some(&b.added_at)).unwrap_or(0);
                b_ts.cmp(&a_ts)
                    .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            }
        });

        filtered
    }
}

impl eframe::App for OpenLauncherApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.drain_background_events(ctx);

        // Schließen in den System-Tray abfangen, falls aktiviert
        if ctx.input(|i| i.viewport().close_requested())
            && self.settings.minimize_to_tray
            && !self.force_quit.load(Ordering::SeqCst)
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }

        // Globaler Shortcut Ctrl+K für die Command-Palette
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            if self.command_palette.is_open {
                self.command_palette.close();
            } else {
                self.command_palette.open();
            }
        }

        // Abgelaufene Toasts entfernen
        let now = Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.created_at) < Duration::from_millis(4500));
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        let mut actions = Vec::new();
        let total_playtime_seconds: i64 = self.games.iter().map(|g| g.playtime_seconds).sum();
        let running_list: Vec<ActiveGameStatus> = self.running_games.values().cloned().collect();

        // Randloses Fenster-Resizing an Kanten und Ecken
        widgets::handle_borderless_window_resize(ctx);

        // 1. Titelleiste (ohne "NATIVE APP"-Pill)
        titlebar::show(
            ctx,
            self.games.len(),
            total_playtime_seconds,
            &running_list,
        );

        // 2. Linke Sidebar
        sidebar::show(
            ctx,
            &self.games,
            &self.tags,
            &self.filter,
            self.is_scanning_steam,
            &mut actions,
        );

        // 3. Hauptbereich (TopBar + Spiele-Grid auf Liquid-Glass-Canvas)
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                theme::paint_liquid_canvas_bg(ui.painter(), rect);

                let filtered_games = self.filtered_and_sorted_games();
                topbar::show(
                    ui,
                    &self.filter,
                    filtered_games.len(),
                    self.running_games.len(),
                    self.sort,
                    &mut actions,
                );

                game_grid::show(
                    ui,
                    &filtered_games,
                    self.games.len(),
                    &self.filter,
                    &self.running_games,
                    self.is_scanning_steam,
                    &mut actions,
                );
            });

        // 4. Modals & Overlays
        let top_overlay_active = self.form_modal.is_some()
            || self.settings_modal.is_some()
            || self.command_palette.is_open;

        if let Some(ref mut detail_state) = self.detail_modal {
            if let Some(game) = self.games.iter().find(|g| g.id == detail_state.game_id) {
                let running = self.running_games.get(&game.id);
                detail_modal::show(
                    ctx,
                    detail_state,
                    game,
                    running,
                    !top_overlay_active,
                    &mut actions,
                );
            } else {
                self.detail_modal = None;
            }
        }

        if let Some(ref mut form_state) = self.form_modal {
            game_form_modal::show(ctx, form_state, &self.tags, &mut actions);
            if form_state.close_requested_flag {
                self.form_modal = None;
            }
        }

        if let Some(ref mut settings_state) = self.settings_modal {
            settings_modal::show(ctx, settings_state, self.is_scanning_steam, &mut actions);
        }

        if self.command_palette.is_open {
            command_palette::show(
                ctx,
                &mut self.command_palette,
                &self.games,
                &self.running_games,
                &mut actions,
            );
        }

        toasts::show(ctx, &self.toasts, &mut actions);

        self.process_actions(actions, ctx);
        self.execute_pending_dialog(ctx, frame);
    }
}
