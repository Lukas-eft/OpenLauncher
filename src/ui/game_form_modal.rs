use std::path::Path;

use eframe::egui::{
    self, Align2, Color32, CursorIcon, Frame, Id, Margin, Pos2, Rect, ScrollArea, Sense, Stroke,
    StrokeKind, Vec2,
};

use crate::covers::{copy_cover_to_dir, cover_image_uri};
use crate::discovery::build_emulator_launch_args;
use crate::format::platform_label;
use crate::models::{
    DiscoveredGame, EmulatorPresetInfo, Game, GameFileInspection, NewGameInput, Platform,
    UpdateGameInput,
};
use crate::theme::{
    self, BG_GLASS_SURFACE, BG_GLASS_WELL, BG_PILL_ACTIVE, BORDER_ACTIVE, BORDER_GLASS,
    BORDER_HOVER, BORDER_SUBTLE, DANGER_BG, DANGER_BORDER, DANGER_TEXT, EMERALD_DOT, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::{
    glass_icon_button, glass_modal_frame, glass_pill_button, glass_primary_button,
    glass_secondary_button, glass_text_edit, glass_toggle, glass_well_frame, paint_cover_image,
    paint_modal_backdrop, status_badge,
};
use crate::ui::{AddModalTab, AppAction};

/// Zustand für das Hinzufügen- und Bearbeiten-Modal (Emulator, Auto-Erkennung, Manuell).
#[derive(Debug, Clone)]
pub struct GameFormState {
    pub editing_game_id: Option<String>,
    pub editing_hero_url: Option<String>,
    pub active_tab: AddModalTab,
    pub close_requested_flag: bool,

    // Manuell / Edit
    pub title: String,
    pub platform: Platform,
    pub steam_app_id: String,
    pub executable_path: String,
    pub install_dir: String,
    pub launch_args: String,
    pub working_dir: String,
    pub cover_url: String,
    pub cover_is_local: bool,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub tag_draft: String,
    pub smart_hint: Option<String>,
    pub error: Option<String>,
    pub is_submitting: bool,

    // Emulator Tab
    pub presets: Vec<EmulatorPresetInfo>,
    pub presets_loading: bool,
    pub selected_preset_id: String,
    pub emu_exe_path: String,
    pub emu_fullscreen: bool,
    pub emu_scan_busy: bool,
    pub emu_discovered: Vec<DiscoveredGame>,
    pub emu_selected: Vec<bool>,
    pub emu_search: String,

    // Auto-Erkennung Tab
    pub discover_busy: bool,
    pub discover_items: Vec<DiscoveredGame>,
    pub discover_selected: Vec<bool>,
    pub discover_search: String,
}

impl GameFormState {
    pub fn new_add(initial_tab: AddModalTab) -> Self {
        Self {
            editing_game_id: None,
            editing_hero_url: None,
            active_tab: initial_tab,
            close_requested_flag: false,
            title: String::new(),
            platform: Platform::Manual,
            steam_app_id: String::new(),
            executable_path: String::new(),
            install_dir: String::new(),
            launch_args: String::new(),
            working_dir: String::new(),
            cover_url: String::new(),
            cover_is_local: false,
            is_favorite: false,
            tags: Vec::new(),
            tag_draft: String::new(),
            smart_hint: None,
            error: None,
            is_submitting: false,
            presets: Vec::new(),
            presets_loading: true,
            selected_preset_id: "cemu".to_string(),
            emu_exe_path: String::new(),
            emu_fullscreen: true,
            emu_scan_busy: false,
            emu_discovered: Vec::new(),
            emu_selected: Vec::new(),
            emu_search: String::new(),
            discover_busy: false,
            discover_items: Vec::new(),
            discover_selected: Vec::new(),
            discover_search: String::new(),
        }
    }

    pub fn new_edit(game: &Game) -> Self {
        Self {
            editing_game_id: Some(game.id.clone()),
            editing_hero_url: game.hero_url.clone(),
            active_tab: AddModalTab::Manual,
            close_requested_flag: false,
            title: game.title.clone(),
            platform: game.platform,
            steam_app_id: game.steam_app_id.clone().unwrap_or_default(),
            executable_path: game.executable_path.clone().unwrap_or_default(),
            install_dir: game.install_dir.clone().unwrap_or_default(),
            launch_args: game.launch_args.clone().unwrap_or_default(),
            working_dir: game.working_dir.clone().unwrap_or_default(),
            cover_url: game.cover_url.clone().unwrap_or_default(),
            cover_is_local: game.cover_is_local,
            is_favorite: game.is_favorite,
            tags: game.tags.clone(),
            tag_draft: String::new(),
            smart_hint: None,
            error: None,
            is_submitting: false,
            presets: Vec::new(),
            presets_loading: false,
            selected_preset_id: "cemu".to_string(),
            emu_exe_path: String::new(),
            emu_fullscreen: true,
            emu_scan_busy: false,
            emu_discovered: Vec::new(),
            emu_selected: Vec::new(),
            emu_search: String::new(),
            discover_busy: false,
            discover_items: Vec::new(),
            discover_selected: Vec::new(),
            discover_search: String::new(),
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing_game_id.is_some()
    }

    pub fn selected_preset(&self) -> Option<&EmulatorPresetInfo> {
        self.presets
            .iter()
            .find(|p| p.id == self.selected_preset_id)
            .or_else(|| self.presets.first())
    }

    pub fn apply_loaded_presets(&mut self, loaded: Vec<EmulatorPresetInfo>) {
        self.presets_loading = false;
        if loaded.is_empty() {
            return;
        }
        let cemu = loaded.iter().find(|p| p.id == "cemu");
        let first_detected = loaded.iter().find(|p| p.detected_exe_path.is_some());
        let initial = cemu
            .filter(|p| p.detected_exe_path.is_some())
            .or(first_detected)
            .or(cemu)
            .unwrap_or(&loaded[0]);

        self.selected_preset_id = initial.id.clone();
        self.emu_exe_path = initial.detected_exe_path.clone().unwrap_or_default();
        self.presets = loaded;
    }

    pub fn set_emu_results(&mut self, items: Vec<DiscoveredGame>, preset_name: &str) {
        self.emu_scan_busy = false;
        self.emu_selected = items.iter().map(|i| !i.already_imported).collect();
        if items.is_empty() {
            self.error = Some(format!(
                "No matching {preset_name} games found in this folder."
            ));
        } else {
            self.error = None;
        }
        self.emu_discovered = items;
    }

    pub fn set_discover_results(&mut self, items: Vec<DiscoveredGame>, empty_msg: &str) {
        self.discover_busy = false;
        self.discover_selected = items.iter().map(|i| !i.already_imported).collect();
        if items.is_empty() {
            self.error = Some(empty_msg.to_string());
        } else {
            self.error = None;
        }
        self.discover_items = items;
    }

    pub fn apply_inspected_file(&mut self, info: GameFileInspection, covers_dir: &Path) {
        self.error = None;
        if !info.title.is_empty() {
            self.title = info.title;
        }
        self.platform = info.platform;
        if let Some(exe) = info.executable_path.as_ref() {
            self.executable_path = exe.clone();
        }
        if let Some(args) = info.launch_args.as_ref() {
            self.launch_args = args.clone();
        }
        if let Some(dir) = info.install_dir.as_ref() {
            self.install_dir = dir.clone();
        }
        if let Some(wd) = info.working_dir.as_ref() {
            self.working_dir = wd.clone();
        }
        if !info.tags.is_empty() && self.tags.is_empty() {
            self.tags = info.tags;
        }
        if self.cover_url.trim().is_empty() {
            if let Some(ref cover_src) = info.cover_path {
                if let Ok(copied) = copy_cover_to_dir(Path::new(cover_src), covers_dir) {
                    self.cover_url = copied;
                    self.cover_is_local = true;
                }
            }
        }

        if info.is_rom_file {
            if info.executable_path.is_none() {
                if let Some(ref emu_id) = info.detected_emulator_id {
                    self.smart_hint = Some(format!(
                        "ROM detected ({}). Select the emulator executable below.",
                        emu_id.to_ascii_uppercase()
                    ));
                }
            } else {
                self.smart_hint =
                    Some("ROM & emulator auto-detected and pre-filled.".to_string());
            }
        } else if info.is_emulator_binary {
            self.smart_hint = Some(
                "Emulator detected. Switch to \"Emulator (Cemu, …)\" to scan your ROM folder directly.".to_string(),
            );
        } else if info.cover_path.is_some() {
            self.smart_hint = Some("Game & cover auto-detected in folder.".to_string());
        } else {
            self.smart_hint = None;
        }
    }

    pub fn apply_single_rom(
        &mut self,
        rom_path: &str,
        info: GameFileInspection,
        covers_dir: &Path,
    ) {
        self.error = None;
        let preset = self.selected_preset().cloned();
        let template = preset
            .as_ref()
            .map(|p| {
                if self.emu_fullscreen {
                    p.fullscreen_args_template.as_str()
                } else {
                    p.default_args_template.as_str()
                }
            })
            .unwrap_or("{rom}");

        let built_args = build_emulator_launch_args(template, rom_path);
        let chosen_exe = if !self.emu_exe_path.trim().is_empty() {
            self.emu_exe_path.trim().to_string()
        } else {
            info.executable_path
                .clone()
                .or_else(|| preset.as_ref().and_then(|p| p.detected_exe_path.clone()))
                .unwrap_or_default()
        };
        let chosen_wd = if !chosen_exe.is_empty() {
            parent_dir_str(&chosen_exe)
        } else {
            info.working_dir.clone().unwrap_or_default()
        };

        self.title = info.title.clone();
        self.platform = Platform::Emulator;
        self.executable_path = chosen_exe;
        self.launch_args = built_args;
        self.install_dir = info.install_dir.unwrap_or_default();
        self.working_dir = chosen_wd;
        self.tags = preset
            .as_ref()
            .filter(|p| !p.default_tags.is_empty())
            .map(|p| p.default_tags.clone())
            .unwrap_or(info.tags);

        if let Some(ref cover_src) = info.cover_path {
            if let Ok(copied) = copy_cover_to_dir(Path::new(cover_src), covers_dir) {
                self.cover_url = copied;
                self.cover_is_local = true;
            }
        }

        let preset_name = preset
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("Emulator");
        self.smart_hint = Some(format!(
            "{preset_name} game detected (\"{}\"). Review the fields and click \"Add Game\".",
            info.title
        ));
        self.active_tab = AddModalTab::Manual;
    }

    fn add_tag_from_draft(&mut self, raw: &str) {
        let trimmed = raw.trim().trim_end_matches(',').trim();
        if trimmed.is_empty() {
            self.tag_draft.clear();
            return;
        }
        if !self.tags.iter().any(|t| t.eq_ignore_ascii_case(trimmed)) {
            self.tags.push(trimmed.to_string());
        }
        self.tag_draft.clear();
    }
}

/// Zeigt das Modal zum Hinzufügen oder Bearbeiten eines Spiels.
pub fn show(
    ctx: &egui::Context,
    state: &mut GameFormState,
    existing_tags: &[String],
    actions: &mut Vec<AppAction>,
) {
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.close_requested_flag = true;
        return;
    }

    if paint_modal_backdrop(ctx, "game_form_modal_backdrop") {
        state.close_requested_flag = true;
        return;
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 48.0).clamp(500.0, 760.0);
    let max_h = (screen.height() - 48.0).max(420.0);

    egui::Area::new(Id::new("game_form_modal_panel"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(modal_w);
            ui.set_max_height(max_h);

            glass_modal_frame().show(ui, |ui| {
                ui.set_width(modal_w);
                ui.set_max_height(max_h);

                // Header mit Titel, Tabs & Close-Button
                Frame::NONE
                    .inner_margin(Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let heading = if state.is_editing() {
                                format!("Edit {}", state.title)
                            } else {
                                "Add Game".to_string()
                            };
                            ui.label(
                                egui::RichText::new(heading)
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );

                            if !state.is_editing() {
                                ui.add_space(12.0);
                                Frame::NONE
                                    .fill(BG_GLASS_WELL)
                                    .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
                                    .corner_radius(10.0)
                                    .inner_margin(Margin::same(3))
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        for (tab, icon, label) in [
                                            (
                                                AddModalTab::Emulator,
                                                IconKind::Cpu,
                                                "Emulator (Cemu, …)",
                                            ),
                                            (
                                                AddModalTab::Discover,
                                                IconKind::FolderSearch,
                                                "Auto-Discover",
                                            ),
                                            (AddModalTab::Manual, IconKind::Plus, "Manual"),
                                        ] {
                                            let active = state.active_tab == tab;
                                            if tab_pill(ui, icon, label, active).clicked() {
                                                state.error = None;
                                                state.active_tab = tab;
                                                if tab == AddModalTab::Discover
                                                    && state.discover_items.is_empty()
                                                    && !state.discover_busy
                                                {
                                                    state.discover_busy = true;
                                                    actions.push(AppAction::ScanPcStores);
                                                }
                                            }
                                        }
                                    });
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if glass_icon_button(
                                        ui,
                                        "form_modal_close",
                                        IconKind::Close,
                                        28.0,
                                        false,
                                        false,
                                    )
                                    .clicked()
                                    {
                                        state.close_requested_flag = true;
                                    }
                                },
                            );
                        });
                    });

                ui.painter().line_segment(
                    [
                        Pos2::new(ui.min_rect().left(), ui.cursor().top()),
                        Pos2::new(ui.min_rect().left() + modal_w, ui.cursor().top()),
                    ],
                    Stroke::new(1.0_f32, BORDER_SUBTLE),
                );

                // Scrollbarer Body
                ScrollArea::vertical()
                    .id_salt("game_form_scroll")
                    .max_height(max_h - 68.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        Frame::NONE
                            .inner_margin(Margin::same(22))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 14.0;

                                if let Some(ref err) = state.error {
                                    Frame::NONE
                                        .fill(DANGER_BG)
                                        .stroke(Stroke::new(1.0_f32, DANGER_BORDER))
                                        .corner_radius(10.0)
                                        .inner_margin(Margin::symmetric(12, 9))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.label(
                                                egui::RichText::new(err)
                                                    .size(12.0)
                                                    .color(DANGER_TEXT),
                                            );
                                        });
                                }

                                if !state.is_editing() && state.active_tab == AddModalTab::Emulator
                                {
                                    show_emulator_tab(ui, state, actions);
                                } else if !state.is_editing()
                                    && state.active_tab == AddModalTab::Discover
                                {
                                    show_discover_tab(ui, state, actions);
                                } else {
                                    show_manual_tab(ui, state, existing_tags, actions);
                                }
                            });
                    });
            });
        });
}

