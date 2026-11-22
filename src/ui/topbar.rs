use eframe::egui::{
    self, Color32, CursorIcon, FontId, Frame, Margin, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

use crate::models::Platform;
use crate::theme::{
    self, BG_GLASS_WELL, BG_PILL_ACTIVE, BG_TOPBAR, BORDER_ACTIVE, BORDER_GLASS, BORDER_HOVER,
    BORDER_SUBTLE, EMERALD_BG, EMERALD_BORDER, EMERALD_DOT, EMERALD_TEXT, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::{glass_secondary_button, status_badge};
use crate::ui::{AppAction, SidebarFilter, SortOption};

fn segment_filters() -> [(&'static str, SidebarFilter); 6] {
    [
        ("All", SidebarFilter::All),
        ("Favorites", SidebarFilter::Favorites),
        ("Recent", SidebarFilter::Recent),
        ("Steam", SidebarFilter::Platform(Platform::Steam)),
        ("Manual", SidebarFilter::Platform(Platform::Manual)),
        ("Emulator", SidebarFilter::Platform(Platform::Emulator)),
    ]
}

pub fn show(
    ui: &mut egui::Ui,
    filter: &SidebarFilter,
    total_visible: usize,
    running_count: usize,
    sort: SortOption,
    actions: &mut Vec<AppAction>,
) {
    Frame::NONE
        .fill(BG_TOPBAR)
        .inner_margin(Margin::symmetric(20, 10))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left() - 20.0, rect.bottom() + 10.0),
                    Pos2::new(rect.right() + 20.0, rect.bottom() + 10.0),
                ],
                Stroke::new(1.0_f32, BORDER_SUBTLE),
            );

            ui.horizontal(|ui| {
                let avail = ui.available_width();

                if avail > 790.0 {
                    Frame::NONE
                        .fill(BG_GLASS_WELL)
                        .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
                        .corner_radius(10.0)
                        .inner_margin(Margin::same(3))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (label, seg_filter) in segment_filters() {
                                let active = *filter == seg_filter;
                                if segment_chip(ui, label, active).clicked() {
                                    actions.push(AppAction::SelectFilter(seg_filter));
                                }
                            }
                        });
                } else {
                    ui.label(
                        egui::RichText::new(filter.title())
                            .size(14.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                }

                ui.add_space(4.0);
                status_badge(
                    ui,
                    &total_visible.to_string(),
                    theme::rgba(255, 255, 255, 10),
                    BORDER_GLASS,
                    TEXT_SECONDARY,
                    None,
                );

                if running_count > 0 {
                    status_badge(
                        ui,
                        &format!("{running_count} active"),
                        EMERALD_BG,
                        EMERALD_BORDER,
                        EMERALD_TEXT,
                        Some(EMERALD_DOT),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    if glass_secondary_button(ui, Some(IconKind::Plus), "Add Game", true)
                        .clicked()
                    {
                        actions.push(AppAction::OpenAddModal(None));
                    }

                    let mut selected_sort = sort;
                    egui::ComboBox::from_id_salt("library_sort_combo")
                        .selected_text(
                            egui::RichText::new(selected_sort.label())
                                .size(12.0)
                                .color(TEXT_SECONDARY),
                        )
                        .width(148.0)
                        .show_ui(ui, |ui| {
                            for option in SortOption::ALL {
                                if ui
                                    .selectable_value(&mut selected_sort, option, option.label())
                                    .clicked()
                                {
                                    actions.push(AppAction::SetSort(option));
                                }
                            }
                        });

                    if search_trigger_box(ui).clicked() {
                        actions.push(AppAction::TogglePalette);
                    }
                });
            });
        });
}

fn segment_chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let font = FontId::proportional(12.0);
    let measure = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(measure.size().x + 18.0, 25.0), Sense::click());
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
        let galley = p.layout_no_wrap(label.to_string(), font, fg);
        p.galley(rect.center() - galley.size() * 0.5, galley, fg);
    }
    resp
}

fn search_trigger_box(ui: &mut egui::Ui) -> egui::Response {
    let width = ui.available_width().clamp(160.0, 225.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let p = ui.painter();
        p.rect(
            rect,
            10.0,
            BG_GLASS_WELL,
            Stroke::new(1.0_f32, if hovered { BORDER_HOVER } else { BORDER_GLASS }),
            StrokeKind::Inside,
        );
        paint_icon(
            p,
            Pos2::new(rect.left() + 14.0, rect.center().y),
            13.0,
            IconKind::Search,
            TEXT_MUTED,
        );
        let hint_galley = p.layout_no_wrap(
            "Quick Search...".to_string(),
            FontId::proportional(12.0),
            if hovered { TEXT_SECONDARY } else { TEXT_MUTED },
        );
        p.galley(
            Pos2::new(rect.left() + 28.0, rect.center().y - hint_galley.size().y * 0.5),
            hint_galley,
            if hovered { TEXT_SECONDARY } else { TEXT_MUTED },
        );

        let kbd_galley = p.layout_no_wrap(
            "Ctrl+K".to_string(),
            FontId::monospace(10.0),
            TEXT_MUTED,
        );
        let kbd_w = kbd_galley.size().x + 12.0;
        let kbd_rect = Rect::from_min_size(
            Pos2::new(rect.right() - 8.0 - kbd_w, rect.center().y - 10.0),
            Vec2::new(kbd_w, 20.0),
        );
        p.rect(
            kbd_rect,
            6.0,
            Color32::from_rgb(16, 16, 20),
            Stroke::new(1.0_f32, BORDER_GLASS),
            StrokeKind::Inside,
        );
        p.galley(kbd_rect.center() - kbd_galley.size() * 0.5, kbd_galley, TEXT_MUTED);
    }
    resp
}
