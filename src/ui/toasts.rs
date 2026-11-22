use eframe::egui::{self, Align2, Id, Margin, Sense, Vec2};

use crate::theme::{DANGER_TEXT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY};
use crate::ui::icons::{paint_icon, IconKind};
use crate::ui::widgets::{glass_icon_button, glass_modal_frame};
use crate::ui::{AppAction, ToastItem, ToastVariant};

pub fn show(ctx: &egui::Context, toasts: &[ToastItem], actions: &mut Vec<AppAction>) {
    if toasts.is_empty() {
        return;
    }

    egui::Area::new(Id::new("toast_container"))
        .order(egui::Order::Tooltip)
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -16.0))
        .show(ctx, |ui| {
            ui.set_width(320.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                for toast in toasts {
                    glass_modal_frame()
                        .corner_radius(12.0)
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(296.0);
                            ui.horizontal_top(|ui| {
                                let (icon_kind, icon_color) = match toast.variant {
                                    ToastVariant::Success => (IconKind::Check, TEXT_PRIMARY),
                                    ToastVariant::Info => (IconKind::Info, TEXT_SECONDARY),
                                    ToastVariant::Error => (IconKind::Alert, DANGER_TEXT),
                                };
                                let (icon_r, _) =
                                    ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                paint_icon(
                                    ui.painter(),
                                    icon_r.center(),
                                    14.0,
                                    icon_kind,
                                    icon_color,
                                );

                                ui.vertical(|ui| {
                                    ui.set_width(240.0);
                                    ui.label(
                                        egui::RichText::new(&toast.title)
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                    if let Some(ref desc) = toast.description {
                                        ui.add_space(2.0);
                                        ui.label(
                                            egui::RichText::new(desc)
                                                .size(11.0)
                                                .color(TEXT_MUTED),
                                        );
                                    }
                                });

                                if glass_icon_button(
                                    ui,
                                    ("toast_dismiss", toast.id),
                                    IconKind::Close,
                                    20.0,
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    actions.push(AppAction::DismissToast(toast.id));
                                }
                            });
                        });
                }
            });
        });
}
