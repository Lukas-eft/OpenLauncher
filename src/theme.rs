use std::fs;
use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Painter, Pos2,
    Rect, Shadow, Stroke, Visuals,
};

#[inline]
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    let pr = ((r as u16 * a as u16 + 127) / 255) as u8;
    let pg = ((g as u16 * a as u16 + 127) / 255) as u8;
    let pb = ((b as u16 * a as u16 + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(pr, pg, pb, a)
}

pub const BG_CANVAS: Color32 = Color32::from_rgb(10, 10, 12);
pub const BG_TITLEBAR: Color32 = rgba(12, 12, 16, 245);
pub const BG_SIDEBAR: Color32 = rgba(13, 13, 17, 232);
pub const BG_TOPBAR: Color32 = rgba(13, 13, 17, 215);
pub const BG_GLASS_SURFACE: Color32 = rgba(21, 21, 26, 224);
pub const BG_GLASS_HOVER: Color32 = rgba(29, 29, 36, 235);
pub const BG_GLASS_ELEVATED: Color32 = rgba(20, 20, 25, 248);
pub const BG_GLASS_WELL: Color32 = rgba(9, 9, 13, 215);
pub const BG_PILL_ACTIVE: Color32 = rgba(255, 255, 255, 28);

pub const BORDER_SUBTLE: Color32 = rgba(255, 255, 255, 19);
pub const BORDER_GLASS: Color32 = rgba(255, 255, 255, 24);
pub const BORDER_HOVER: Color32 = rgba(255, 255, 255, 48);
pub const BORDER_ACTIVE: Color32 = rgba(255, 255, 255, 62);
pub const SPECULAR_TOP: Color32 = rgba(255, 255, 255, 34);

pub const BTN_PRIMARY_BG: Color32 = Color32::from_rgb(230, 230, 234);
pub const BTN_PRIMARY_HOVER: Color32 = Color32::from_rgb(255, 255, 255);
pub const BTN_PRIMARY_TEXT: Color32 = Color32::from_rgb(9, 9, 11);

pub const BTN_SECONDARY_BG: Color32 = rgba(255, 255, 255, 14);
pub const BTN_SECONDARY_HOVER: Color32 = rgba(255, 255, 255, 26);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 244, 245);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(212, 212, 216);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(161, 161, 170);
pub const TEXT_DIM: Color32 = Color32::from_rgb(113, 113, 122);

pub const EMERALD_BG: Color32 = rgba(16, 185, 129, 28);
pub const EMERALD_BORDER: Color32 = rgba(52, 211, 153, 76);
pub const EMERALD_TEXT: Color32 = Color32::from_rgb(110, 231, 183);
pub const EMERALD_DOT: Color32 = Color32::from_rgb(52, 211, 153);

pub const DANGER_BG: Color32 = rgba(244, 63, 94, 26);
pub const DANGER_BORDER: Color32 = rgba(251, 113, 133, 92);
pub const DANGER_TEXT: Color32 = Color32::from_rgb(253, 164, 175);
pub const DANGER_SOLID: Color32 = Color32::from_rgb(225, 29, 72);
pub const DANGER_SOLID_HOVER: Color32 = Color32::from_rgb(244, 63, 94);

