use std::collections::HashMap;

use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect, ScrollArea,
    Sense, Stroke, StrokeKind, Vec2,
};

use crate::covers::cover_image_uri;
use crate::format::{format_playtime, platform_label};
use crate::models::{ActiveGameStatus, Game};
use crate::theme::{
    self, BG_GLASS_HOVER, BG_GLASS_SURFACE, BORDER_ACTIVE, BORDER_GLASS,
    BORDER_SUBTLE, BTN_PRIMARY_BG, BTN_PRIMARY_HOVER, BTN_PRIMARY_TEXT, EMERALD_BG,
    EMERALD_BORDER, EMERALD_DOT, EMERALD_TEXT, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::{glass_primary_button, glass_secondary_button, paint_cover_image};
use crate::ui::{AddModalTab, AppAction, SidebarFilter};

/// Responsive game grid including empty library and filter states.
pub fn show(
    ui: &mut egui::Ui,
    games: &[&Game],
    total_library_count: usize,
    filter: &SidebarFilter,
    running_games: &HashMap<String, ActiveGameStatus>,
    is_scanning_steam: bool,
    actions: &mut Vec<AppAction>,
) {
    if games.is_empty() {
        show_empty_state(
            ui,
            total_library_count,
            filter,
            is_scanning_steam,
            actions,
        );
        return;
    }

    ScrollArea::vertical()
        .id_salt("game_grid_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::same(22))
                .show(ui, |ui| {
                    let avail_w = ui.available_width().max(320.0);
                    let gap = 16.0;
                    let cols = ((avail_w + gap) / (186.0 + gap)).floor().max(2.0) as usize;
                    let card_w = ((avail_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32)
                        .floor()
                        .max(130.0);
                    let cover_h = (card_w * 1.5).round();
                    let footer_h = 56.0;
                    let card_h = cover_h + footer_h;

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = gap;
                        for row_games in games.chunks(cols) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for game in row_games {
                                    let is_running = running_games.contains_key(&game.id);
                                    show_game_card(
                                        ui,
                                        game,
                                        is_running,
                                        Vec2::new(card_w, card_h),
                                        cover_h,
                                        actions,
                                    );
                                }
                            });
                        }
                    });
                });
        });
}

fn show_empty_state(
    ui: &mut egui::Ui,
    total_library_count: usize,
    filter: &SidebarFilter,
    is_scanning_steam: bool,
    actions: &mut Vec<AppAction>,
) {
    let completely_empty = total_library_count == 0 && *filter == SidebarFilter::All;
    let avail = ui.available_size();

    ui.allocate_ui_with_layout(
        avail,
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            ui.set_max_width(580.0);
            ui.vertical_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                let p = ui.painter();
                p.rect(
                    icon_rect,
                    16.0,
                    BG_GLASS_SURFACE,
                    Stroke::new(1.0_f32, BORDER_GLASS),
                    StrokeKind::Inside,
                );
                theme::paint_glass_specular_top(p, icon_rect, 16.0);
                paint_icon(p, icon_rect.center(), 26.0, IconKind::Gamepad, TEXT_SECONDARY);

                ui.add_space(14.0);
                let title = if completely_empty {
                    "Your library is empty"
                } else {
                    "No games in this view"
                };
                ui.label(egui::RichText::new(title).size(17.0).strong().color(TEXT_PRIMARY));

                ui.add_space(6.0);
                let subtitle = if completely_empty {
                    "Automatically discover installed PC, Steam, Epic, and emulator games (Cemu, Dolphin, Ryujinx) or import a folder."
                } else {
                    "No games match the currently selected filter."
                };
                ui.label(egui::RichText::new(subtitle).size(13.0).color(TEXT_MUTED));

                if completely_empty {
                    ui.add_space(18.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::splat(10.0);
                        if glass_primary_button(
                            ui,
                            Some(IconKind::FolderSearch),
                            "Auto-Discover",
                            true,
                        )
                        .clicked()
                        {
                            actions.push(AppAction::OpenAddModal(Some(AddModalTab::Discover)));
                        }
                        if glass_secondary_button(
                            ui,
                            Some(IconKind::Cpu),
                            "Emulator (Cemu, …)",
                            true,
                        )
                        .clicked()
                        {
                            actions.push(AppAction::OpenAddModal(Some(AddModalTab::Emulator)));
                        }
                        if glass_secondary_button(
                            ui,
                            Some(IconKind::Refresh),
                            "Scan Steam",
                            !is_scanning_steam,
                        )
                        .clicked()
                        {
                            actions.push(AppAction::ScanSteam);
                        }
                        if glass_secondary_button(ui, Some(IconKind::Plus), "Manual", true)
                            .clicked()
                        {
                            actions.push(AppAction::OpenAddModal(Some(AddModalTab::Manual)));
                        }
                    });
                }
            });
        },
    );
}