fn show_emulator_tab(
    ui: &mut egui::Ui,
    state: &mut GameFormState,
    actions: &mut Vec<AppAction>,
) {
    let detected_count = state
        .presets
        .iter()
        .filter(|p| p.detected_exe_path.is_some())
        .count();

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("1. Select Emulator")
                .size(12.5)
                .strong()
                .color(TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let status_txt = if state.presets_loading {
                "Searching emulators...".to_string()
            } else {
                format!("{detected_count} detected locally")
            };
            ui.label(
                egui::RichText::new(status_txt)
                    .monospace()
                    .size(11.0)
                    .color(TEXT_DIM),
            );
        });
    });

    // Preset grid (3 columns)
    let gap = 8.0;
    let cols = 3usize;
    let avail_w = ui.available_width();
    let card_w = ((avail_w - gap * (cols as f32 - 1.0)) / cols as f32).max(120.0);

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        let presets_snapshot = state.presets.clone();
        for row in presets_snapshot.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for preset in row {
                    let selected = state
                        .selected_preset()
                        .is_some_and(|p| p.id == preset.id);
                    if preset_card(ui, preset, selected, card_w).clicked() {
                        state.selected_preset_id = preset.id.clone();
                        state.emu_exe_path =
                            preset.detected_exe_path.clone().unwrap_or_default();
                        state.emu_discovered.clear();
                        state.emu_selected.clear();
                        state.error = None;
                        if let Some(first_dir) = preset.detected_rom_dirs.first() {
                            state.emu_scan_busy = true;
                            actions.push(AppAction::ScanEmulatorFolder {
                                preset_id: preset.id.clone(),
                                emulator_path: opt_trimmed(&state.emu_exe_path),
                                rom_dir: first_dir.clone(),
                                fullscreen: state.emu_fullscreen,
                            });
                        }
                    }
                }
            });
        }
    });

    if let Some(preset) = state.selected_preset().cloned() {
        glass_well_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 10.0;

            ui.label(
                egui::RichText::new(format!("2. {} Executable", preset.name))
                    .size(12.0)
                    .strong()
                    .color(TEXT_SECONDARY),
            );

            ui.horizontal(|ui| {
                let input_w = (ui.available_width() - 220.0).max(160.0);
                glass_text_edit(
                    ui,
                    &mut state.emu_exe_path,
                    &format!("Path to {} (e.g. Cemu.exe)", preset.name),
                    true,
                    Some(input_w),
                );
                if glass_secondary_button(ui, Some(IconKind::Folder), "Browse", true).clicked()
                {
                    actions.push(AppAction::PickEmulatorBinary);
                }
                let fs_icon = if state.emu_fullscreen {
                    Some(IconKind::Check)
                } else {
                    None
                };
                if glass_pill_button(
                    ui,
                    fs_icon,
                    "Launch Fullscreen",
                    state.emu_fullscreen,
                    None,
                    false,
                )
                .clicked()
                {
                    state.emu_fullscreen = !state.emu_fullscreen;
                }
            });

            if !preset.detected_rom_dirs.is_empty() {
                Frame::NONE
                    .fill(theme::rgba(255, 255, 255, 7))
                    .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
                    .corner_radius(10.0)
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new(format!(
                                "Detected {} game folders:",
                                preset.name
                            ))
                            .size(11.0)
                            .color(TEXT_SECONDARY),
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                            for dir in &preset.detected_rom_dirs {
                                if glass_secondary_button(
                                    ui,
                                    Some(IconKind::FolderSearch),
                                    dir,
                                    !state.emu_scan_busy,
                                )
                                .clicked()
                                {
                                    state.emu_scan_busy = true;
                                    state.error = None;
                                    actions.push(AppAction::ScanEmulatorFolder {
                                        preset_id: preset.id.clone(),
                                        emulator_path: opt_trimmed(&state.emu_exe_path),
                                        rom_dir: dir.clone(),
                                        fullscreen: state.emu_fullscreen,
                                    });
                                }
                            }
                        });
                    });
            }

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                let scan_lbl = if state.emu_scan_busy {
                    "Scanning folder..."
                } else {
                    "Scan ROM Folder"
                };
                if glass_primary_button(
                    ui,
                    Some(IconKind::FolderSearch),
                    scan_lbl,
                    !state.emu_scan_busy,
                )
                .clicked()
                {
                    actions.push(AppAction::PickEmulatorRomDir {
                        preset_name: preset.name.clone(),
                    });
                }

                if glass_secondary_button(
                    ui,
                    Some(IconKind::Plus),
                    "Pick Single Game File",
                    true,
                )
                .clicked()
                {
                    actions.push(AppAction::PickSingleRom {
                        preset_name: preset.name.clone(),
                        extensions: preset.extensions.clone(),
                    });
                }

                if !preset.extensions.is_empty() {
                    let ext_text = preset
                        .extensions
                        .iter()
                        .map(|e| format!(".{e}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    ui.label(
                        egui::RichText::new(ext_text)
                            .monospace()
                            .size(11.0)
                            .color(TEXT_DIM),
                    );
                }
            });
        });
    }

    if !state.emu_discovered.is_empty() {
        let override_exe = opt_trimmed(&state.emu_exe_path);
        let preset_name = state
            .selected_preset()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "the emulator".to_string());
        if let Some(items_to_import) = show_discovered_list(
            ui,
            "emu_discovered_list",
            &state.emu_discovered,
            &mut state.emu_selected,
            &mut state.emu_search,
            state.is_submitting,
        ) {
            if override_exe.is_none()
                && state
                    .emu_discovered
                    .iter()
                    .any(|g| g.executable_path.is_none())
            {
                state.error = Some(format!(
                    "Please select the {preset_name} executable first (e.g. Cemu.exe)."
                ));
            } else {
                let payload = items_to_import
                    .iter()
                    .map(|g| discovered_to_new_input(g, override_exe.as_deref()))
                    .collect();
                state.is_submitting = true;
                actions.push(AppAction::BulkImportGames(payload));
            }
        }
    }
}

