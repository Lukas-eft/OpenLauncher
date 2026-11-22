use eframe::egui::{self, Align2, Color32, Frame, Id, Margin, Pos2, Stroke, Vec2};

use crate::models::AppSettings;
use crate::theme::{BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY};
use crate::ui::icons::IconKind;
use crate::ui::widgets::{
    glass_icon_button, glass_modal_frame, glass_primary_button, glass_secondary_button,
    glass_text_edit, glass_toggle, glass_well_frame, paint_modal_backdrop,
};
use crate::ui::AppAction;

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub minimize_to_tray: bool,
    pub auto_scan_steam_on_startup: bool,
    pub custom_steam_path: String,
    pub is_saving: bool,
}

impl From<&AppSettings> for SettingsModalState {
    fn from(settings: &AppSettings) -> Self {
        Self {
            minimize_to_tray: settings.minimize_to_tray,
            auto_scan_steam_on_startup: settings.auto_scan_steam_on_startup,
            custom_steam_path: settings.custom_steam_path.clone().unwrap_or_default(),
            is_saving: false,
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    state: &mut SettingsModalState,
    is_scanning_steam: bool,
    actions: &mut Vec<AppAction>,
) {
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        actions.push(AppAction::CloseSettings);
        return;
    }

    if paint_modal_backdrop(ctx, "settings_modal_backdrop") {
        actions.push(AppAction::CloseSettings);
        return;
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 48.0).clamp(380.0, 520.0);

    egui::Area::new(Id::new("settings_modal_panel"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(modal_w);

            glass_modal_frame().show(ui, |ui| {
                ui.set_width(modal_w);

                Frame::NONE
                    .inner_margin(Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Settings")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if glass_icon_button(
                                        ui,
                                        "settings_close",
                                        IconKind::Close,
                                        28.0,
                                        false,
                                        false,
                                    )
                                    .clicked()
                                    {
                                        actions.push(AppAction::CloseSettings);
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

                Frame::NONE
                    .inner_margin(Margin::same(20))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 12.0;

                        glass_well_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_max_width(ui.available_width() - 52.0);
                                    ui.label(
                                        egui::RichText::new("Minimize to System Tray")
                                            .size(13.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Closing the window hides OpenLauncher in the tray so active playtime tracking continues.",
                                        )
                                        .size(11.5)
                                        .color(TEXT_MUTED),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |ui| {
                                        glass_toggle(ui, &mut state.minimize_to_tray);
                                    },
                                );
                            });
                        });

                        glass_well_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_max_width(ui.available_width() - 52.0);
                                    ui.label(
                                        egui::RichText::new("Scan Steam on Startup")
                                            .size(13.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Automatically syncs installed Steam games when launching the app.",
                                        )
                                        .size(11.5)
                                        .color(TEXT_MUTED),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |ui| {
                                        glass_toggle(ui, &mut state.auto_scan_steam_on_startup);
                                    },
                                );
                            });
                        });

                        glass_well_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new("Custom Steam Path (optional)")
                                    .size(13.0)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    "Leave empty for automatic detection via Windows Registry and default paths.",
                                )
                                .size(11.5)
                                .color(TEXT_MUTED),
                            );
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                let has_reset = !state.custom_steam_path.trim().is_empty();
                                let reserved = if has_reset { 132.0 } else { 94.0 };
                                let input_w = (ui.available_width() - reserved).max(150.0);
                                glass_text_edit(
                                    ui,
                                    &mut state.custom_steam_path,
                                    "Automatic detection",
                                    true,
                                    Some(input_w),
                                );

                                if glass_secondary_button(
                                    ui,
                                    Some(IconKind::Folder),
                                    "Folder",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(AppAction::PickSteamDir);
                                }

                                if has_reset
                                    && glass_icon_button(
                                        ui,
                                        "reset_steam_dir",
                                        IconKind::RotateCcw,
                                        32.0,
                                        false,
                                        false,
                                    )
                                    .clicked()
                                {
                                    state.custom_steam_path.clear();
                                }
                            });

                            ui.add_space(8.0);
                            let scan_label = if is_scanning_steam {
                                "Scanning Steam..."
                            } else {
                                "Scan Steam Libraries Now"
                            };
                            if glass_secondary_button(
                                ui,
                                Some(IconKind::Refresh),
                                scan_label,
                                !is_scanning_steam,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::ScanSteam);
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let save_label = if state.is_saving {
                                        "Saving..."
                                    } else {
                                        "Save"
                                    };
                                    if glass_primary_button(ui, None, save_label, !state.is_saving)
                                        .clicked()
                                    {
                                        state.is_saving = true;
                                        let custom = state.custom_steam_path.trim().to_string();
                                        actions.push(AppAction::SaveSettings(AppSettings {
                                            minimize_to_tray: state.minimize_to_tray,
                                            custom_steam_path: if custom.is_empty() {
                                                None
                                            } else {
                                                Some(custom)
                                            },
                                            auto_scan_steam_on_startup: state
                                                .auto_scan_steam_on_startup,
                                        }));
                                    }

                                    if glass_secondary_button(ui, None, "Cancel", true)
                                        .clicked()
                                    {
                                        actions.push(AppAction::CloseSettings);
                                    }
                                },
                            );
                        });
                    });
            });
        });
}
