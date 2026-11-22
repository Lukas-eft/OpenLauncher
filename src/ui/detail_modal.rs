use std::time::Duration;

use chrono::{DateTime, Utc};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect,
    ScrollArea, Sense, Stroke, StrokeKind, Vec2,
};

use crate::covers::cover_image_uri;
use crate::format::{
    format_duration_exact, format_playtime, format_relative_date, platform_label,
};
use crate::models::{ActiveGameStatus, Game, PlaySession};
use crate::theme::{
    self, BG_GLASS_SURFACE, BORDER_GLASS, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
    EMERALD_BG, EMERALD_BORDER, EMERALD_DOT, EMERALD_TEXT,
};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::{
    glass_danger_button, glass_icon_button, glass_modal_frame, glass_primary_button,
    glass_secondary_button, glass_well_frame, paint_cover_image, paint_modal_backdrop,
    section_overline, status_badge,
};
use crate::ui::AppAction;

/// State of the game detail modal.
#[derive(Debug, Clone)]
pub struct DetailModalState {
    pub game_id: String,
    pub confirm_delete: bool,
    pub sessions: Vec<PlaySession>,
    pub loading_sessions: bool,
}

impl DetailModalState {
    pub fn new(game_id: String) -> Self {
        Self {
            game_id,
            confirm_delete: false,
            sessions: Vec::new(),
            loading_sessions: true,
        }
    }
}

