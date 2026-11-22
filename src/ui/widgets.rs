use std::hash::Hash;

use eframe::egui::load::TexturePoll;
use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, FontId, Id, Margin, Pos2, Rect, ResizeDirection,
    Response, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2, ViewportCommand,
};

use crate::theme::{
    self, BG_GLASS_WELL, BG_PILL_ACTIVE, BORDER_ACTIVE, BORDER_GLASS, BORDER_HOVER,
    BTN_PRIMARY_TEXT, BTN_SECONDARY_BG, BTN_SECONDARY_HOVER, DANGER_SOLID, DANGER_SOLID_HOVER,
    TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons::{paint_icon, IconKind};

#[derive(Clone, Copy)]
enum ButtonTone {
    Primary,
    Secondary,
    Danger,
}

pub fn glass_primary_button(
    ui: &mut Ui,
    icon: Option<IconKind>,
    label: &str,
    enabled: bool,
) -> Response {
    glass_action_button(ui, icon, label, enabled, ButtonTone::Primary)
}

pub fn glass_secondary_button(
    ui: &mut Ui,
    icon: Option<IconKind>,
    label: &str,
    enabled: bool,
) -> Response {
    glass_action_button(ui, icon, label, enabled, ButtonTone::Secondary)
}

pub fn glass_danger_button(
    ui: &mut Ui,
    icon: Option<IconKind>,
    label: &str,
    enabled: bool,
) -> Response {
    glass_action_button(ui, icon, label, enabled, ButtonTone::Danger)
}

fn glass_action_button(
    ui: &mut Ui,
    icon: Option<IconKind>,
    label: &str,
    enabled: bool,
    tone: ButtonTone,
) -> Response {
    let font = FontId::proportional(12.5);
    let measure_galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);

    let is_primary = matches!(tone, ButtonTone::Primary);
    let icon_space = if icon.is_some() { 20.0 } else { 0.0 };
    let pad_x = if is_primary { 15.0 } else { 12.0 };
    let width = pad_x * 2.0 + icon_space + measure_galley.size().x;
    let height = if is_primary { 33.0 } else { 32.0 };

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, height),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    let response = if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    };

    if ui.is_rect_visible(rect) {
        let hovered = enabled && response.hovered();
        let pressed = enabled && response.is_pointer_button_down_on();
        let hover_t = ui
            .ctx()
            .animate_bool_responsive(response.id.with("hover"), hovered);
        let draw_rect = if pressed {
            rect.translate(Vec2::new(0.0, 1.0))
        } else {
            rect
        };
        let radius = if is_primary { 11.0 } else { 10.0 };
        let radius_u8 = radius as u8;
        let p = ui.painter();

        match tone {
            ButtonTone::Primary => {
                if enabled {

                    p.rect_filled(
                        draw_rect.translate(Vec2::new(0.0, 3.0)),
                        radius,
                        Color32::from_black_alpha(145),
                    );
                    if hover_t > 0.01 {
                        p.rect_filled(
                            draw_rect.expand(1.5 * hover_t),
                            radius + 1.5 * hover_t,
                            theme::rgba(255, 255, 255, (28.0 * hover_t) as u8),
                        );
                    }

                    let bottom_bg = Color32::from_rgb(
                        (212.0 + 16.0 * hover_t) as u8,
                        (212.0 + 16.0 * hover_t) as u8,
                        (218.0 + 15.0 * hover_t) as u8,
                    );
                    let top_bg = Color32::from_rgb(
                        (246.0 + 9.0 * hover_t) as u8,
                        (246.0 + 9.0 * hover_t) as u8,
                        (250.0 + 5.0 * hover_t) as u8,
                    );
                    p.rect_filled(draw_rect, radius, bottom_bg);

                    let top_half = Rect::from_min_max(
                        draw_rect.min,
                        Pos2::new(draw_rect.max.x, draw_rect.center().y + 2.0),
                    );
                    p.rect_filled(
                        top_half,
                        CornerRadius {
                            nw: radius_u8,
                            ne: radius_u8,
                            sw: 0,
                            se: 0,
                        },
                        top_bg,
                    );

                    p.rect_stroke(
                        draw_rect,
                        radius,
                        Stroke::new(
                            1.0_f32,
                            theme::rgba(255, 255, 255, (150.0 + 65.0 * hover_t) as u8),
                        ),
                        StrokeKind::Inside,
                    );
                    let inset = 8.0;
                    p.line_segment(
                        [
                            Pos2::new(draw_rect.left() + inset, draw_rect.top() + 1.5),
                            Pos2::new(draw_rect.right() - inset, draw_rect.top() + 1.5),
                        ],
                        Stroke::new(1.0_f32, Color32::WHITE),
                    );
                } else {
                    p.rect(
                        draw_rect,
                        radius,
                        theme::rgba(150, 150, 158, 110),
                        Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 50)),
                        StrokeKind::Inside,
                    );
                }

                let fg = if enabled {
                    BTN_PRIMARY_TEXT
                } else {
                    Color32::from_rgb(40, 40, 46)
                };
                let fg_galley = p.layout_no_wrap(label.to_string(), font, fg);
                let mut x = draw_rect.left() + pad_x;
                if let Some(kind) = icon {
                    paint_icon(p, Pos2::new(x + 6.5, draw_rect.center().y), 13.5, kind, fg);
                    x += icon_space;
                }
                p.galley(
                    Pos2::new(x, draw_rect.center().y - fg_galley.size().y * 0.5),
                    fg_galley,
                    fg,
                );
            }
            ButtonTone::Secondary | ButtonTone::Danger => {
                let (bg, border, fg) = match tone {
                    ButtonTone::Secondary => (
                        if hovered {
                            BTN_SECONDARY_HOVER
                        } else {
                            BTN_SECONDARY_BG
                        },
                        if hovered { BORDER_HOVER } else { BORDER_GLASS },
                        if enabled { TEXT_PRIMARY } else { TEXT_DIM },
                    ),
                    ButtonTone::Danger => (
                        if hovered {
                            DANGER_SOLID_HOVER
                        } else {
                            DANGER_SOLID
                        },
                        theme::rgba(253, 164, 175, 120),
                        Color32::WHITE,
                    ),
                    ButtonTone::Primary => unreachable!(),
                };

                p.rect(
                    draw_rect,
                    radius,
                    bg,
                    Stroke::new(1.0_f32, border),
                    StrokeKind::Inside,
                );
                if matches!(tone, ButtonTone::Secondary) && enabled {
                    theme::paint_glass_specular_top(p, draw_rect, radius);
                }

                let fg_galley = p.layout_no_wrap(label.to_string(), font, fg);
                let mut x = draw_rect.left() + pad_x;
                if let Some(kind) = icon {
                    paint_icon(p, Pos2::new(x + 6.0, draw_rect.center().y), 13.0, kind, fg);
                    x += icon_space;
                }
                p.galley(
                    Pos2::new(x, draw_rect.center().y - fg_galley.size().y * 0.5),
                    fg_galley,
                    fg,
                );
            }
        }
    }

    response
}

