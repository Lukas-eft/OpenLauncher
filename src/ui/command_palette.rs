use std::collections::HashMap;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect,
    ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Vec2,
};

use crate::covers::cover_image_uri;
use crate::format::{format_playtime, platform_label};
use crate::fuzzy::search_games;
use crate::models::{ActiveGameStatus, Game};
use crate::theme::{
    self, BG_GLASS_WELL, BG_PILL_ACTIVE, BORDER_ACTIVE, BORDER_GLASS, BORDER_SUBTLE, EMERALD_BG,
    EMERALD_BORDER, EMERALD_TEXT, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::{
    glass_modal_frame, paint_cover_image, paint_kbd_chip, paint_modal_backdrop,
};
use crate::ui::AppAction;

#[derive(Debug, Default)]
pub struct CommandPaletteState {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
    pub focus_requested: bool,
}

impl CommandPaletteState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.focus_requested = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.selected_index = 0;
        self.focus_requested = false;
    }
}

pub fn show(
    ctx: &egui::Context,
    state: &mut CommandPaletteState,
    games: &[Game],
    running_games: &HashMap<String, ActiveGameStatus>,
    actions: &mut Vec<AppAction>,
) {
    if !state.is_open {
        return;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        actions.push(AppAction::ClosePalette);
        return;
    }

    if paint_modal_backdrop(ctx, "command_palette_backdrop") {
        actions.push(AppAction::ClosePalette);
        return;
    }

    let results: Vec<&Game> = search_games(games, &state.query).into_iter().take(12).collect();
    if state.selected_index >= results.len() {
        state.selected_index = 0;
    }

    let (arrow_down, arrow_up, enter_pressed, shift_held) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::Enter),
            i.modifiers.shift,
        )
    });

    if !results.is_empty() {
        if arrow_down {
            state.selected_index = (state.selected_index + 1) % results.len();
        } else if arrow_up {
            state.selected_index = (state.selected_index + results.len() - 1) % results.len();
        } else if enter_pressed {
            if let Some(selected) = results.get(state.selected_index) {
                if shift_held || running_games.contains_key(&selected.id) {
                    actions.push(AppAction::OpenDetailModal(selected.id.clone()));
                } else {
                    actions.push(AppAction::LaunchGame(selected.id.clone()));
                }
                actions.push(AppAction::ClosePalette);
                return;
            }
        }
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 48.0).clamp(380.0, 560.0);

    egui::Area::new(Id::new("command_palette_panel"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, screen.height() * 0.12))
        .show(ctx, |ui| {
            ui.set_max_width(modal_w);

            glass_modal_frame().show(ui, |ui| {
                ui.set_width(modal_w);

                Frame::NONE
                    .inner_margin(Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (sr, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                            paint_icon(
                                ui.painter(),
                                sr.center(),
                                15.0,
                                IconKind::Search,
                                crate::theme::TEXT_SECONDARY,
                            );

                            let input_w = (ui.available_width() - 44.0).max(160.0);
                            let resp = ui.add_sized(
                                [input_w, 24.0],
                                TextEdit::singleline(&mut state.query)
                                    .frame(false)
                                    .hint_text("Type game name, tag, or platform...")
                                    .font(FontId::proportional(14.0))
                                    .text_color(TEXT_PRIMARY),
                            );
                            if state.focus_requested {
                                resp.request_focus();
                                state.focus_requested = false;
                            }
                            if resp.changed() {
                                state.selected_index = 0;
                            }

                            paint_kbd_chip(ui, "ESC");
                        });
                    });

                ui.painter().line_segment(
                    [
                        Pos2::new(ui.min_rect().left(), ui.cursor().top()),
                        Pos2::new(ui.min_rect().left() + modal_w, ui.cursor().top()),
                    ],
                    Stroke::new(1.0_f32, BORDER_SUBTLE),
                );

                ScrollArea::vertical()
                    .id_salt("palette_results_scroll")
                    .max_height(330.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        Frame::NONE
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 3.0;

                                if results.is_empty() {
                                    ui.add_space(28.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new("No matching games found.")
                                                .size(12.5)
                                                .color(TEXT_DIM),
                                        );
                                    });
                                    ui.add_space(28.0);
                                    return;
                                }

                                for (idx, game) in results.iter().enumerate() {
                                    let selected = idx == state.selected_index;
                                    let is_running = running_games.contains_key(&game.id);
                                    palette_row(
                                        ui,
                                        game,
                                        selected,
                                        is_running,
                                        || state.selected_index = idx,
                                        actions,
                                    );
                                }
                            });
                    });

                Frame::NONE
                    .fill(theme::rgba(11, 11, 14, 235))
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 16,
                        se: 16,
                    })
                    .inner_margin(Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.set_width(modal_w - 32.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("↑↓ Navigate")
                                    .monospace()
                                    .size(10.5)
                                    .color(TEXT_DIM),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new("Enter Launch  ·  Shift+Enter Details")
                                            .monospace()
                                            .size(10.5)
                                            .color(TEXT_DIM),
                                    );
                                },
                            );
                        });
                    });
            });
        });
}