fn show_game_card(
    ui: &mut egui::Ui,
    game: &Game,
    is_running: bool,
    card_size: Vec2,
    cover_h: f32,
    actions: &mut Vec<AppAction>,
) {
    let (base_rect, _) = ui.allocate_exact_size(card_size, Sense::hover());
    if !ui.is_rect_visible(base_rect) {
        return;
    }

    let pointer_in_card = ui.rect_contains_pointer(base_rect);
    let rect = base_rect;
    let cover_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), cover_h));
    let footer_rect = Rect::from_min_max(
        Pos2::new(rect.left(), cover_rect.bottom()),
        rect.max,
    );

    let radius = 14.0;
    let radius_u8 = radius as u8;
    let p = ui.painter();

    // Base glass card surface
    p.rect_filled(
        rect,
        radius,
        if pointer_in_card {
            BG_GLASS_HOVER
        } else {
            BG_GLASS_SURFACE
        },
    );

    // 2:3 Cover or monochrome fallback
    let cover_rounding = CornerRadius {
        nw: radius_u8,
        ne: radius_u8,
        sw: 0,
        se: 0,
    };
    let uri = cover_image_uri(game.cover_url.as_deref(), game.cover_is_local);
    let image_drawn = uri.as_deref().is_some_and(|u| {
        paint_cover_image(
            ui,
            u,
            cover_rect,
            cover_rounding,
            Color32::WHITE,
            1.0,
        )
    });

    if !image_drawn {
        p.rect_filled(cover_rect, cover_rounding, Color32::from_rgb(16, 16, 20));
        let top_glow = Rect::from_min_max(
            cover_rect.min,
            Pos2::new(cover_rect.max.x, cover_rect.top() + cover_rect.height() * 0.45),
        );
        p.rect_filled(top_glow, cover_rounding, theme::rgba(255, 255, 255, 10));
        let title_galley = p.layout(
            game.title.clone(),
            FontId::proportional(14.5),
            TEXT_PRIMARY,
            cover_rect.width() - 28.0,
        );
        let ty = (cover_rect.bottom() - 16.0 - title_galley.size().y).max(cover_rect.top() + 44.0);
        p.with_clip_rect(cover_rect.shrink(12.0)).galley(
            Pos2::new(cover_rect.left() + 14.0, ty),
            title_galley,
            TEXT_PRIMARY,
        );
    }

    // Top-left badge: RUNNING or platform
    if is_running {
        let label = "RUNNING";
        let g = p.layout_no_wrap(label.to_string(), FontId::monospace(10.0), EMERALD_TEXT);
        let badge_rect = Rect::from_min_size(
            Pos2::new(cover_rect.left() + 10.0, cover_rect.top() + 10.0),
            Vec2::new(g.size().x + 24.0, 21.0),
        );
        p.rect(
            badge_rect,
            10.5,
            EMERALD_BG,
            Stroke::new(1.0_f32, EMERALD_BORDER),
            StrokeKind::Inside,
        );
        p.circle_filled(
            Pos2::new(badge_rect.left() + 9.0, badge_rect.center().y),
            3.0,
            EMERALD_DOT,
        );
        p.galley(
            Pos2::new(badge_rect.left() + 16.0, badge_rect.center().y - g.size().y * 0.5),
            g,
            EMERALD_TEXT,
        );
    } else if !image_drawn || pointer_in_card {
        let label = platform_label(game.platform);
        let g = p.layout_no_wrap(label.to_string(), FontId::monospace(10.0), TEXT_SECONDARY);
        let badge_rect = Rect::from_min_size(
            Pos2::new(cover_rect.left() + 10.0, cover_rect.top() + 10.0),
            Vec2::new(g.size().x + 14.0, 20.0),
        );
        p.rect(
            badge_rect,
            6.0,
            theme::rgba(12, 12, 16, 205),
            Stroke::new(1.0_f32, BORDER_GLASS),
            StrokeKind::Inside,
        );
        p.galley(badge_rect.center() - g.size() * 0.5, g, TEXT_SECONDARY);
    }

    // Footer (Title + Playtime + first tag)
    p.line_segment(
        [
            Pos2::new(footer_rect.left() + 1.0, footer_rect.top()),
            Pos2::new(footer_rect.right() - 1.0, footer_rect.top()),
        ],
        Stroke::new(1.0_f32, BORDER_SUBTLE),
    );
    let footer_bg_rounding = CornerRadius {
        nw: 0,
        ne: 0,
        sw: radius_u8,
        se: radius_u8,
    };
    p.rect_filled(
        footer_rect,
        footer_bg_rounding,
        theme::rgba(11, 11, 15, 220),
    );

    let title_galley = p.layout_no_wrap(
        game.title.clone(),
        FontId::proportional(13.0),
        TEXT_PRIMARY,
    );
    let title_clip = Rect::from_min_max(
        Pos2::new(footer_rect.left() + 11.0, footer_rect.top() + 8.0),
        Pos2::new(footer_rect.right() - 11.0, footer_rect.top() + 28.0),
    );
    p.with_clip_rect(title_clip).galley(
        Pos2::new(footer_rect.left() + 11.0, footer_rect.top() + 9.0),
        title_galley,
        TEXT_PRIMARY,
    );

    let playtime_str = format_playtime(game.playtime_seconds);
    let pt_galley = p.layout_no_wrap(playtime_str, FontId::monospace(10.5), TEXT_DIM);

    let mut tag_right_limit = footer_rect.right() - 11.0;
    if let Some(first_tag) = game.tags.first() {
        let tg = p.layout_no_wrap(first_tag.clone(), FontId::proportional(10.0), TEXT_MUTED);
        let tw = (tg.size().x + 12.0).min(footer_rect.width() * 0.44);
        let tag_rect = Rect::from_min_size(
            Pos2::new(footer_rect.right() - 11.0 - tw, footer_rect.bottom() - 23.0),
            Vec2::new(tw, 17.0),
        );
        p.rect(
            tag_rect,
            5.0,
            theme::rgba(255, 255, 255, 10),
            Stroke::new(1.0_f32, BORDER_SUBTLE),
            StrokeKind::Inside,
        );
        p.with_clip_rect(tag_rect.shrink2(Vec2::new(4.0, 0.0)))
            .galley(
                Pos2::new(tag_rect.left() + 6.0, tag_rect.center().y - tg.size().y * 0.5),
                tg,
                TEXT_MUTED,
            );
        tag_right_limit = tag_rect.left() - 6.0;
    }

    let pt_clip = Rect::from_min_max(
        Pos2::new(footer_rect.left() + 11.0, footer_rect.bottom() - 24.0),
        Pos2::new(
            tag_right_limit.max(footer_rect.left() + 20.0),
            footer_rect.bottom() - 4.0,
        ),
    );
    p.with_clip_rect(pt_clip).galley(
        Pos2::new(footer_rect.left() + 11.0, footer_rect.bottom() - 21.0),
        pt_galley,
        TEXT_DIM,
    );

    // Subtle outer border
    p.rect_stroke(
        rect,
        radius,
        Stroke::new(
            1.0_f32,
            if pointer_in_card {
                theme::rgba(255, 255, 255, 34)
            } else {
                theme::rgba(255, 255, 255, 16)
            },
        ),
        StrokeKind::Inside,
    );

    // Register base card interaction FIRST so overlay buttons placed afterwards take hit-test priority
    let card_resp = ui
        .interact(base_rect, Id::new(("card_open", &game.id)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);

    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    let mut overlay_clicked = false;
    let mut pointer_in_overlay = false;

    if game.is_favorite || pointer_in_card {
        let fav_rect = Rect::from_center_size(
            Pos2::new(cover_rect.right() - 23.0, cover_rect.top() + 23.0),
            Vec2::splat(28.0),
        );
        if pointer_pos.is_some_and(|pos| fav_rect.contains(pos)) {
            pointer_in_overlay = true;
        }
        let fav_resp = ui
            .interact(fav_rect, Id::new(("card_fav", &game.id)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        let fav_hov = fav_resp.hovered();

        let fav_bg = if game.is_favorite {
            theme::rgba(14, 14, 18, 225)
        } else if fav_hov {
            theme::rgba(28, 28, 36, 230)
        } else {
            theme::rgba(14, 14, 18, 190)
        };
        p.circle_filled(fav_rect.center(), 14.0, fav_bg);
        p.circle_stroke(
            fav_rect.center(),
            14.0,
            Stroke::new(
                1.0_f32,
                if game.is_favorite || fav_hov {
                    BORDER_ACTIVE
                } else {
                    BORDER_GLASS
                },
            ),
        );
        paint_icon(
            p,
            fav_rect.center(),
            13.0,
            IconKind::Heart {
                filled: game.is_favorite,
            },
            if game.is_favorite || fav_hov {
                Color32::WHITE
            } else {
                TEXT_MUTED
            },
        );
        if fav_resp.clicked() {
            overlay_clicked = true;
            actions.push(AppAction::ToggleFavorite(game.id.clone()));
        }
    }

    if pointer_in_card {
        let btn_label = if is_running { "Running" } else { "Play" };
        let lg = p.layout_no_wrap(
            btn_label.to_string(),
            FontId::proportional(11.5),
            BTN_PRIMARY_TEXT,
        );
        let btn_size = Vec2::new(lg.size().x + 32.0, 28.0);
        let launch_rect = Rect::from_min_size(
            Pos2::new(
                cover_rect.right() - 10.0 - btn_size.x,
                cover_rect.bottom() - 10.0 - btn_size.y,
            ),
            btn_size,
        );
        if pointer_pos.is_some_and(|pos| launch_rect.contains(pos)) {
            pointer_in_overlay = true;
        }
        let launch_resp = ui.interact(
            launch_rect,
            Id::new(("card_launch", &game.id)),
            if is_running { Sense::hover() } else { Sense::click() },
        );
        let launch_resp = if is_running {
            launch_resp
        } else {
            launch_resp.on_hover_cursor(CursorIcon::PointingHand)
        };
        let l_hov = !is_running && launch_resp.hovered();

        let bg = if is_running {
            theme::rgba(150, 150, 158, 130)
        } else if l_hov {
            BTN_PRIMARY_HOVER
        } else {
            BTN_PRIMARY_BG
        };
        p.rect(
            launch_rect,
            9.0,
            bg,
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 140)),
            StrokeKind::Inside,
        );
        paint_icon(
            p,
            Pos2::new(launch_rect.left() + 14.0, launch_rect.center().y),
            11.0,
            IconKind::Play,
            BTN_PRIMARY_TEXT,
        );
        p.galley(
            Pos2::new(
                launch_rect.left() + 24.0,
                launch_rect.center().y - lg.size().y * 0.5,
            ),
            lg,
            BTN_PRIMARY_TEXT,
        );
        if launch_resp.clicked() && !is_running {
            overlay_clicked = true;
            actions.push(AppAction::LaunchGame(game.id.clone()));
        }
    }

    if card_resp.clicked() && !overlay_clicked && !pointer_in_overlay {
        actions.push(AppAction::OpenDetailModal(game.id.clone()));
    }
}