pub fn glass_pill_button(
    ui: &mut Ui,
    icon: Option<IconKind>,
    label: &str,
    active: bool,
    trailing_count: Option<usize>,
    full_width: bool,
) -> Response {
    let font = FontId::proportional(12.5);
    let measure_galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);

    let measure_count = trailing_count.map(|c| {
        ui.painter()
            .layout_no_wrap(c.to_string(), FontId::monospace(11.0), Color32::WHITE)
    });

    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let count_w = measure_count
        .as_ref()
        .map_or(0.0, |g| g.size().x + 12.0);
    let width = if full_width {
        ui.available_width()
    } else {
        20.0 + icon_w + measure_galley.size().x + count_w
    };
    let height = 31.0;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let bg = if active {
            BG_PILL_ACTIVE
        } else if hovered {
            theme::rgba(255, 255, 255, 12)
        } else {
            Color32::TRANSPARENT
        };
        let border = if active {
            Stroke::new(1.0_f32, BORDER_ACTIVE)
        } else if hovered {
            Stroke::new(1.0_f32, BORDER_GLASS)
        } else {
            Stroke::NONE
        };
        let fg = if active || hovered {
            TEXT_PRIMARY
        } else {
            TEXT_MUTED
        };

        let p = ui.painter();
        p.rect(rect, 9.0, bg, border, StrokeKind::Inside);
        if active {
            theme::paint_glass_specular_top(p, rect, 9.0);
        }

        let mut x = rect.left() + 10.0;
        if let Some(kind) = icon {
            paint_icon(p, Pos2::new(x + 7.0, rect.center().y), 13.5, kind, fg);
            x += 22.0;
        }

        let fg_galley = p.layout_no_wrap(label.to_string(), font, fg);
        let max_text_right = rect.right() - 10.0 - count_w;
        let text_clip = Rect::from_min_max(
            Pos2::new(x, rect.top()),
            Pos2::new(max_text_right.max(x), rect.bottom()),
        );
        p.with_clip_rect(text_clip).galley(
            Pos2::new(x, rect.center().y - fg_galley.size().y * 0.5),
            fg_galley,
            fg,
        );

        if let Some(c) = trailing_count {
            let count_fg = if active { TEXT_SECONDARY } else { TEXT_DIM };
            let cg = p.layout_no_wrap(c.to_string(), FontId::monospace(11.0), count_fg);
            let cx = rect.right() - 10.0 - cg.size().x;
            p.galley(
                Pos2::new(cx, rect.center().y - cg.size().y * 0.5),
                cg,
                count_fg,
            );
        }
    }

    response
}