fn palette_row(
    ui: &mut egui::Ui,
    game: &Game,
    selected: bool,
    is_running: bool,
    mut on_hover_select: impl FnMut(),
    actions: &mut Vec<AppAction>,
) {
    let row_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(row_w, 56.0), Sense::hover());
    let hovered = ui.rect_contains_pointer(rect);
    if hovered {
        on_hover_select();
    }

    let p = ui.painter();
    let bg = if selected {
        BG_PILL_ACTIVE
    } else if hovered {
        theme::rgba(255, 255, 255, 11)
    } else {
        Color32::TRANSPARENT
    };
    let border = if selected {
        Stroke::new(1.0_f32, BORDER_ACTIVE)
    } else {
        Stroke::NONE
    };
    p.rect(rect, 11.0, bg, border, StrokeKind::Inside);
    if selected {
        theme::paint_glass_specular_top(p, rect, 11.0);
    }

    let thumb_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 10.0, rect.center().y - 22.0),
        Vec2::new(32.0, 44.0),
    );
    let uri = cover_image_uri(game.cover_url.as_deref(), game.cover_is_local);
    let drew = uri
        .as_deref()
        .is_some_and(|u| paint_cover_image(ui, u, thumb_rect, 6, Color32::WHITE, 1.0));
    if !drew {
        p.rect(
            thumb_rect,
            6.0,
            BG_GLASS_WELL,
            Stroke::new(1.0_f32, BORDER_GLASS),
            StrokeKind::Inside,
        );
        paint_icon(
            p,
            thumb_rect.center(),
            13.0,
            IconKind::Gamepad,
            TEXT_DIM,
        );
    }

    let eye_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 88.0, rect.center().y),
        Vec2::splat(26.0),
    );
    let eye_resp = ui
        .interact(eye_rect, Id::new(("pal_eye", &game.id)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    p.rect(
        eye_rect,
        8.0,
        if eye_resp.hovered() {
            theme::rgba(255, 255, 255, 24)
        } else {
            theme::rgba(255, 255, 255, 12)
        },
        Stroke::new(1.0_f32, BORDER_GLASS),
        StrokeKind::Inside,
    );
    paint_icon(
        p,
        eye_rect.center(),
        13.0,
        IconKind::Eye,
        if eye_resp.hovered() {
            Color32::WHITE
        } else {
            crate::theme::TEXT_SECONDARY
        },
    );
    if eye_resp.clicked() {
        actions.push(AppAction::OpenDetailModal(game.id.clone()));
        actions.push(AppAction::ClosePalette);
        return;
    }

    let pill_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 68.0, rect.center().y - 13.0),
        Vec2::new(58.0, 26.0),
    );
    if is_running {
        p.rect(
            pill_rect,
            8.0,
            EMERALD_BG,
            Stroke::new(1.0_f32, EMERALD_BORDER),
            StrokeKind::Inside,
        );
        let g = p.layout_no_wrap("Running".to_string(), FontId::monospace(10.5), EMERALD_TEXT);
        p.galley(pill_rect.center() - g.size() * 0.5, g, EMERALD_TEXT);
    } else {
        p.rect(
            pill_rect,
            8.0,
            theme::rgba(255, 255, 255, 18),
            Stroke::new(1.0_f32, BORDER_GLASS),
            StrokeKind::Inside,
        );
        paint_icon(
            p,
            Pos2::new(pill_rect.left() + 14.0, pill_rect.center().y),
            10.0,
            IconKind::Play,
            Color32::WHITE,
        );
        let g = p.layout_no_wrap("Play".to_string(), FontId::proportional(11.0), Color32::WHITE);
        p.galley(
            Pos2::new(pill_rect.left() + 24.0, pill_rect.center().y - g.size().y * 0.5),
            g,
            Color32::WHITE,
        );
    }

    let text_clip = Rect::from_min_max(
        Pos2::new(thumb_rect.right() + 12.0, rect.top() + 6.0),
        Pos2::new(eye_rect.left() - 10.0, rect.bottom() - 6.0),
    );
    let title_g = p.layout_no_wrap(game.title.clone(), FontId::proportional(13.5), TEXT_PRIMARY);
    let sub_text = format!(
        "{}  •  {}",
        platform_label(game.platform),
        format_playtime(game.playtime_seconds)
    );
    let sub_g = p.layout_no_wrap(sub_text, FontId::monospace(10.5), TEXT_MUTED);
    p.with_clip_rect(text_clip).galley(
        Pos2::new(text_clip.left(), rect.center().y - 16.0),
        title_g,
        TEXT_PRIMARY,
    );
    p.with_clip_rect(text_clip).galley(
        Pos2::new(text_clip.left(), rect.center().y + 3.0),
        sub_g,
        TEXT_MUTED,
    );

    let row_resp = ui
        .interact(rect, Id::new(("pal_row", &game.id)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    if row_resp.clicked() && !eye_resp.hovered() {
        if is_running {
            actions.push(AppAction::OpenDetailModal(game.id.clone()));
        } else {
            actions.push(AppAction::LaunchGame(game.id.clone()));
        }
        actions.push(AppAction::ClosePalette);
    }
}