/// Shows the game detail modal with hero banner, telemetry KPIs, and launch configuration.
pub fn show(
    ctx: &egui::Context,
    state: &mut DetailModalState,
    game: &Game,
    running_status: Option<&ActiveGameStatus>,
    allow_escape_close: bool,
    actions: &mut Vec<AppAction>,
) {
    if allow_escape_close && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        actions.push(AppAction::CloseDetailModal);
        return;
    }

    if paint_modal_backdrop(ctx, "detail_modal_backdrop") && allow_escape_close {
        actions.push(AppAction::CloseDetailModal);
        return;
    }

    if running_status.is_some() {
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 48.0).clamp(480.0, 740.0);
    let max_h = (screen.height() - 48.0).max(380.0);

    egui::Area::new(Id::new("detail_modal_panel"))
        .order(egui::Order::Foreground)
        .movable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(modal_w);
            ui.set_max_height(max_h);

            glass_modal_frame().show(ui, |ui| {
                ui.set_width(modal_w);
                ui.set_max_width(modal_w);
                ui.set_max_height(max_h);

                ScrollArea::vertical()
                    .id_salt("detail_modal_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.set_width(modal_w);
                        ui.set_max_width(modal_w);

                        // Hero banner spanning 100% of modal width
                        let (hero_rect, _) =
                            ui.allocate_exact_size(Vec2::new(modal_w, 156.0), Sense::hover());
                        let hero_rounding = CornerRadius {
                            nw: 16,
                            ne: 16,
                            sw: 0,
                            se: 0,
                        };
                        let cover_uri =
                            cover_image_uri(game.cover_url.as_deref(), game.cover_is_local);

                        let hero_drawn = game.hero_url.as_deref().is_some_and(|h| {
                            paint_cover_image(
                                ui,
                                h,
                                hero_rect,
                                hero_rounding,
                                theme::rgba(255, 255, 255, 195),
                                1.0,
                            )
                        }) || cover_uri.as_deref().is_some_and(|c| {
                            paint_cover_image(
                                ui,
                                c,
                                hero_rect,
                                hero_rounding,
                                theme::rgba(255, 255, 255, 110),
                                1.08,
                            )
                        });

                        if !hero_drawn {
                            theme::paint_monochrome_gradient(ui.painter(), hero_rect, 16.0);
                        }

                        // Subtle bottom fade into elevated modal surface
                        let fade_rect = Rect::from_min_max(
                            Pos2::new(hero_rect.left(), hero_rect.bottom() - 48.0),
                            hero_rect.max,
                        );
                        ui.painter().rect_filled(
                            fade_rect,
                            CornerRadius::ZERO,
                            theme::rgba(18, 18, 22, 200),
                        );

                        // Close button in the exact top-right corner of the modal
                        let close_rect = Rect::from_center_size(
                            Pos2::new(hero_rect.right() - 22.0, hero_rect.top() + 22.0),
                            Vec2::splat(30.0),
                        );
                        let close_resp = ui
                            .interact(close_rect, Id::new("detail_close_btn"), Sense::click())
                            .on_hover_cursor(CursorIcon::PointingHand);
                        let close_hov = close_resp.hovered();
                        let p = ui.painter();
                        p.circle_filled(
                            close_rect.center(),
                            15.0,
                            if close_hov {
                                theme::rgba(32, 32, 40, 235)
                            } else {
                                theme::rgba(14, 14, 18, 205)
                            },
                        );
                        p.circle_stroke(
                            close_rect.center(),
                            15.0,
                            Stroke::new(
                                1.0_f32,
                                if close_hov {
                                    theme::BORDER_HOVER
                                } else {
                                    BORDER_GLASS
                                },
                            ),
                        );
                        paint_icon(
                            p,
                            close_rect.center(),
                            14.0,
                            IconKind::Close,
                            if close_hov { Color32::WHITE } else { TEXT_SECONDARY },
                        );
                        if close_resp.clicked() {
                            actions.push(AppAction::CloseDetailModal);
                        }

                        let content_w = (modal_w - 48.0).max(200.0);

                        // Content cleanly below the hero banner
                        Frame::NONE
                            .inner_margin(Margin {
                                left: 24,
                                right: 24,
                                top: 18,
                                bottom: 24,
                            })
                            .show(ui, |ui| {
                                ui.set_width(content_w);
                                ui.set_max_width(content_w);

                                // Header row: 2:3 cover + title + actions
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 18.0;

                                    // 2:3 Cover preview
                                    let (c_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(108.0, 156.0),
                                        Sense::hover(),
                                    );
                                    let drew_cover = cover_uri.as_deref().is_some_and(|uri| {
                                        paint_cover_image(
                                            ui,
                                            uri,
                                            c_rect,
                                            11,
                                            Color32::WHITE,
                                            1.0,
                                        )
                                    });
                                    if !drew_cover {
                                        theme::paint_monochrome_gradient(
                                            ui.painter(),
                                            c_rect,
                                            11.0,
                                        );
                                        let g = ui.painter().layout(
                                            game.title.clone(),
                                            FontId::proportional(12.5),
                                            TEXT_PRIMARY,
                                            c_rect.width() - 20.0,
                                        );
                                        ui.painter().galley(
                                            Pos2::new(
                                                c_rect.left() + 10.0,
                                                c_rect.bottom() - 12.0 - g.size().y,
                                            ),
                                            g,
                                            TEXT_PRIMARY,
                                        );
                                    }
                                    ui.painter().rect_stroke(
                                        c_rect,
                                        11.0,
                                        Stroke::new(1.0_f32, theme::BORDER_HOVER),
                                        StrokeKind::Inside,
                                    );

                                    // Right header column
                                    let right_w = (content_w - 126.0).max(160.0);
                                    ui.vertical(|ui| {
                                        ui.set_max_width(right_w);
                                        ui.spacing_mut().item_spacing.y = 8.0;

                                        ui.horizontal_wrapped(|ui| {
                                            status_badge(
                                                ui,
                                                platform_label(game.platform),
                                                theme::rgba(255, 255, 255, 12),
                                                BORDER_GLASS,
                                                TEXT_SECONDARY,
                                                None,
                                            );
                                            if let Some(ref app_id) = game.steam_app_id {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "AppID {app_id}"
                                                    ))
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(TEXT_DIM),
                                                );
                                            }
                                            if let Some(status) = running_status {
                                                let elapsed = DateTime::parse_from_rfc3339(
                                                    &status.started_at,
                                                )
                                                .map_or(0, |dt| {
                                                    (Utc::now() - dt.with_timezone(&Utc))
                                                        .num_seconds()
                                                        .max(0)
                                                });
                                                let lbl = format!(
                                                    "RUNNING · {}",
                                                    format_duration_exact(elapsed)
                                                );
                                                status_badge(
                                                    ui,
                                                    &lbl,
                                                    EMERALD_BG,
                                                    EMERALD_BORDER,
                                                    EMERALD_TEXT,
                                                    Some(EMERALD_DOT),
                                                );
                                            }
                                        });

                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&game.title)
                                                    .size(22.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            )
                                            .wrap(),
                                        );

                                        ui.add_space(2.0);
                                        ui.horizontal_wrapped(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                                            let is_running = running_status.is_some();
                                            let play_lbl = if is_running {
                                                "Running..."
                                            } else {
                                                "Launch Game"
                                            };
                                            if glass_primary_button(
                                                ui,
                                                Some(IconKind::Play),
                                                play_lbl,
                                                !is_running,
                                            )
                                            .clicked()
                                            {
                                                actions.push(AppAction::LaunchGame(
                                                    game.id.clone(),
                                                ));
                                            }

                                            if glass_icon_button(
                                                ui,
                                                "detail_fav_btn",
                                                IconKind::Heart {
                                                    filled: game.is_favorite,
                                                },
                                                32.0,
                                                game.is_favorite,
                                                false,
                                            )
                                            .clicked()
                                            {
                                                actions.push(AppAction::ToggleFavorite(
                                                    game.id.clone(),
                                                ));
                                            }

                                            if glass_secondary_button(
                                                ui,
                                                Some(IconKind::Edit),
                                                "Edit",
                                                true,
                                            )
                                            .clicked()
                                            {
                                                actions
                                                    .push(AppAction::OpenEditModal(game.clone()));
                                            }

                                            if !state.confirm_delete {
                                                if glass_secondary_button(
                                                    ui,
                                                    Some(IconKind::Trash),
                                                    "Delete",
                                                    !is_running,
                                                )
                                                .clicked()
                                                {
                                                    state.confirm_delete = true;
                                                }
                                            } else {
                                                if glass_danger_button(
                                                    ui,
                                                    Some(IconKind::Trash),
                                                    "Confirm Delete",
                                                    true,
                                                )
                                                .clicked()
                                                {
                                                    actions.push(AppAction::DeleteGame(
                                                        game.id.clone(),
                                                    ));
                                                }
                                                if glass_secondary_button(
                                                    ui,
                                                    None,
                                                    "Cancel",
                                                    true,
                                                )
                                                .clicked()
                                                {
                                                    state.confirm_delete = false;
                                                }
                                            }
                                        });
                                    });
                                });

                                // Tags
                                if !game.tags.is_empty() {
                                    ui.add_space(14.0);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                                        for tag in &game.tags {
                                            status_badge(
                                                ui,
                                                &format!("# {tag}"),
                                                theme::rgba(255, 255, 255, 11),
                                                BORDER_GLASS,
                                                TEXT_SECONDARY,
                                                None,
                                            );
                                        }
                                    });
                                }

                                // 3 Monochrome Telemetry KPI Cards (strictly sized to content_w)
                                ui.add_space(16.0);
                                let card_w = ((content_w - 20.0) / 3.0).floor().max(120.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    kpi_card(
                                        ui,
                                        card_w,
                                        IconKind::Clock,
                                        "Total Playtime",
                                        &format_playtime(game.playtime_seconds),
                                    );
                                    kpi_card(
                                        ui,
                                        card_w,
                                        IconKind::Refresh,
                                        "Last Played",
                                        &format_relative_date(game.last_played_at.as_deref()),
                                    );
                                    kpi_card(
                                        ui,
                                        card_w,
                                        IconKind::Play,
                                        "Sessions",
                                        &state.sessions.len().to_string(),
                                    );
                                });

                                // Launch Configuration
                                ui.add_space(16.0);
                                glass_well_frame().show(ui, |ui| {
                                    let inner_w = (content_w - 30.0).max(180.0);
                                    ui.set_width(inner_w);
                                    ui.set_max_width(inner_w);

                                    ui.horizontal(|ui| {
                                        let (r, _) =
                                            ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                        paint_icon(
                                            ui.painter(),
                                            r.center(),
                                            13.0,
                                            IconKind::Terminal,
                                            TEXT_SECONDARY,
                                        );
                                        section_overline(ui, "Launch Configuration");
                                    });
                                    ui.add_space(8.0);

                                    let has_any = game.executable_path.is_some()
                                        || game.install_dir.is_some()
                                        || game.working_dir.is_some()
                                        || game.launch_args.is_some();

                                    if !has_any {
                                        let fallback = if let Some(ref app_id) = game.steam_app_id
                                        {
                                            format!(
                                                "Direct launch via Steam client (-applaunch {app_id})"
                                            )
                                        } else {
                                            "Direct launch via system launcher".to_string()
                                        };
                                        ui.label(
                                            egui::RichText::new(fallback)
                                                .size(12.0)
                                                .color(TEXT_DIM),
                                        );
                                    } else {
                                        if let Some(ref exe) = game.executable_path {
                                            detail_kv_row(ui, inner_w, "Executable", exe);
                                        }
                                        if let Some(ref dir) = game.install_dir {
                                            detail_kv_row(ui, inner_w, "Install Directory", dir);
                                        }
                                        if let Some(ref wd) = game.working_dir {
                                            detail_kv_row(ui, inner_w, "Working Directory", wd);
                                        }
                                        if let Some(ref args) = game.launch_args {
                                            detail_kv_row(ui, inner_w, "Launch Arguments", args);
                                        }
                                    }
                                });
                            });
                    });
            });
        });
}

fn kpi_card(ui: &mut egui::Ui, width: f32, icon: IconKind, label: &str, value: &str) {
    let inner_w = (width - 26.0).max(80.0);
    Frame::NONE
        .fill(BG_GLASS_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
        .corner_radius(12.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(inner_w);
            ui.set_max_width(inner_w);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
                    paint_icon(ui.painter(), r.center(), 12.5, icon, TEXT_SECONDARY);
                    ui.label(egui::RichText::new(label).size(11.5).color(TEXT_MUTED));
                });
                ui.label(
                    egui::RichText::new(value)
                        .monospace()
                        .size(16.5)
                        .strong()
                        .color(Color32::WHITE),
                );
            });
        });
}

fn detail_kv_row(ui: &mut egui::Ui, max_w: f32, key: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(132.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.label(egui::RichText::new(key).size(11.5).color(TEXT_DIM));
            },
        );
        ui.vertical(|ui| {
            ui.set_max_width((max_w - 140.0).max(100.0));
            ui.add(
                egui::Label::new(
                    egui::RichText::new(value)
                        .monospace()
                        .size(11.5)
                        .color(TEXT_SECONDARY),
                )
                .wrap(),
            );
        });
    });
}