pub fn glass_icon_button(
    ui: &mut Ui,
    _id_salt: impl Hash,
    icon: IconKind,
    size: f32,
    active: bool,
    danger_hover: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let bg = if active {
            theme::rgba(24, 24, 30, 235)
        } else if hovered {
            if danger_hover {
                theme::rgba(225, 29, 72, 180)
            } else {
                theme::rgba(255, 255, 255, 22)
            }
        } else {
            theme::rgba(14, 14, 18, 190)
        };
        let border = if active {
            BORDER_ACTIVE
        } else if hovered {
            BORDER_HOVER
        } else {
            BORDER_GLASS
        };
        let fg = if active || hovered {
            Color32::WHITE
        } else {
            TEXT_MUTED
        };

        let radius = size * 0.5;
        let p = ui.painter();
        p.circle_filled(rect.center(), radius, bg);
        p.circle_stroke(rect.center(), radius, Stroke::new(1.0_f32, border));
        paint_icon(p, rect.center(), size * 0.48, icon, fg);
    }

    response
}

pub fn glass_toggle(ui: &mut Ui, checked: &mut bool) -> Response {
    let desired = Vec2::new(38.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    let mut response = response.on_hover_cursor(CursorIcon::PointingHand);
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *checked);
        let bg = if *checked {
            theme::rgba(255, 255, 255, 68)
        } else {
            BG_GLASS_WELL
        };
        let border = if *checked {
            BORDER_ACTIVE
        } else {
            BORDER_GLASS
        };
        let p = ui.painter();
        let radius = rect.height() * 0.5;
        p.rect(
            rect,
            radius,
            bg,
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
        let knob_x = egui::lerp((rect.left() + 10.5)..=(rect.right() - 10.5), how_on);
        let knob_color = if *checked { Color32::WHITE } else { TEXT_DIM };
        p.circle_filled(Pos2::new(knob_x, rect.center().y), 7.0, knob_color);
    }

    response
}

pub fn glass_text_edit(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    monospace: bool,
    width: Option<f32>,
) -> Response {
    let mut edit = TextEdit::singleline(text)
        .hint_text(hint)
        .margin(Margin::symmetric(10, 7))
        .text_color(TEXT_PRIMARY);
    if monospace {
        edit = edit.font(FontId::monospace(12.0));
    } else {
        edit = edit.font(FontId::proportional(13.0));
    }
    if let Some(w) = width {
        edit = edit.desired_width(w);
    } else {
        edit = edit.desired_width(f32::INFINITY);
    }

    let frame_resp = egui::Frame::NONE
        .fill(BG_GLASS_WELL)
        .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
        .corner_radius(10.0)
        .show(ui, |ui| ui.add(edit));
    frame_resp.inner
}

