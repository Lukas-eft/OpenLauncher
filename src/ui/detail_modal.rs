use std::time::Duration;

use chrono::{DateTime, Utc};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Id, Margin, Pos2, Rect, ScrollArea,
    Sense, Stroke, StrokeKind, Vec2,
};

use crate::covers::cover_image_uri;
use crate::format::{
    format_date_time, format_duration_exact, format_playtime, format_relative_date, platform_label,
};
use crate::models::{ActiveGameStatus, Game, PlaySession};
use crate::theme::{
    self, BG_GLASS_SURFACE, BORDER_GLASS, BORDER_SUBTLE, EMERALD_BG, EMERALD_BORDER, EMERALD_DOT,
    EMERALD_TEXT, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons::IconKind;
use crate::ui::widgets::{
    glass_danger_button, glass_icon_button, glass_modal_frame, glass_pill_button,
    glass_primary_button, glass_secondary_button, glass_well_frame, paint_cover_image,
    paint_modal_backdrop_order, section_overline, status_badge,
};
use crate::ui::AppAction;

/// Zustand des Detail-Modals für ein ausgewähltes Spiel.
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

/// Zeigt das Detail-Modal mit Hero-Banner, Telemetrie-KPIs, Launch-Konfiguration und Session-Verlauf.
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

    if paint_modal_backdrop_order(ctx, "detail_modal_backdrop", egui::Order::Middle)
        && allow_escape_close
    {
        actions.push(AppAction::CloseDetailModal);
        return;
    }

    if running_status.is_some() {
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 48.0).clamp(480.0, 760.0);
    let max_h = (screen.height() - 48.0).max(400.0);

    egui::Area::new(Id::new("detail_modal_panel"))
        .order(egui::Order::Middle)
        .movable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(modal_w);
            ui.set_max_height(max_h);

            glass_modal_frame().show(ui, |ui| {
                ui.set_width(modal_w);
                ui.set_max_height(max_h);

                ScrollArea::vertical()
                    .id_salt("detail_modal_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // Hero-Banner
                        let (hero_rect, _) =
                            ui.allocate_exact_size(Vec2::new(modal_w, 178.0), Sense::hover());
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
                                theme::rgba(255, 255, 255, 175),
                                1.0,
                            )
                        }) || cover_uri.as_deref().is_some_and(|c| {
                            paint_cover_image(
                                ui,
                                c,
                                hero_rect,
                                hero_rounding,
                                theme::rgba(255, 255, 255, 90),
                                1.08,
                            )
                        });

                        if !hero_drawn {
                            theme::paint_monochrome_gradient(ui.painter(), hero_rect, 16.0);
                        }

                        // Dunkler Verlaufsübergang ins Elevated-Panel
                        let fade_rect = Rect::from_min_max(
                            Pos2::new(hero_rect.left(), hero_rect.top() + 54.0),
                            hero_rect.max,
                        );
                        ui.painter().rect_filled(
                            fade_rect,
                            CornerRadius::ZERO,
                            theme::rgba(20, 20, 25, 185),
                        );

                        // Schließen-Button im Hero oben rechts
                        let close_rect = Rect::from_center_size(
                            Pos2::new(hero_rect.right() - 24.0, hero_rect.top() + 24.0),
                            Vec2::splat(30.0),
                        );
                        let close_resp = ui
                            .allocate_new_ui(egui::UiBuilder::new().max_rect(close_rect), |ui| {
                                glass_icon_button(
                                    ui,
                                    "detail_close_btn",
                                    IconKind::Close,
                                    30.0,
                                    false,
                                    false,
                                )
                            })
                            .inner;
                        if close_resp.clicked() {
                            actions.push(AppAction::CloseDetailModal);
                        }

                        // Inhalt unterhalb des Hero-Banners
                        Frame::NONE
                            .inner_margin(Margin {
                                left: 24,
                                right: 24,
                                top: 0,
                                bottom: 24,
                            })
                            .show(ui, |ui| {
                                // Kopfzeile mit 2:3-Cover + Titel + Aktionen
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 18.0;

                                    // 2:3 Cover-Vorschau
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

                                    // Rechte Kopf-Spalte
                                    ui.vertical(|ui| {
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
                                                    "LÄUFT · {}",
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

                                        ui.label(
                                            egui::RichText::new(&game.title)
                                                .size(22.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );

                                        ui.add_space(2.0);
                                        ui.horizontal_wrapped(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                                            let is_running = running_status.is_some();
                                            let play_lbl = if is_running {
                                                "Wird ausgeführt..."
                                            } else {
                                                "Spiel starten"
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

                                            let fav_lbl = if game.is_favorite {
                                                "Favorit"
                                            } else {
                                                "Favorisieren"
                                            };
                                            if glass_pill_button(
                                                ui,
                                                Some(IconKind::Heart {
                                                    filled: game.is_favorite,
                                                }),
                                                fav_lbl,
                                                game.is_favorite,
                                                None,
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
                                                "Bearbeiten",
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
                                                    "Löschen",
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
                                                    "Wirklich löschen",
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
                                                    "Abbrechen",
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

                                // 3 Monochrom-Telemetrie-KPI-Karten
                                ui.add_space(16.0);
                                let card_w = ((ui.available_width() - 20.0) / 3.0).max(140.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    kpi_card(
                                        ui,
                                        card_w,
                                        IconKind::Clock,
                                        "Gesamtspielzeit",
                                        if running_status.is_some() {
                                            "AKTIV"
                                        } else {
                                            "TOTAL"
                                        },
                                        &format_playtime(game.playtime_seconds),
                                    );
                                    kpi_card(
                                        ui,
                                        card_w,
                                        IconKind::Refresh,
                                        "Zuletzt gespielt",
                                        "TIMELINE",
                                        &format_relative_date(game.last_played_at.as_deref()),
                                    );
                                    kpi_card(
                                        ui,
                                        card_w,
                                        IconKind::Play,
                                        "Sessions",
                                        "SQLITE",
                                        &state.sessions.len().to_string(),
                                    );
                                });

                                // Launch-Konfiguration
                                ui.add_space(16.0);
                                glass_well_frame().show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (r, _) =
                                            ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                        crate::ui::icons::paint_icon(
                                            ui.painter(),
                                            r.center(),
                                            13.0,
                                            IconKind::Terminal,
                                            TEXT_SECONDARY,
                                        );
                                        section_overline(ui, "Launch-Konfiguration");
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
                                                "Direktstart über Steam-Client (-applaunch {app_id})"
                                            )
                                        } else {
                                            "Direktstart über Systemstarter".to_string()
                                        };
                                        ui.label(
                                            egui::RichText::new(fallback)
                                                .size(12.0)
                                                .color(TEXT_DIM),
                                        );
                                    } else {
                                        if let Some(ref exe) = game.executable_path {
                                            detail_kv_row(ui, "Ausführbare Datei", exe);
                                        }
                                        if let Some(ref dir) = game.install_dir {
                                            detail_kv_row(ui, "Installationspfad", dir);
                                        }
                                        if let Some(ref wd) = game.working_dir {
                                            detail_kv_row(ui, "Arbeitsverzeichnis", wd);
                                        }
                                        if let Some(ref args) = game.launch_args {
                                            detail_kv_row(ui, "Start-Argumente", args);
                                        }
                                    }
                                });

                                // Session-Verlauf
                                ui.add_space(14.0);
                                glass_well_frame().show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    section_overline(ui, "Letzte Sessions");
                                    ui.add_space(8.0);

                                    if state.loading_sessions {
                                        ui.label(
                                            egui::RichText::new("Lade Sessions...")
                                                .size(12.0)
                                                .color(TEXT_DIM),
                                        );
                                    } else if state.sessions.is_empty() {
                                        ui.label(
                                            egui::RichText::new(
                                                "Noch keine aufgezeichneten Spielsessions vorhanden.",
                                            )
                                            .size(12.0)
                                            .color(TEXT_DIM),
                                        );
                                    } else {
                                        ScrollArea::vertical()
                                            .id_salt("detail_sessions_scroll")
                                            .max_height(165.0)
                                            .show(ui, |ui| {
                                                ui.spacing_mut().item_spacing.y = 5.0;
                                                for session in &state.sessions {
                                                    Frame::NONE
                                                        .fill(theme::rgba(255, 255, 255, 7))
                                                        .stroke(Stroke::new(
                                                            1.0_f32,
                                                            BORDER_SUBTLE,
                                                        ))
                                                        .corner_radius(8.0)
                                                        .inner_margin(Margin::symmetric(10, 7))
                                                        .show(ui, |ui| {
                                                            ui.set_width(ui.available_width());
                                                            ui.horizontal(|ui| {
                                                                ui.label(
                                                                    egui::RichText::new(
                                                                        format_date_time(
                                                                            &session.started_at,
                                                                        ),
                                                                    )
                                                                    .monospace()
                                                                    .size(11.5)
                                                                    .color(TEXT_SECONDARY),
                                                                );
                                                                ui.with_layout(
                                                                    egui::Layout::right_to_left(
                                                                        egui::Align::Center,
                                                                    ),
                                                                    |ui| {
                                                                        let dur = if session
                                                                            .ended_at
                                                                            .is_some()
                                                                        {
                                                                            format_duration_exact(
                                                                                session
                                                                                    .duration_seconds,
                                                                            )
                                                                        } else {
                                                                            "Aktiv".to_string()
                                                                        };
                                                                        ui.label(
                                                                            egui::RichText::new(
                                                                                dur,
                                                                            )
                                                                            .monospace()
                                                                            .size(11.5)
                                                                            .color(TEXT_PRIMARY),
                                                                        );
                                                                    },
                                                                );
                                                            });
                                                        });
                                                }
                                            });
                                    }
                                });
                            });
                    });
            });
        });
}

fn kpi_card(
    ui: &mut egui::Ui,
    width: f32,
    icon: IconKind,
    label: &str,
    tag: &str,
    value: &str,
) {
    Frame::NONE
        .fill(BG_GLASS_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
        .corner_radius(12.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width - 26.0);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
                crate::ui::icons::paint_icon(
                    ui.painter(),
                    r.center(),
                    12.5,
                    icon,
                    TEXT_SECONDARY,
                );
                ui.label(egui::RichText::new(label).size(11.5).color(TEXT_MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(
                        ui,
                        tag,
                        theme::rgba(255, 255, 255, 10),
                        BORDER_SUBTLE,
                        TEXT_MUTED,
                        None,
                    );
                });
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(value)
                    .monospace()
                    .size(16.5)
                    .strong()
                    .color(Color32::WHITE),
            );
        });
}

fn detail_kv_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(136.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.label(egui::RichText::new(key).size(11.5).color(TEXT_DIM));
            },
        );
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
}