fn show_discover_tab(
    ui: &mut egui::Ui,
    state: &mut GameFormState,
    actions: &mut Vec<AppAction>,
) {
    let half_w = ((ui.available_width() - 12.0) * 0.5).max(180.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        if discover_action_card(
            ui,
            half_w,
            IconKind::Refresh,
            "Scan Installed Stores & Emulators",
            "Automatically detects Steam, Epic Games, GOG Galaxy & emulator libraries.",
            !state.discover_busy,
        )
        .clicked()
        && !state.discover_busy
        {
            state.error = None;
            state.discover_busy = true;
            actions.push(AppAction::ScanPcStores);
        }

        if discover_action_card(
            ui,
            half_w,
            IconKind::FolderSearch,
            "Scan Games Folder",
            "Searches e.g. D:\\Games automatically for .exe binaries & covers.",
            !state.discover_busy,
        )
        .clicked()
        && !state.discover_busy
        {
            actions.push(AppAction::PickPcGamesFolder);
        }
    });

    if state.discover_busy {
        ui.label(
            egui::RichText::new("Scanning installed games...")
                .size(12.0)
                .color(TEXT_MUTED),
        );
    }

    if !state.discover_items.is_empty() {
        if let Some(items_to_import) = show_discovered_list(
            ui,
            "pc_discovered_list",
            &state.discover_items,
            &mut state.discover_selected,
            &mut state.discover_search,
            state.is_submitting,
        ) {
            let payload = items_to_import
                .iter()
                .map(|g| discovered_to_new_input(g, None))
                .collect();
            state.is_submitting = true;
            actions.push(AppAction::BulkImportGames(payload));
        }
    }
}