pub fn paint_cover_image(
    ui: &Ui,
    uri: &str,
    rect: Rect,
    corner_radius: impl Into<CornerRadius>,
    tint: Color32,
    hover_zoom: f32,
) -> bool {
    let img = egui::Image::new(uri);
    let Ok(TexturePoll::Ready { texture }) = img.load_for_size(ui.ctx(), rect.size()) else {
        return false;
    };

    if texture.size.x <= 0.0 || texture.size.y <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return false;
    }

    let img_aspect = texture.size.x / texture.size.y;
    let rect_aspect = rect.width() / rect.height();

    let (mut uv_w, mut uv_h) = if img_aspect > rect_aspect {
        (rect_aspect / img_aspect, 1.0)
    } else {
        (1.0, img_aspect / rect_aspect)
    };

    let zoom = hover_zoom.max(1.0);
    uv_w /= zoom;
    uv_h /= zoom;

    let uv_min = Pos2::new((1.0 - uv_w) * 0.5, (1.0 - uv_h) * 0.5);
    let uv_max = Pos2::new(uv_min.x + uv_w, uv_min.y + uv_h);

    egui::Image::new(uri)
        .uv(Rect::from_min_max(uv_min, uv_max))
        .corner_radius(corner_radius)
        .tint(tint)
        .paint_at(ui, rect);

    true
}

pub fn status_badge(
    ui: &mut Ui,
    label: &str,
    bg: Color32,
    border: Color32,
    fg: Color32,
    dot: Option<Color32>,
) {
    let font = FontId::monospace(10.5);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    let dot_w = if dot.is_some() { 11.0 } else { 0.0 };
    let width = 16.0 + dot_w + galley.size().x;
    let height = 22.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect(
            rect,
            11.0,
            bg,
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
        let mut x = rect.left() + 8.0;
        if let Some(dot_color) = dot {
            p.circle_filled(Pos2::new(x + 3.0, rect.center().y), 3.0, dot_color);
            x += dot_w;
        }
        p.galley(
            Pos2::new(x, rect.center().y - galley.size().y * 0.5),
            galley,
            fg,
        );
    }
}

pub fn paint_modal_backdrop_order(
    ctx: &egui::Context,
    id_salt: &str,
    order: egui::Order,
) -> bool {
    let screen = ctx.screen_rect();
    let mut clicked_outside = false;
    egui::Area::new(Id::new(id_salt))
        .order(order)
        .movable(false)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(175));
            if resp.clicked() {
                clicked_outside = true;
            }
        });
    clicked_outside
}

pub fn paint_modal_backdrop(ctx: &egui::Context, id_salt: &str) -> bool {
    paint_modal_backdrop_order(ctx, id_salt, egui::Order::Middle)
}

pub fn section_overline(ui: &mut Ui, label: &str) {
    ui.label(
        egui::RichText::new(label.to_uppercase())
            .size(10.0)
            .strong()
            .color(TEXT_DIM),
    );
}

pub fn handle_borderless_window_resize(ctx: &egui::Context) {
    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if is_maximized {
        return;
    }

    let screen = ctx.screen_rect();
    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
    let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
    let Some(pos) = pointer_pos else {
        return;
    };

    let edge = 6.0;
    let left = pos.x <= screen.left() + edge;
    let right = pos.x >= screen.right() - edge;
    let top = pos.y <= screen.top() + edge;
    let bottom = pos.y >= screen.bottom() - edge;

    let (dir, cursor) = match (left, right, top, bottom) {
        (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
        (_, true, true, _) => (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
        (true, _, _, true) => (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
        (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
        (true, _, _, _) => (ResizeDirection::West, CursorIcon::ResizeHorizontal),
        (_, true, _, _) => (ResizeDirection::East, CursorIcon::ResizeHorizontal),
        (_, _, true, _) => (ResizeDirection::North, CursorIcon::ResizeVertical),
        (_, _, _, true) => (ResizeDirection::South, CursorIcon::ResizeVertical),
        _ => return,
    };

    ctx.set_cursor_icon(cursor);
    if primary_pressed {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}

pub fn glass_well_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(BG_GLASS_WELL)
        .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
        .corner_radius(12.0)
        .inner_margin(Margin::same(14))
}

pub fn glass_modal_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(theme::BG_GLASS_ELEVATED)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_HOVER))
        .corner_radius(16.0)
        .shadow(egui::Shadow {
            offset: [0, 18],
            blur: 48,
            spread: 0,
            color: Color32::from_black_alpha(220),
        })
}

pub fn paint_kbd_chip(ui: &mut Ui, text: &str) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::monospace(10.0), TEXT_MUTED);
    let size = Vec2::new(galley.size().x + 12.0, 20.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect(
            rect,
            6.0,
            Color32::from_rgb(16, 16, 20),
            Stroke::new(1.0_f32, BORDER_GLASS),
            StrokeKind::Inside,
        );
        p.galley(rect.center() - galley.size() * 0.5, galley, TEXT_MUTED);
    }
}
