use eframe::egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, Vec2};

/// Scharfe Vektor-Icons ohne Plattform-Emoji-Abhängigkeiten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Minimize,
    Maximize,
    Restore,
    Close,
    Layers,
    Heart { filled: bool },
    Clock,
    Gamepad,
    Folder,
    FolderSearch,
    Cpu,
    Tag,
    Refresh,
    Settings,
    Search,
    Plus,
    Play,
    Edit,
    Trash,
    Check,
    Eye,
    Sparkles,
    ImagePlus,
    Terminal,
    RotateCcw,
    Info,
    Alert,
}

pub fn paint_icon(painter: &Painter, center: Pos2, size: f32, kind: IconKind, color: Color32) {
    let r = size * 0.5;
    let stroke = Stroke::new((size * 0.115).clamp(1.25, 2.0), color);

    match kind {
        IconKind::Minimize => {
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.7, center.y + r * 0.05),
                    Pos2::new(center.x + r * 0.7, center.y + r * 0.05),
                ],
                stroke,
            );
        }
        IconKind::Maximize => {
            let rect = Rect::from_center_size(center, Vec2::splat(size * 0.68));
            painter.rect_stroke(rect, 2.0, stroke, eframe::egui::StrokeKind::Inside);
        }
        IconKind::Restore => {
            let back = Rect::from_center_size(
                center + Vec2::new(r * 0.18, -r * 0.18),
                Vec2::splat(size * 0.52),
            );
            let front = Rect::from_center_size(
                center + Vec2::new(-r * 0.14, r * 0.16),
                Vec2::splat(size * 0.52),
            );
            painter.rect_stroke(back, 1.5, stroke, eframe::egui::StrokeKind::Inside);
            painter.rect_filled(front, 1.5, Color32::from_rgb(14, 14, 18));
            painter.rect_stroke(front, 1.5, stroke, eframe::egui::StrokeKind::Inside);
        }
        IconKind::Close => {
            let d = r * 0.62;
            painter.line_segment(
                [
                    Pos2::new(center.x - d, center.y - d),
                    Pos2::new(center.x + d, center.y + d),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + d, center.y - d),
                    Pos2::new(center.x - d, center.y + d),
                ],
                stroke,
            );
        }
        IconKind::Layers => {
            for dy in [-r * 0.42, 0.0, r * 0.42] {
                painter.line_segment(
                    [
                        Pos2::new(center.x - r * 0.68, center.y + dy),
                        Pos2::new(center.x + r * 0.68, center.y + dy),
                    ],
                    stroke,
                );
            }
        }
        IconKind::Heart { filled } => {
            let left = Pos2::new(center.x - r * 0.34, center.y - r * 0.22);
            let right = Pos2::new(center.x + r * 0.34, center.y - r * 0.22);
            let bottom = Pos2::new(center.x, center.y + r * 0.68);
            let lobe_r = r * 0.38;
            if filled {
                painter.circle_filled(left, lobe_r, color);
                painter.circle_filled(right, lobe_r, color);
                painter.add(eframe::egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(center.x - r * 0.69, center.y - r * 0.06),
                        Pos2::new(center.x + r * 0.69, center.y - r * 0.06),
                        bottom,
                    ],
                    color,
                    Stroke::NONE,
                ));
            } else {
                let pts = vec![
                    bottom,
                    Pos2::new(center.x - r * 0.70, center.y - r * 0.08),
                    Pos2::new(center.x - r * 0.56, center.y - r * 0.50),
                    Pos2::new(center.x - r * 0.20, center.y - r * 0.54),
                    Pos2::new(center.x, center.y - r * 0.26),
                    Pos2::new(center.x + r * 0.20, center.y - r * 0.54),
                    Pos2::new(center.x + r * 0.56, center.y - r * 0.50),
                    Pos2::new(center.x + r * 0.70, center.y - r * 0.08),
                ];
                painter.add(eframe::egui::Shape::closed_line(pts, stroke));
            }
        }
        IconKind::Clock => {
            painter.circle_stroke(center, r * 0.78, stroke);
            painter.line_segment([center, Pos2::new(center.x, center.y - r * 0.44)], stroke);
            painter.line_segment(
                [center, Pos2::new(center.x + r * 0.36, center.y + r * 0.16)],
                stroke,
            );
        }
        IconKind::Gamepad => {
            let body = Rect::from_center_size(center, Vec2::new(size * 0.92, size * 0.58));
            painter.rect_stroke(
                body,
                CornerRadius::same((size * 0.22) as u8),
                stroke,
                eframe::egui::StrokeKind::Inside,
            );
            let dpad = Pos2::new(center.x - r * 0.38, center.y);
            painter.line_segment(
                [
                    Pos2::new(dpad.x - r * 0.16, dpad.y),
                    Pos2::new(dpad.x + r * 0.16, dpad.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(dpad.x, dpad.y - r * 0.16),
                    Pos2::new(dpad.x, dpad.y + r * 0.16),
                ],
                stroke,
            );
            painter.circle_filled(
                Pos2::new(center.x + r * 0.40, center.y - r * 0.10),
                r * 0.10,
                color,
            );
            painter.circle_filled(
                Pos2::new(center.x + r * 0.24, center.y + r * 0.10),
                r * 0.10,
                color,
            );
        }
        IconKind::Folder => {
            let body = Rect::from_min_max(
                Pos2::new(center.x - r * 0.78, center.y - r * 0.34),
                Pos2::new(center.x + r * 0.78, center.y + r * 0.64),
            );
            painter.rect_stroke(body, 2.5, stroke, eframe::egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.78, center.y - r * 0.34),
                    Pos2::new(center.x - r * 0.56, center.y - r * 0.60),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.56, center.y - r * 0.60),
                    Pos2::new(center.x - r * 0.05, center.y - r * 0.60),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.05, center.y - r * 0.60),
                    Pos2::new(center.x + r * 0.12, center.y - r * 0.34),
                ],
                stroke,
            );
        }
        IconKind::FolderSearch => {
            paint_icon(
                painter,
                center + Vec2::new(-r * 0.12, -r * 0.05),
                size * 0.85,
                IconKind::Folder,
                color,
            );
            let glass = Pos2::new(center.x + r * 0.34, center.y + r * 0.28);
            painter.circle_stroke(glass, r * 0.28, stroke);
            painter.line_segment(
                [
                    glass + Vec2::splat(r * 0.20),
                    glass + Vec2::splat(r * 0.44),
                ],
                stroke,
            );
        }
        IconKind::Cpu => {
            let core = Rect::from_center_size(center, Vec2::splat(size * 0.54));
            painter.rect_stroke(core, 2.0, stroke, eframe::egui::StrokeKind::Inside);
            let inner = Rect::from_center_size(center, Vec2::splat(size * 0.22));
            painter.rect_filled(inner, 1.0, color);
            for offset in [-r * 0.26, r * 0.26] {
                painter.line_segment(
                    [
                        Pos2::new(center.x + offset, core.top()),
                        Pos2::new(center.x + offset, center.y - r * 0.84),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(center.x + offset, core.bottom()),
                        Pos2::new(center.x + offset, center.y + r * 0.84),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(core.left(), center.y + offset),
                        Pos2::new(center.x - r * 0.84, center.y + offset),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(core.right(), center.y + offset),
                        Pos2::new(center.x + r * 0.84, center.y + offset),
                    ],
                    stroke,
                );
            }
        }
        IconKind::Tag => {
            let pts = vec![
                Pos2::new(center.x - r * 0.72, center.y - r * 0.72),
                Pos2::new(center.x - r * 0.04, center.y - r * 0.72),
                Pos2::new(center.x + r * 0.74, center.y + r * 0.06),
                Pos2::new(center.x + r * 0.06, center.y + r * 0.74),
                Pos2::new(center.x - r * 0.72, center.y - r * 0.04),
            ];
            painter.add(eframe::egui::Shape::closed_line(pts, stroke));
            painter.circle_filled(
                Pos2::new(center.x - r * 0.34, center.y - r * 0.34),
                r * 0.12,
                color,
            );
        }
        IconKind::Refresh => {
            painter.circle_stroke(center, r * 0.66, stroke);
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.36, center.y - r * 0.78),
                    Pos2::new(center.x + r * 0.68, center.y - r * 0.38),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.68, center.y - r * 0.38),
                    Pos2::new(center.x + r * 0.28, center.y - r * 0.30),
                ],
                stroke,
            );
        }
        IconKind::Settings => {
            painter.circle_stroke(center, r * 0.34, stroke);
            for i in 0..6 {
                let angle = (i as f32) * std::f32::consts::TAU / 6.0;
                let dir = Vec2::new(angle.cos(), angle.sin());
                painter.line_segment([center + dir * (r * 0.56), center + dir * (r * 0.84)], stroke);
            }
        }
        IconKind::Search => {
            let c = center + Vec2::new(-r * 0.14, -r * 0.14);
            painter.circle_stroke(c, r * 0.52, stroke);
            painter.line_segment(
                [c + Vec2::splat(r * 0.38), center + Vec2::splat(r * 0.74)],
                stroke,
            );
        }
        IconKind::Plus => {
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.64, center.y),
                    Pos2::new(center.x + r * 0.64, center.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - r * 0.64),
                    Pos2::new(center.x, center.y + r * 0.64),
                ],
                stroke,
            );
        }
        IconKind::Play => {
            let pts = vec![
                Pos2::new(center.x - r * 0.40, center.y - r * 0.56),
                Pos2::new(center.x + r * 0.62, center.y),
                Pos2::new(center.x - r * 0.40, center.y + r * 0.56),
            ];
            painter.add(eframe::egui::Shape::closed_line(pts, stroke));
        }
        IconKind::Edit => {
            // Crisp angled pencil outline
            let pts = vec![
                Pos2::new(center.x - r * 0.68, center.y + r * 0.68),
                Pos2::new(center.x - r * 0.56, center.y + r * 0.22),
                Pos2::new(center.x + r * 0.34, center.y - r * 0.68),
                Pos2::new(center.x + r * 0.68, center.y - r * 0.34),
                Pos2::new(center.x - r * 0.22, center.y + r * 0.56),
            ];
            painter.add(eframe::egui::Shape::closed_line(pts, stroke));
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.14, center.y - r * 0.48),
                    Pos2::new(center.x + r * 0.48, center.y - r * 0.14),
                ],
                stroke,
            );
        }
        IconKind::Trash => {
            // Lid
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.68, center.y - r * 0.38),
                    Pos2::new(center.x + r * 0.68, center.y - r * 0.38),
                ],
                stroke,
            );
            // Connected top handle
            let handle = vec![
                Pos2::new(center.x - r * 0.24, center.y - r * 0.38),
                Pos2::new(center.x - r * 0.20, center.y - r * 0.68),
                Pos2::new(center.x + r * 0.20, center.y - r * 0.68),
                Pos2::new(center.x + r * 0.24, center.y - r * 0.38),
            ];
            painter.add(eframe::egui::Shape::line(handle, stroke));
            // Bin body
            let body = vec![
                Pos2::new(center.x - r * 0.50, center.y - r * 0.38),
                Pos2::new(center.x - r * 0.42, center.y + r * 0.68),
                Pos2::new(center.x + r * 0.42, center.y + r * 0.68),
                Pos2::new(center.x + r * 0.50, center.y - r * 0.38),
            ];
            painter.add(eframe::egui::Shape::line(body, stroke));
            // Inner ribs
            for dx in [-r * 0.16, r * 0.16] {
                painter.line_segment(
                    [
                        Pos2::new(center.x + dx, center.y - r * 0.14),
                        Pos2::new(center.x + dx, center.y + r * 0.44),
                    ],
                    stroke,
                );
            }
        }
        IconKind::Check => {
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.60, center.y + r * 0.04),
                    Pos2::new(center.x - r * 0.14, center.y + r * 0.50),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.14, center.y + r * 0.50),
                    Pos2::new(center.x + r * 0.64, center.y - r * 0.46),
                ],
                stroke,
            );
        }
        IconKind::Eye => {
            let pts = vec![
                Pos2::new(center.x - r * 0.80, center.y),
                Pos2::new(center.x - r * 0.36, center.y - r * 0.44),
                Pos2::new(center.x + r * 0.36, center.y - r * 0.44),
                Pos2::new(center.x + r * 0.80, center.y),
                Pos2::new(center.x + r * 0.36, center.y + r * 0.44),
                Pos2::new(center.x - r * 0.36, center.y + r * 0.44),
            ];
            painter.add(eframe::egui::Shape::closed_line(pts, stroke));
            painter.circle_filled(center, r * 0.22, color);
        }
        IconKind::Sparkles => {
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - r * 0.78),
                    Pos2::new(center.x, center.y + r * 0.78),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.78, center.y),
                    Pos2::new(center.x + r * 0.78, center.y),
                ],
                stroke,
            );
            painter.circle_filled(center, r * 0.20, color);
        }
        IconKind::ImagePlus => {
            let frame = Rect::from_center_size(center, Vec2::splat(size * 0.78));
            painter.rect_stroke(frame, 2.5, stroke, eframe::egui::StrokeKind::Inside);
            painter.circle_filled(
                Pos2::new(center.x - r * 0.28, center.y - r * 0.26),
                r * 0.12,
                color,
            );
            painter.line_segment(
                [
                    Pos2::new(frame.left() + 2.0, frame.bottom() - r * 0.25),
                    Pos2::new(center.x + r * 0.08, center.y + r * 0.05),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.08, center.y + r * 0.05),
                    Pos2::new(frame.right() - 2.0, frame.bottom() - r * 0.18),
                ],
                stroke,
            );
        }
        IconKind::Terminal => {
            let box_rect = Rect::from_center_size(center, Vec2::new(size * 0.86, size * 0.68));
            painter.rect_stroke(box_rect, 2.5, stroke, eframe::egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.48, center.y - r * 0.22),
                    Pos2::new(center.x - r * 0.16, center.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.16, center.y),
                    Pos2::new(center.x - r * 0.48, center.y + r * 0.22),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.04, center.y + r * 0.22),
                    Pos2::new(center.x + r * 0.46, center.y + r * 0.22),
                ],
                stroke,
            );
        }
        IconKind::RotateCcw => {
            painter.circle_stroke(center, r * 0.64, stroke);
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.64, center.y - r * 0.64),
                    Pos2::new(center.x - r * 0.64, center.y - r * 0.14),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.64, center.y - r * 0.14),
                    Pos2::new(center.x - r * 0.16, center.y - r * 0.14),
                ],
                stroke,
            );
        }
        IconKind::Info => {
            painter.circle_stroke(center, r * 0.78, stroke);
            painter.circle_filled(Pos2::new(center.x, center.y - r * 0.34), r * 0.10, color);
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - r * 0.08),
                    Pos2::new(center.x, center.y + r * 0.44),
                ],
                stroke,
            );
        }
        IconKind::Alert => {
            painter.circle_stroke(center, r * 0.78, stroke);
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - r * 0.40),
                    Pos2::new(center.x, center.y + r * 0.10),
                ],
                stroke,
            );
            painter.circle_filled(Pos2::new(center.x, center.y + r * 0.38), r * 0.10, color);
        }
    }
}