fn show_manual_tab(
    ui: &mut egui::Ui,
    state: &mut GameFormState,
    existing_tags: &[String],
    actions: &mut Vec<AppAction>,
) {
    if !state.is_editing() {
        glass_well_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                paint_icon(ui.painter(), r.center(), 14.0, IconKind::Sparkles, TEXT_PRIMARY);

                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 205.0).max(160.0));
                    ui.label(
                        egui::RichText::new(
                            "Select an .exe, .desktop, or ROM file – title, path, arguments & cover are detected automatically.",
                        )
                        .size(11.5)
                        .color(TEXT_SECONDARY),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if glass_secondary_button(
                        ui,
                        Some(IconKind::Folder),
                        "Select & Detect File",
                        true,
                    )
                    .clicked()
                    {
                        actions.push(AppAction::PickExecutableForInspect);
                    }
                });
            });
        });
    }

    if let Some(ref hint) = state.smart_hint {
        Frame::NONE
            .fill(theme::rgba(255, 255, 255, 11))
            .stroke(Stroke::new(1.0_f32, BORDER_HOVER))
            .corner_radius(10.0)
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(hint).size(12.0).color(TEXT_PRIMARY));
            });
    }

    // 2-Spalten-Formular: Links 2:3-Cover, Rechts Metadaten
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;

        // Linke Spalte: Cover-Vorschau
        ui.vertical(|ui| {
            ui.set_width(134.0);
            ui.spacing_mut().item_spacing.y = 8.0;

            let (c_rect, _) = ui.allocate_exact_size(Vec2::new(134.0, 201.0), Sense::hover());
            let uri = cover_image_uri(opt_trimmed(&state.cover_url).as_deref(), state.cover_is_local);
            let drew = uri.as_deref().is_some_and(|u| {
                paint_cover_image(ui, u, c_rect, 11, Color32::WHITE, 1.0)
            });
            if !drew {
                ui.painter().rect(
                    c_rect,
                    11.0,
                    BG_GLASS_WELL,
                    Stroke::new(1.0_f32, BORDER_GLASS),
                    StrokeKind::Inside,
                );
                paint_icon(
                    ui.painter(),
                    c_rect.center() - Vec2::new(0.0, 12.0),
                    22.0,
                    IconKind::ImagePlus,
                    TEXT_DIM,
                );
                let g = ui.painter().layout_no_wrap(
                    "Cover (2:3)".to_string(),
                    egui::FontId::monospace(10.5),
                    TEXT_DIM,
                );
                ui.painter().galley(
                    Pos2::new(
                        c_rect.center().x - g.size().x * 0.5,
                        c_rect.center().y + 10.0,
                    ),
                    g,
                    TEXT_DIM,
                );
            }

            if ui
                .add_sized([134.0, 32.0], |ui: &mut egui::Ui| {
                    glass_secondary_button(
                        ui,
                        Some(IconKind::ImagePlus),
                        "Local Cover",
                        true,
                    )
                })
                .clicked()
            {
                actions.push(AppAction::PickLocalCover);
            }

            if !state.cover_url.trim().is_empty()
                && glass_secondary_button(ui, Some(IconKind::Trash), "Remove Cover", true)
                    .clicked()
            {
                state.cover_url.clear();
                state.cover_is_local = false;
            }
        });

        // Right column: input fields
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 10.0;

            field_label(ui, "Title *");
            glass_text_edit(
                ui,
                &mut state.title,
                "e.g. Breath of the Wild or Hades II",
                false,
                None,
            );

            // Platform + Steam AppID / Favorite
            let col_w = ((ui.available_width() - 12.0) * 0.5).max(120.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    field_label(ui, "Platform");
                    egui::ComboBox::from_id_salt("form_platform_combo")
                        .selected_text(platform_label(state.platform))
                        .width(col_w)
                        .show_ui(ui, |ui| {
                            for p in [Platform::Manual, Platform::Emulator, Platform::Steam] {
                                ui.selectable_value(&mut state.platform, p, platform_label(p));
                            }
                        });
                });

                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    if state.platform == Platform::Steam {
                        field_label(ui, "Steam AppID");
                        glass_text_edit(
                            ui,
                            &mut state.steam_app_id,
                            "e.g. 1145360",
                            true,
                            Some(col_w),
                        );
                    } else {
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            glass_toggle(ui, &mut state.is_favorite);
                            ui.label(
                                egui::RichText::new("Mark as Favorite")
                                    .size(12.0)
                                    .color(TEXT_SECONDARY),
                            );
                        });
                    }
                });
            });

            let exe_label = if state.platform == Platform::Emulator {
                "Emulator Executable (e.g. Cemu.exe)"
            } else {
                "Executable File (.exe / Binary / .desktop)"
            };
            field_label(ui, exe_label);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 92.0).max(140.0);
                let hint = if state.platform == Platform::Emulator {
                    r"e.g. C:\Emulators\Cemu\Cemu.exe"
                } else {
                    "Path to .exe, Linux binary, or .desktop file"
                };
                glass_text_edit(ui, &mut state.executable_path, hint, true, Some(w));
                if glass_secondary_button(ui, Some(IconKind::Folder), "Browse", true).clicked()
                {
                    actions.push(AppAction::PickExecutableForInspect);
                }
            });

            field_label(ui, "Launch Arguments (optional)");
            glass_text_edit(
                ui,
                &mut state.launch_args,
                r#"e.g. -f -g "D:\WiiU\BotW.wua""#,
                true,
                None,
            );

            field_label(ui, "Working Directory (optional)");
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 92.0).max(140.0);
                glass_text_edit(
                    ui,
                    &mut state.working_dir,
                    "Automatic (executable parent directory)",
                    true,
                    Some(w),
                );
                if glass_secondary_button(ui, Some(IconKind::Folder), "Folder", true).clicked()
                {
                    actions.push(AppAction::PickWorkingDir);
                }
            });

            if !state.cover_is_local {
                field_label(ui, "Cover URL (optional)");
                let resp = glass_text_edit(
                    ui,
                    &mut state.cover_url,
                    "https://...",
                    true,
                    None,
                );
                if resp.changed() {
                    state.cover_is_local = false;
                }
            }

            if state.platform == Platform::Steam {
                field_label(ui, "Install Directory (for playtime tracking)");
                glass_text_edit(
                    ui,
                    &mut state.install_dir,
                    r"e.g. C:\Program Files (x86)\Steam\steamapps\common\Hades",
                    true,
                    None,
                );
            }
        });
    });

    // Tags section
    glass_well_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;

        field_label(ui, "Tags");

        if !state.tags.is_empty() {
            let mut remove_idx: Option<usize> = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                for (idx, tag) in state.tags.iter().enumerate() {
                    Frame::NONE
                        .fill(BG_PILL_ACTIVE)
                        .stroke(Stroke::new(1.0_f32, BORDER_ACTIVE))
                        .corner_radius(8.0)
                        .inner_margin(Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                ui.label(
                                    egui::RichText::new(tag)
                                        .size(11.5)
                                        .color(Color32::WHITE),
                                );
                                if glass_icon_button(
                                    ui,
                                    ("rm_tag", idx),
                                    IconKind::Close,
                                    16.0,
                                    false,
                                    true,
                                )
                                .clicked()
                                {
                                    remove_idx = Some(idx);
                                }
                            });
                        });
                }
            });
            if let Some(idx) = remove_idx {
                state.tags.remove(idx);
            }
        }

        ui.horizontal(|ui| {
            let w = (ui.available_width() - 116.0).max(150.0);
            let resp = glass_text_edit(
                ui,
                &mut state.tag_draft,
                "Type a tag and press Enter (e.g. Wii U, RPG, Indie)",
                false,
                Some(w),
            );
            let enter_hit = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let comma_typed = state.tag_draft.ends_with(',');
            if enter_hit || comma_typed {
                let draft = state.tag_draft.clone();
                state.add_tag_from_draft(&draft);
            }
            if glass_secondary_button(ui, Some(IconKind::Plus), "Add", true).clicked() {
                let draft = state.tag_draft.clone();
                state.add_tag_from_draft(&draft);
            }
        });

        let unused: Vec<&String> = existing_tags
            .iter()
            .filter(|t| !state.tags.iter().any(|active| active.eq_ignore_ascii_case(t)))
            .take(10)
            .collect();
        if !unused.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                ui.label(
                    egui::RichText::new("Suggestions:")
                        .size(11.0)
                        .color(TEXT_DIM),
                );
                for suggestion in unused {
                    if glass_secondary_button(ui, None, &format!("+ {suggestion}"), true).clicked()
                    {
                        state.add_tag_from_draft(suggestion);
                    }
                }
            });
        }
    });

    // Footer: Cancel + Save
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let submit_lbl = if state.is_submitting {
                "Saving..."
            } else if state.is_editing() {
                "Save Changes"
            } else {
                "Add Game"
            };
            if glass_primary_button(ui, None, submit_lbl, !state.is_submitting).clicked() {
                let title = state.title.trim().to_string();
                if title.is_empty() {
                    state.error = Some("Please enter a game title.".to_string());
                    return;
                }
                if state.platform != Platform::Steam && state.executable_path.trim().is_empty() {
                    state.error = Some(if state.platform == Platform::Emulator {
                        "Please select the emulator executable (e.g. Cemu.exe).".to_string()
                    } else {
                        "Please select an executable file or launcher.".to_string()
                    });
                    return;
                }

                state.error = None;
                state.is_submitting = true;

                if let Some(ref game_id) = state.editing_game_id {
                    actions.push(AppAction::SaveUpdatedGame(UpdateGameInput {
                        id: game_id.clone(),
                        title,
                        platform: state.platform,
                        steam_app_id: opt_trimmed(&state.steam_app_id),
                        executable_path: opt_trimmed(&state.executable_path),
                        install_dir: opt_trimmed(&state.install_dir),
                        launch_args: opt_trimmed(&state.launch_args),
                        working_dir: opt_trimmed(&state.working_dir),
                        cover_url: opt_trimmed(&state.cover_url),
                        cover_is_local: state.cover_is_local,
                        hero_url: state.editing_hero_url.clone(),
                        is_favorite: state.is_favorite,
                        tags: state.tags.clone(),
                    }));
                } else {
                    actions.push(AppAction::SaveNewGame(NewGameInput {
                        title,
                        platform: state.platform,
                        steam_app_id: opt_trimmed(&state.steam_app_id),
                        executable_path: opt_trimmed(&state.executable_path),
                        install_dir: opt_trimmed(&state.install_dir),
                        launch_args: opt_trimmed(&state.launch_args),
                        working_dir: opt_trimmed(&state.working_dir),
                        cover_url: opt_trimmed(&state.cover_url),
                        cover_is_local: state.cover_is_local,
                        hero_url: None,
                        is_favorite: state.is_favorite,
                        tags: state.tags.clone(),
                    }));
                }
            }

            if glass_secondary_button(ui, None, "Cancel", true).clicked() {
                state.close_requested_flag = true;
            }
        });
    });
}

