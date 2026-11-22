use eframe::egui::{self, Frame, Margin, Pos2, ScrollArea, Stroke};

use crate::models::{Game, Platform};
use crate::theme::{BG_SIDEBAR, BORDER_SUBTLE, TEXT_DIM};
use crate::ui::icons::IconKind;
use crate::ui::widgets::{glass_pill_button, glass_secondary_button, section_overline};
use crate::ui::{AddModalTab, AppAction, SidebarFilter};

/// Linke Navigations-Sidebar mit Bibliothek, Plattformen, Tags und Aktions-Footer.
pub fn show(
    ctx: &egui::Context,
    games: &[Game],
    tags: &[String],
    active_filter: &SidebarFilter,
    is_scanning_steam: bool,
    actions: &mut Vec<AppAction>,
) {
    let favorites_count = games.iter().filter(|g| g.is_favorite).count();
    let recent_count = games.iter().filter(|g| g.last_played_at.is_some()).count();
    let steam_count = games
        .iter()
        .filter(|g| g.platform == Platform::Steam)
        .count();
    let manual_count = games
        .iter()
        .filter(|g| g.platform == Platform::Manual)
        .count();
    let emulator_count = games
        .iter()
        .filter(|g| g.platform == Platform::Emulator)
        .count();

    egui::SidePanel::left("sidebar")
        .exact_width(236.0)
        .resizable(false)
        .frame(Frame::NONE.fill(BG_SIDEBAR).inner_margin(Margin::ZERO))
        .show_separator_line(false)
        .show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            ui.painter().line_segment(
                [
                    Pos2::new(panel_rect.right(), panel_rect.top()),
                    Pos2::new(panel_rect.right(), panel_rect.bottom()),
                ],
                Stroke::new(1.0_f32, BORDER_SUBTLE),
            );

            // Fixierter Aktions-Footer am unteren Rand der Sidebar
            egui::TopBottomPanel::bottom("sidebar_footer")
                .frame(
                    Frame::NONE
                        .fill(BG_SIDEBAR)
                        .inner_margin(Margin::symmetric(12, 12)),
                )
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    let f_rect = ui.max_rect();
                    ui.painter().line_segment(
                        [
                            Pos2::new(f_rect.left() - 12.0, f_rect.top() - 12.0),
                            Pos2::new(f_rect.right() + 12.0, f_rect.top() - 12.0),
                        ],
                        Stroke::new(1.0_f32, BORDER_SUBTLE),
                    );

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 7.0;

                        if ui
                            .add_sized(
                                [ui.available_width(), 32.0],
                                |ui: &mut egui::Ui| {
                                    glass_secondary_button(
                                        ui,
                                        Some(IconKind::FolderSearch),
                                        "Discover Games",
                                        true,
                                    )
                                },
                            )
                            .clicked()
                        {
                            actions.push(AppAction::OpenAddModal(Some(AddModalTab::Discover)));
                        }

                        let scan_label = if is_scanning_steam {
                            "Scanning Steam..."
                        } else {
                            "Scan Steam"
                        };
                        if ui
                            .add_sized(
                                [ui.available_width(), 32.0],
                                |ui: &mut egui::Ui| {
                                    glass_secondary_button(
                                        ui,
                                        Some(IconKind::Refresh),
                                        scan_label,
                                        !is_scanning_steam,
                                    )
                                },
                            )
                            .clicked()
                        {
                            actions.push(AppAction::ScanSteam);
                        }

                        if glass_pill_button(
                            ui,
                            Some(IconKind::Settings),
                            "Settings",
                            false,
                            None,
                            true,
                        )
                        .clicked()
                        {
                            actions.push(AppAction::OpenSettings);
                        }
                    });
                });

            // Scrollable navigation area
            ScrollArea::vertical()
                .id_salt("sidebar_nav_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    Frame::NONE
                        .inner_margin(Margin {
                            left: 12,
                            right: 12,
                            top: 16,
                            bottom: 14,
                        })
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;

                            // Section: Library
                            ui.horizontal(|ui| {
                                ui.add_space(6.0);
                                section_overline(ui, "Library");
                            });
                            ui.add_space(2.0);

                            if glass_pill_button(
                                ui,
                                Some(IconKind::Layers),
                                "All Games",
                                *active_filter == SidebarFilter::All,
                                Some(games.len()),
                                true,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::SelectFilter(SidebarFilter::All));
                            }

                            if glass_pill_button(
                                ui,
                                Some(IconKind::Heart { filled: false }),
                                "Favorites",
                                *active_filter == SidebarFilter::Favorites,
                                Some(favorites_count),
                                true,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::SelectFilter(SidebarFilter::Favorites));
                            }

                            if glass_pill_button(
                                ui,
                                Some(IconKind::Clock),
                                "Recently Played",
                                *active_filter == SidebarFilter::Recent,
                                Some(recent_count),
                                true,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::SelectFilter(SidebarFilter::Recent));
                            }

                            ui.add_space(16.0);

                            // Section: Platform
                            ui.horizontal(|ui| {
                                ui.add_space(6.0);
                                section_overline(ui, "Platform");
                            });
                            ui.add_space(2.0);

                            if glass_pill_button(
                                ui,
                                Some(IconKind::Gamepad),
                                "Steam",
                                *active_filter == SidebarFilter::Platform(Platform::Steam),
                                Some(steam_count),
                                true,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::SelectFilter(SidebarFilter::Platform(
                                    Platform::Steam,
                                )));
                            }

                            if glass_pill_button(
                                ui,
                                Some(IconKind::Folder),
                                "Manual",
                                *active_filter == SidebarFilter::Platform(Platform::Manual),
                                Some(manual_count),
                                true,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::SelectFilter(SidebarFilter::Platform(
                                    Platform::Manual,
                                )));
                            }

                            if glass_pill_button(
                                ui,
                                Some(IconKind::Cpu),
                                "Emulator",
                                *active_filter == SidebarFilter::Platform(Platform::Emulator),
                                Some(emulator_count),
                                true,
                            )
                            .clicked()
                            {
                                actions.push(AppAction::SelectFilter(SidebarFilter::Platform(
                                    Platform::Emulator,
                                )));
                            }

                            ui.add_space(16.0);

                            // Section: Tags
                            ui.horizontal(|ui| {
                                ui.add_space(6.0);
                                section_overline(ui, "Tags");
                            });
                            ui.add_space(2.0);

                            if tags.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new("No tags assigned yet")
                                            .size(11.5)
                                            .color(TEXT_DIM),
                                    );
                                });
                            } else {
                                for tag in tags {
                                    let count = games
                                        .iter()
                                        .filter(|g| {
                                            g.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
                                        })
                                        .count();
                                    let is_active = matches!(
                                        active_filter,
                                        SidebarFilter::Tag(active) if active.eq_ignore_ascii_case(tag)
                                    );
                                    if glass_pill_button(
                                        ui,
                                        Some(IconKind::Tag),
                                        tag,
                                        is_active,
                                        Some(count),
                                        true,
                                    )
                                    .clicked()
                                    {
                                        actions.push(AppAction::SelectFilter(SidebarFilter::Tag(
                                            tag.clone(),
                                        )));
                                    }
                                }
                            }
                        });
                });
        });
}