pub fn configure_theme_and_fonts(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = Margin::same(16);
    style.spacing.interact_size = egui::vec2(32.0, 28.0);
    style.spacing.scroll = egui::style::ScrollStyle {
        floating: true,
        bar_width: 6.0,
        handle_min_length: 28.0,
        bar_inner_margin: 2.0,
        bar_outer_margin: 3.0,
        floating_width: 5.0,
        floating_allocated_width: 0.0,
        foreground_color: true,
        dormant_background_opacity: 0.0,
        active_background_opacity: 0.08,
        interact_background_opacity: 0.14,
        dormant_handle_opacity: 0.28,
        active_handle_opacity: 0.52,
        interact_handle_opacity: 0.78,
    };

    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_GLASS_ELEVATED;
    visuals.faint_bg_color = rgba(255, 255, 255, 8);
    visuals.extreme_bg_color = BG_GLASS_WELL;
    visuals.code_bg_color = BG_GLASS_WELL;
    visuals.window_corner_radius = CornerRadius::same(16);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_GLASS);
    visuals.window_shadow = Shadow {
        offset: [0, 14],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(200),
    };
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.popup_shadow = Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(190),
    };
    visuals.selection.bg_fill = rgba(255, 255, 255, 38);
    visuals.selection.stroke = Stroke::new(1.0_f32, BORDER_ACTIVE);

    visuals.widgets.noninteractive.bg_fill = BG_GLASS_SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = BG_GLASS_WELL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_MUTED);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    visuals.widgets.inactive.bg_fill = BTN_SECONDARY_BG;
    visuals.widgets.inactive.weak_bg_fill = BTN_SECONDARY_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_GLASS);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(10);

    visuals.widgets.hovered.bg_fill = BTN_SECONDARY_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BTN_SECONDARY_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_HOVER);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(10);

    visuals.widgets.active.bg_fill = BG_PILL_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BG_PILL_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, BORDER_ACTIVE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(10);

    visuals.widgets.open.bg_fill = BG_PILL_ACTIVE;
    visuals.widgets.open.weak_bg_fill = BG_PILL_ACTIVE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, BORDER_ACTIVE);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.open.corner_radius = CornerRadius::same(10);

    style.visuals = visuals;
    ctx.set_style(style);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    #[cfg(target_os = "windows")]
    {
        if let Some(bytes) = load_first_existing_font(&[
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
        ]) {
            fonts
                .font_data
                .insert("win_ui".to_owned(), Arc::new(FontData::from_owned(bytes)));
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
                fam.insert(0, "win_ui".to_owned());
            }
        }
        if let Some(bytes) = load_first_existing_font(&[
            r"C:\Windows\Fonts\CascadiaMono.ttf",
            r"C:\Windows\Fonts\consola.ttf",
        ]) {
            fonts.font_data.insert(
                "win_mono".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
                fam.insert(0, "win_mono".to_owned());
            }
        }
    }

    ctx.set_fonts(fonts);
}

#[cfg(target_os = "windows")]
fn load_first_existing_font(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|path| fs::read(path).ok())
}

pub fn paint_liquid_canvas_bg(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 0.0, BG_CANVAS);

    let glow_centers = [
        (
            Pos2::new(rect.left() + rect.width() * 0.16, rect.top() + 30.0),
            rect.width() * 0.48,
            (255u8, 255u8, 255u8),
            7u8,
        ),
        (
            Pos2::new(rect.left() + rect.width() * 0.84, rect.top() + 90.0),
            rect.width() * 0.42,
            (180u8, 184u8, 195u8),
            6u8,
        ),
        (
            Pos2::new(rect.center().x, rect.bottom() - 20.0),
            rect.width() * 0.45,
            (255u8, 255u8, 255u8),
            4u8,
        ),
    ];

    for (center, radius, (r, g, b), base_alpha) in glow_centers {
        for step in 0..4 {
            let factor = 1.0 - (step as f32) * 0.22;
            let ring_alpha = ((base_alpha as f32) * (0.28 + 0.18 * (step as f32))).round() as u8;
            painter.circle_filled(center, radius * factor, rgba(r, g, b, ring_alpha));
        }
    }
}

pub fn paint_glass_specular_top(painter: &Painter, rect: Rect, radius: f32) {
    let inset = (radius * 0.5).clamp(6.0, 16.0);
    if rect.width() <= inset * 2.0 + 4.0 {
        return;
    }
    let y = rect.top() + 1.0;
    painter.line_segment(
        [
            Pos2::new(rect.left() + inset, y),
            Pos2::new(rect.right() - inset, y),
        ],
        Stroke::new(1.0_f32, SPECULAR_TOP),
    );
}

pub fn paint_monochrome_gradient(painter: &Painter, rect: Rect, radius: f32) {
    painter.rect_filled(rect, radius, Color32::from_rgb(15, 15, 19));
    let top_rect = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.top() + rect.height() * 0.48),
    );
    painter.rect_filled(
        top_rect,
        CornerRadius {
            nw: radius as u8,
            ne: radius as u8,
            sw: 0,
            se: 0,
        },
        rgba(255, 255, 255, 10),
    );
    paint_glass_specular_top(painter, rect, radius);
}