fn show_discovered_list(
    ui: &mut egui::Ui,
    id_salt: &str,
    items: &[DiscoveredGame],
    selected: &mut [bool],
    search: &mut String,
    is_submitting: bool,
) -> Option<Vec<DiscoveredGame>> {
    let mut import_trigger: Option<Vec<DiscoveredGame>> = None;

    glass_well_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 10.0;

        // Header with filter field & selection actions
        ui.horizontal(|ui| {
            let search_w = (ui.available_width() - 250.0).max(140.0);
            glass_text_edit(ui, search, "Filter discovered games...", false, Some(search_w));

            if glass_secondary_button(ui, None, "Select All New", true).clicked() {
                for (i, item) in items.iter().enumerate() {
                    if let Some(slot) = selected.get_mut(i) {
                        *slot = !item.already_imported;
                    }
                }
            }
            if glass_secondary_button(ui, None, "Clear Selection", true).clicked() {
                for slot in selected.iter_mut() {
                    *slot = false;
                }
            }
        });

        let q = search.trim().to_ascii_lowercase();
        ScrollArea::vertical()
            .id_salt(id_salt)
            .max_height(220.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;

                for (idx, item) in items.iter().enumerate() {
                    if !q.is_empty()
                        && !item.title.to_ascii_lowercase().contains(&q)
                        && !item.source.to_ascii_lowercase().contains(&q)
                    {
                        continue;
                    }

                    let checked = selected.get(idx).copied().unwrap_or(false);
                    let row_resp = Frame::NONE
                        .fill(if checked {
                            BG_PILL_ACTIVE
                        } else {
                            theme::rgba(255, 255, 255, 6)
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if checked {
                                BORDER_ACTIVE
                            } else {
                                BORDER_SUBTLE
                            },
                        ))
                        .corner_radius(9.0)
                        .inner_margin(Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (cb_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                let p = ui.painter();
                                p.rect(
                                    cb_rect,
                                    5.0,
                                    if checked {
                                        Color32::WHITE
                                    } else {
                                        BG_GLASS_WELL
                                    },
                                    Stroke::new(
                                        1.0_f32,
                                        if checked {
                                            Color32::WHITE
                                        } else {
                                            BORDER_GLASS
                                        },
                                    ),
                                    StrokeKind::Inside,
                                );
                                if checked {
                                    paint_icon(
                                        p,
                                        cb_rect.center(),
                                        11.0,
                                        IconKind::Check,
                                        Color32::from_rgb(9, 9, 11),
                                    );
                                }

                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(&item.title)
                                                .size(12.5)
                                                .strong()
                                                .color(TEXT_PRIMARY),
                                        );
                                        status_badge(
                                            ui,
                                            &item.source,
                                            theme::rgba(255, 255, 255, 11),
                                            BORDER_GLASS,
                                            TEXT_SECONDARY,
                                            None,
                                        );
                                        if item.already_imported {
                                            status_badge(
                                                ui,
                                                "Already imported",
                                                theme::rgba(255, 255, 255, 7),
                                                BORDER_SUBTLE,
                                                TEXT_DIM,
                                                None,
                                            );
                                        }
                                    });

                                    let detail = item
                                        .launch_args
                                        .as_deref()
                                        .or(item.executable_path.as_deref())
                                        .or(item.install_dir.as_deref())
                                        .unwrap_or("");
                                    if !detail.is_empty() {
                                        ui.label(
                                            egui::RichText::new(detail)
                                                .monospace()
                                                .size(10.5)
                                                .color(TEXT_DIM),
                                        );
                                    }
                                });
                            });
                        })
                        .response
                        .interact(Sense::click())
                        .on_hover_cursor(CursorIcon::PointingHand);

                    if row_resp.clicked() {
                        if let Some(slot) = selected.get_mut(idx) {
                            *slot = !*slot;
                        }
                    }
                }
            });

        let selected_count = selected.iter().filter(|&&v| v).count();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{selected_count} of {} games selected",
                    items.len()
                ))
                .size(11.5)
                .color(TEXT_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let lbl = if is_submitting {
                    "Importing...".to_string()
                } else {
                    format!("Import {selected_count} selected")
                };
                if glass_primary_button(
                    ui,
                    Some(IconKind::Plus),
                    &lbl,
                    selected_count > 0 && !is_submitting,
                )
                .clicked()
                {
                    let chosen = items
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| selected.get(*i).copied().unwrap_or(false))
                        .map(|(_, item)| item.clone())
                        .collect();
                    import_trigger = Some(chosen);
                }
            });
        });
    });

    import_trigger
}

