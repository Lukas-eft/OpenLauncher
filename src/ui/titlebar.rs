use eframe::egui::{
    self, Color32, CursorIcon, Frame, Margin, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
    ViewportCommand,
};

use crate::models::ActiveGameStatus;
use crate::theme::{
    self, BG_TITLEBAR, BORDER_SUBTLE, EMERALD_BG, EMERALD_BORDER, EMERALD_DOT, EMERALD_TEXT,
    TEXT_MUTED, TEXT_PRIMARY,
};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::status_badge;

pub fn show(ctx: &egui::Context, running_games: &[ActiveGameStatus]) {
    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    egui::TopBottomPanel::top("titlebar")
        .exact_height(42.0)
        .frame(
            Frame::NONE
                .fill(BG_TITLEBAR)
                .inner_margin(Margin::symmetric(14, 0)),
        )
        .show_separator_line(false)
        .show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            ui.painter().line_segment(
                [
                    Pos2::new(panel_rect.left() - 14.0, panel_rect.bottom()),
                    Pos2::new(panel_rect.right() + 14.0, panel_rect.bottom()),
                ],
                Stroke::new(1.0_f32, BORDER_SUBTLE),
            );

            ui.horizontal_centered(|ui| {

                ui.label(
                    egui::RichText::new("OpenLauncher")
                        .size(13.5)
                        .strong()
                        .color(TEXT_PRIMARY),
                );

                if let Some(primary) = running_games.first() {
                    ui.add_space(6.0);
                    let label = format!("RUNNING · {}", truncate_title(&primary.title, 28));
                    status_badge(
                        ui,
                        &label,
                        EMERALD_BG,
                        EMERALD_BORDER,
                        EMERALD_TEXT,
                        Some(EMERALD_DOT),
                    );
                }

                let buttons_w = 122.0;
                let drag_w = (ui.available_width() - buttons_w).max(20.0);
                let (drag_rect, drag_resp) =
                    ui.allocate_exact_size(Vec2::new(drag_w, 40.0), Sense::click_and_drag());

                if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag_resp.double_clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
                }
                let _ = drag_rect;

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    if window_control_btn(ui, IconKind::Close, true).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    let max_icon = if is_maximized {
                        IconKind::Restore
                    } else {
                        IconKind::Maximize
                    };
                    if window_control_btn(ui, max_icon, false).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
                    }
                    if window_control_btn(ui, IconKind::Minimize, false).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                });
            });
        });
}

fn window_control_btn(ui: &mut egui::Ui, icon: IconKind, is_close: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(36.0, 28.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let bg = if hovered {
            if is_close {
                theme::rgba(225, 29, 72, 210)
            } else {
                theme::rgba(255, 255, 255, 20)
            }
        } else {
            Color32::TRANSPARENT
        };
        let fg = if hovered {
            Color32::WHITE
        } else {
            TEXT_MUTED
        };
        let p = ui.painter();
        p.rect(rect, 7.0, bg, Stroke::NONE, StrokeKind::Inside);
        let shrink_rect = Rect::from_center_size(rect.center(), Vec2::splat(11.0));
        paint_icon(p, shrink_rect.center(), 11.0, icon, fg);
    }
    resp
}

fn truncate_title(title: &str, max_chars: usize) -> String {
    let count = title.chars().count();
    if count <= max_chars {
        title.to_string()
    } else {
        let prefix: String = title.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{prefix}…")
    }
}
