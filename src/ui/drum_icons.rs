//! Tiny vector drum icons drawn with egui's painter: crisp at any UI scale, recolourable,
//! no image assets or extra dependencies. Each fits a square `rect`.

use crate::model::drum::DrumKind;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke};

pub fn paint_drum_icon(painter: &Painter, rect: Rect, kind: DrumKind, color: Color32) {
    let r = rect.shrink(rect.width() * 0.08);
    let c = r.center();
    let w = r.width();
    let s = Stroke::new((w * 0.075).max(1.2), color);
    let fill = color.gamma_multiply(0.35);

    match kind {
        DrumKind::Kick => {
            // Bass drum head with a beater.
            let rad = w * 0.36;
            let cc = c + vec2(0.0, w * 0.06);
            painter.circle(cc, rad, fill, s);
            painter.circle_stroke(cc, rad * 0.55, Stroke::new(s.width * 0.7, color));
            painter.line_segment([cc + vec2(-rad * 0.2, -rad * 1.35), cc + vec2(0.0, -rad * 0.45)], s);
            painter.circle_filled(cc + vec2(-rad * 0.2, -rad * 1.35), w * 0.07, color);
        }
        DrumKind::Snare => {
            drum_shell(painter, c, w * 0.42, w * 0.16, w * 0.22, fill, s);
            // Crossed sticks.
            painter.line_segment([c + vec2(-w * 0.42, -w * 0.46), c + vec2(w * 0.05, -w * 0.12)], s);
            painter.line_segment([c + vec2(w * 0.42, -w * 0.46), c + vec2(-w * 0.05, -w * 0.12)], s);
        }
        DrumKind::LowTom => drum_shell(painter, c + vec2(0.0, w * 0.04), w * 0.44, w * 0.14, w * 0.34, fill, s),
        DrumKind::MidTom => drum_shell(painter, c + vec2(0.0, w * 0.04), w * 0.36, w * 0.12, w * 0.26, fill, s),
        DrumKind::HighTom => drum_shell(painter, c + vec2(0.0, w * 0.04), w * 0.28, w * 0.10, w * 0.20, fill, s),
        DrumKind::ClosedHat => {
            stand(painter, c, w, s);
            let y = c.y - w * 0.12;
            cymbal(painter, pos2(c.x, y - w * 0.05), w * 0.4, w * 0.07, fill, s);
            cymbal(painter, pos2(c.x, y + w * 0.04), w * 0.4, w * 0.07, fill, s);
        }
        DrumKind::OpenHat => {
            stand(painter, c, w, s);
            let y = c.y - w * 0.12;
            cymbal(painter, pos2(c.x, y - w * 0.16), w * 0.4, w * 0.07, fill, s);
            cymbal(painter, pos2(c.x, y + w * 0.08), w * 0.4, w * 0.07, fill, s);
            // "Open" sizzle marks.
            for dx in [-0.3, 0.3] {
                painter.line_segment(
                    [pos2(c.x + w * dx, y - w * 0.04), pos2(c.x + w * dx * 1.25, y - w * 0.04)],
                    Stroke::new(s.width * 0.7, color),
                );
            }
        }
        DrumKind::Cymbal => {
            stand(painter, c, w, s);
            // Tilted ride/crash with a bell.
            let top = c + vec2(0.0, -w * 0.18);
            let pts = ellipse_points(top, w * 0.46, w * 0.1, -0.22, 24);
            painter.add(Shape::convex_polygon(pts, fill, s));
            painter.circle_filled(top + vec2(0.0, -w * 0.02), w * 0.07, color);
        }
        DrumKind::Clap => {
            // Two palms meeting, with burst lines.
            let palm = |angle: f32, off: f32| {
                let base = c + vec2(off, w * 0.12);
                rotated_rect(base, vec2(w * 0.2, w * 0.42), angle)
            };
            painter.add(Shape::convex_polygon(palm(-0.35, -w * 0.11), fill, s));
            painter.add(Shape::convex_polygon(palm(0.35, w * 0.11), fill, s));
            for (dx, dy) in [(-0.3, -0.36), (0.0, -0.44), (0.3, -0.36)] {
                let p = c + vec2(w * dx, w * dy);
                let dir = (p - (c + vec2(0.0, -w * 0.1))).normalized() * (w * 0.09);
                painter.line_segment([p, p + dir], Stroke::new(s.width * 0.8, color));
            }
        }
        DrumKind::Shaker => {
            // Egg shaker with motion lines.
            let pts = ellipse_points(c + vec2(w * 0.06, 0.0), w * 0.2, w * 0.3, 0.5, 24);
            painter.add(Shape::convex_polygon(pts, fill, s));
            for i in 0..3 {
                let y = c.y - w * 0.18 + i as f32 * w * 0.18;
                painter.line_segment(
                    [pos2(c.x - w * 0.46, y), pos2(c.x - w * 0.28, y)],
                    Stroke::new(s.width * 0.7, color),
                );
            }
        }
    }
}

/// Side view of a drum: top ellipse, shell, bottom rim.
fn drum_shell(painter: &Painter, c: Pos2, half_w: f32, rim: f32, depth: f32, fill: Color32, s: Stroke) {
    let top = c + vec2(0.0, -depth * 0.5);
    let bottom = c + vec2(0.0, depth * 0.5);
    let body = Rect::from_min_max(pos2(c.x - half_w, top.y), pos2(c.x + half_w, bottom.y));
    painter.rect_filled(body, 0.0, fill);
    painter.line_segment([body.left_top(), body.left_bottom()], s);
    painter.line_segment([body.right_top(), body.right_bottom()], s);
    painter.add(Shape::closed_line(
        ellipse_points(bottom, half_w, rim, 0.0, 24).into_iter().skip(12).collect(),
        s,
    ));
    painter.add(Shape::convex_polygon(ellipse_points(top, half_w, rim, 0.0, 24), fill, s));
}

fn cymbal(painter: &Painter, c: Pos2, half_w: f32, half_h: f32, fill: Color32, s: Stroke) {
    painter.add(Shape::convex_polygon(ellipse_points(c, half_w, half_h, 0.0, 20), fill, s));
}

fn stand(painter: &Painter, c: Pos2, w: f32, s: Stroke) {
    let thin = Stroke::new(s.width * 0.7, s.color);
    painter.line_segment([c + vec2(0.0, -w * 0.2), c + vec2(0.0, w * 0.4)], thin);
    painter.line_segment([c + vec2(0.0, w * 0.3), c + vec2(-w * 0.2, w * 0.44)], thin);
    painter.line_segment([c + vec2(0.0, w * 0.3), c + vec2(w * 0.2, w * 0.44)], thin);
}

fn ellipse_points(c: Pos2, rx: f32, ry: f32, rot: f32, n: usize) -> Vec<Pos2> {
    let (sn, cs) = rot.sin_cos();
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            let (x, y) = (rx * t.cos(), ry * t.sin());
            pos2(c.x + x * cs - y * sn, c.y + x * sn + y * cs)
        })
        .collect()
}

fn rotated_rect(c: Pos2, size: egui::Vec2, rot: f32) -> Vec<Pos2> {
    let (sn, cs) = rot.sin_cos();
    let (hx, hy) = (size.x * 0.5, size.y * 0.5);
    [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
        .iter()
        .map(|&(x, y)| pos2(c.x + x * cs - y * sn, c.y + x * sn + y * cs))
        .collect()
}