fn tab_pill(ui: &mut egui::Ui, icon: IconKind, label: &str, active: bool) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let measure = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(measure.size().x + 34.0, 25.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let bg = if active {
            BG_PILL_ACTIVE
        } else if hovered {
            theme::rgba(255, 255, 255, 10)
        } else {
            Color32::TRANSPARENT
        };
        let border = if active {
            Stroke::new(1.0_f32, BORDER_ACTIVE)
        } else {
            Stroke::NONE
        };
        let fg = if active || hovered {
            TEXT_PRIMARY
        } else {
            TEXT_MUTED
        };
        let p = ui.painter();
        p.rect(rect, 7.0, bg, border, StrokeKind::Inside);
        if active {
            theme::paint_glass_specular_top(p, rect, 7.0);
        }
        paint_icon(
            p,
            Pos2::new(rect.left() + 14.0, rect.center().y),
            12.0,
            icon,
            fg,
        );
        let galley = p.layout_no_wrap(label.to_string(), font, fg);
        p.galley(
            Pos2::new(rect.left() + 25.0, rect.center().y - galley.size().y * 0.5),
            galley,
            fg,
        );
    }

    resp
}

fn preset_card(
    ui: &mut egui::Ui,
    preset: &EmulatorPresetInfo,
    selected: bool,
    width: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 48.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let bg = if selected {
            BG_PILL_ACTIVE
        } else if hovered {
            theme::rgba(255, 255, 255, 12)
        } else {
            BG_GLASS_WELL
        };
        let border = if selected {
            BORDER_ACTIVE
        } else if hovered {
            BORDER_HOVER
        } else {
            BORDER_GLASS
        };
        let p = ui.painter();
        p.rect(
            rect,
            10.0,
            bg,
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
        if selected {
            theme::paint_glass_specular_top(p, rect, 10.0);
        }

        let name_g = p.layout_no_wrap(
            preset.name.clone(),
            egui::FontId::proportional(12.0),
            TEXT_PRIMARY,
        );
        let sys_g = p.layout_no_wrap(
            preset.system.clone(),
            egui::FontId::proportional(10.5),
            TEXT_MUTED,
        );
        let clip = rect.shrink2(Vec2::new(10.0, 4.0));
        p.with_clip_rect(clip)
            .galley(Pos2::new(rect.left() + 10.0, rect.top() + 8.0), name_g, TEXT_PRIMARY);
        p.with_clip_rect(clip)
            .galley(Pos2::new(rect.left() + 10.0, rect.top() + 26.0), sys_g, TEXT_MUTED);

        if preset.detected_exe_path.is_some() {
            p.circle_filled(
                Pos2::new(rect.right() - 12.0, rect.top() + 14.0),
                3.5,
                EMERALD_DOT,
            );
        }
    }

    resp
}

fn discover_action_card(
    ui: &mut egui::Ui,
    width: f32,
    icon: IconKind,
    title: &str,
    description: &str,
    enabled: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(width, 78.0),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    let resp = if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    };

    if ui.is_rect_visible(rect) {
        let hovered = enabled && resp.hovered();
        let p = ui.painter();
        p.rect(
            rect,
            12.0,
            if hovered {
                theme::BG_GLASS_HOVER
            } else {
                BG_GLASS_SURFACE
            },
            Stroke::new(1.0_f32, if hovered { BORDER_HOVER } else { BORDER_GLASS }),
            StrokeKind::Inside,
        );
        theme::paint_glass_specular_top(p, rect, 12.0);

        let icon_box = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, rect.top() + 16.0),
            Vec2::splat(34.0),
        );
        p.rect(
            icon_box,
            9.0,
            BG_GLASS_WELL,
            Stroke::new(1.0_f32, BORDER_GLASS),
            StrokeKind::Inside,
        );
        paint_icon(p, icon_box.center(), 16.0, icon, TEXT_SECONDARY);

        let text_left = icon_box.right() + 12.0;
        let text_w = (rect.right() - 14.0 - text_left).max(60.0);
        let tg = p.layout_no_wrap(
            title.to_string(),
            egui::FontId::proportional(12.5),
            TEXT_PRIMARY,
        );
        let dg = p.layout(
            description.to_string(),
            egui::FontId::proportional(11.0),
            TEXT_MUTED,
            text_w,
        );
        p.galley(Pos2::new(text_left, rect.top() + 14.0), tg, Color32::WHITE);
        p.galley(Pos2::new(text_left, rect.top() + 34.0), dg, TEXT_MUTED);
    }

    resp
}

fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(11.5)
            .strong()
            .color(TEXT_SECONDARY),
    );
}

fn opt_trimmed(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parent_dir_str(file_path: &str) -> String {
    Path::new(file_path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn discovered_to_new_input(
    item: &DiscoveredGame,
    override_exe: Option<&str>,
) -> NewGameInput {
    let exe = override_exe
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| item.executable_path.clone());

    let working_dir = item.working_dir.clone().or_else(|| {
        exe.as_deref()
            .map(parent_dir_str)
            .filter(|s| !s.is_empty())
    });

    let cover_url = item.cover_path.clone().or_else(|| item.cover_url.clone());

    NewGameInput {
        title: item.title.clone(),
        platform: item.platform,
        steam_app_id: item.steam_app_id.clone(),
        executable_path: exe,
        install_dir: item.install_dir.clone(),
        launch_args: item.launch_args.clone(),
        working_dir,
        cover_url,
        cover_is_local: item.cover_path.is_some(),
        hero_url: item.hero_url.clone(),
        is_favorite: false,
        tags: item.tags.clone(),
    }
}
