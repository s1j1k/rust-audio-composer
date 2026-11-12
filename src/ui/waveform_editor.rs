//! Waveform view and editor for single-note samples.
//!
//! The widget draws straight onto egui's painter: when zoomed out each pixel column shows
//! the min..max of the samples under it (the standard "peak" display); zoomed in it draws
//! the samples themselves as a line, with dots once each sample is a few pixels wide.
//!
//! Editing never touches undo directly. Whenever an edit starts, the widget hands back a
//! copy of the buffer as it was (`WaveResponse::before`) and the caller decides where that
//! goes. That keeps this module independent of the designer and easy to test.

use crate::audio::note_render::normalize;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WaveTool {
    #[default]
    Select,
    Pencil,
}

/// Editing operations. Each applies to the selection, or to the whole note if none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveOp {
    FadeIn,
    FadeOut,
    Normalize,
    Reverse,
    Silence,
    /// Gain change in decibels.
    Gain(f32),
    /// Keep only the selected range.
    Crop,
    /// Remove the selected range.
    Delete,
}

#[derive(Debug, Default)]
pub struct WaveEditorState {
    pub tool: WaveTool,
    /// First visible sample and number of visible samples. `view_len == 0` means "fit".
    view_start: f64,
    view_len: f64,
    /// Selected sample range, start < end.
    pub selection: Option<(usize, usize)>,
    drag_anchor: Option<usize>,
    last_pencil: Option<(usize, f32)>,
}

/// What happened this frame.
#[derive(Debug, Default)]
pub struct WaveResponse {
    /// The buffer as it was before an edit that started this frame (push it to undo).
    pub before: Option<Vec<f32>>,
    /// An edit finished this frame (pencil released or a tool applied): a good moment to
    /// preview the sound.
    pub committed: bool,
}

impl WaveEditorState {
    /// Show the whole note and clear the selection (after loading a new sound).
    pub fn reset_view(&mut self) {
        self.view_start = 0.0;
        self.view_len = 0.0;
        self.selection = None;
        self.drag_anchor = None;
        self.last_pencil = None;
    }

    fn clamp_view(&mut self, n: usize) {
        let n = n.max(1) as f64;
        if self.view_len <= 0.0 || self.view_len > n {
            self.view_len = n;
        }
        // Never zoom in past ~8 samples across the whole width.
        self.view_len = self.view_len.max(n.min(8.0));
        self.view_start = self.view_start.clamp(0.0, n - self.view_len);
        if let Some((a, b)) = self.selection {
            let n = n as usize;
            self.selection = (a.min(n) < b.min(n)).then_some((a.min(n), b.min(n)));
        }
    }

    /// Zoom by `factor` (> 1 zooms in) keeping sample `anchor` under the same pixel.
    fn zoom(&mut self, factor: f64, anchor: f64, n: usize) {
        let t = (anchor - self.view_start) / self.view_len;
        self.view_len /= factor;
        self.clamp_view(n);
        self.view_start = anchor - t * self.view_len;
        self.clamp_view(n);
    }
}

/// Set every sample between two points on a straight line (so a fast mouse drag leaves no
/// gaps).
pub fn draw_line(data: &mut [f32], from: (usize, f32), to: (usize, f32)) {
    if data.is_empty() {
        return;
    }
    let (a, b) = if from.0 <= to.0 { (from, to) } else { (to, from) };
    let last = data.len() - 1;
    let span = (b.0 - a.0).max(1) as f32;
    for i in a.0.min(last)..=b.0.min(last) {
        let t = (i - a.0) as f32 / span;
        data[i] = (a.1 + (b.1 - a.1) * t).clamp(-1.0, 1.0);
    }
}

/// Apply `op` to `range` (clamped to the buffer). The buffer never ends up empty.
pub fn apply_op(data: &mut Vec<f32>, op: WaveOp, range: (usize, usize)) {
    let b = range.1.min(data.len());
    let a = range.0.min(b);
    let n = b - a;
    match op {
        WaveOp::FadeIn => {
            for (k, s) in data[a..b].iter_mut().enumerate() {
                *s *= k as f32 / n.max(1) as f32;
            }
        }
        WaveOp::FadeOut => {
            for (k, s) in data[a..b].iter_mut().enumerate() {
                *s *= 1.0 - (k + 1) as f32 / n.max(1) as f32;
            }
        }
        WaveOp::Normalize => normalize(&mut data[a..b], 0.95),
        WaveOp::Reverse => data[a..b].reverse(),
        WaveOp::Silence => data[a..b].fill(0.0),
        WaveOp::Gain(db) => {
            let g = 10f32.powf(db / 20.0);
            for s in &mut data[a..b] {
                *s = (*s * g).clamp(-1.0, 1.0);
            }
        }
        WaveOp::Crop => {
            data.truncate(b);
            data.drain(..a);
        }
        WaveOp::Delete => {
            data.drain(a..b);
        }
    }
    if data.is_empty() {
        data.push(0.0);
    }
}

/// The full editor: toolbar, waveform canvas, scroll bar and edit tools.
pub fn show(ui: &mut Ui, st: &mut WaveEditorState, data: &mut Vec<f32>, sample_rate: u32) -> WaveResponse {
    let mut out = WaveResponse::default();
    let sr = sample_rate.max(1) as f64;
    if data.is_empty() {
        data.push(0.0); // e.g. a damaged project file; the canvas needs one sample
    }
    st.clamp_view(data.len());

    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut st.tool, WaveTool::Select, "↔ Select")
            .on_hover_text("Drag across the waveform to select a range. Tools below apply to the selection, or to the whole note.");
        ui.selectable_value(&mut st.tool, WaveTool::Pencil, "✏ Draw")
            .on_hover_text("Drag to redraw the waveform by hand. Zoom in to draw individual samples.");
        ui.separator();
        let n = data.len();
        let centre = st.view_start + st.view_len / 2.0;
        if ui.small_button(" + ").on_hover_text("Zoom in (Ctrl + wheel)").clicked() {
            st.zoom(2.0, centre, n);
        }
        if ui.small_button(" - ").on_hover_text("Zoom out").clicked() {
            st.zoom(0.5, centre, n);
        }
        if ui.small_button("Fit").on_hover_text("Show the whole note").clicked() {
            st.view_len = 0.0;
            st.clamp_view(n);
        }
        if ui
            .add_enabled(st.selection.is_some(), egui::Button::new("Zoom to selection").small())
            .clicked()
        {
            if let Some((a, b)) = st.selection {
                st.view_start = a as f64;
                st.view_len = (b - a) as f64;
                st.clamp_view(n);
            }
        }
    });

    // ---- Canvas -------------------------------------------------------------------
    let width = ui.available_width().max(200.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 170.0), Sense::click_and_drag());
    let n = data.len();

    // Zoom (Ctrl+wheel arrives as zoom_delta) and pan (Shift+wheel arrives as horizontal
    // scroll) while hovering. Handled first so the drawing below reflects it.
    if resp.hovered() {
        let (zoom, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
        if let Some(p) = resp.hover_pos() {
            if (zoom - 1.0).abs() > 1e-4 {
                let anchor = st.view_start + (p.x - rect.left()) as f64 / rect.width() as f64 * st.view_len;
                st.zoom(zoom as f64, anchor, n);
            }
        }
        if scroll.x != 0.0 {
            st.view_start -= scroll.x as f64 * st.view_len / rect.width() as f64;
            st.clamp_view(n);
        }
    }

    let (vs, vl) = (st.view_start, st.view_len);
    let spp = vl / rect.width() as f64; // samples per pixel
    let mid = rect.center().y;
    let half = rect.height() * 0.5 - 8.0;
    let y_of = |v: f32| mid - v.clamp(-1.0, 1.0) * half;
    let v_of = |y: f32| ((mid - y) / half).clamp(-1.0, 1.0);
    let x_of = |i: f64| rect.left() + ((i - vs) / spp) as f32;
    let idx_at = |x: f32| -> usize {
        let i = vs + (x - rect.left()) as f64 * spp;
        (i.round() as i64).clamp(0, n as i64 - 1) as usize
    };

    match st.tool {
        WaveTool::Select => {
            if resp.drag_started() {
                st.drag_anchor = resp.interact_pointer_pos().map(|p| idx_at(p.x));
            }
            if resp.dragged() {
                if let (Some(a), Some(p)) = (st.drag_anchor, resp.interact_pointer_pos()) {
                    let b = idx_at(p.x);
                    st.selection = Some((a.min(b), a.max(b)));
                }
            }
            if resp.drag_stopped() {
                st.drag_anchor = None;
                if matches!(st.selection, Some((a, b)) if b - a < 2) {
                    st.selection = None;
                }
            }
            if resp.clicked() {
                st.selection = None;
            }
        }
        WaveTool::Pencil => {
            let pencil_at = |p: Pos2| (idx_at(p.x), v_of(p.y));
            if resp.drag_started() || resp.clicked() {
                out.before = Some(data.clone());
                st.last_pencil = None;
            }
            if resp.drag_started() || resp.dragged() || resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    let pt = pencil_at(p);
                    draw_line(data, st.last_pencil.unwrap_or(pt), pt);
                    st.last_pencil = Some(pt);
                }
            }
            if resp.drag_stopped() || resp.clicked() {
                st.last_pencil = None;
                out.committed = true;
            }
        }
    }

    // ---- Drawing ------------------------------------------------------------------
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 20, 27));
    let grid = Stroke::new(1.0, Color32::from_gray(45));
    painter.hline(rect.x_range(), mid, Stroke::new(1.0, Color32::from_gray(70)));
    for v in [-0.5, 0.5] {
        painter.hline(rect.x_range(), y_of(v), grid);
    }

    if let Some((a, b)) = st.selection {
        let sel = Rect::from_x_y_ranges(x_of(a as f64)..=x_of(b as f64), rect.y_range()).intersect(rect);
        painter.rect_filled(sel, 0.0, Color32::from_rgba_unmultiplied(90, 140, 255, 45));
    }

    let wave = Color32::from_rgb(110, 195, 255);
    if spp > 1.0 {
        // Peak display: one vertical min..max line per pixel column.
        let cols = rect.width() as usize;
        for px in 0..cols {
            let s0 = (vs + px as f64 * spp) as usize;
            let s1 = ((vs + (px + 1) as f64 * spp) as usize).max(s0 + 1).min(n);
            if s0 >= n {
                break;
            }
            let (lo, hi) = data[s0..s1]
                .iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &s| (lo.min(s), hi.max(s)));
            let x = rect.left() + px as f32 + 0.5;
            let (y0, y1) = (y_of(hi), y_of(lo).max(y_of(hi) + 1.0));
            painter.line_segment([Pos2::new(x, y0), Pos2::new(x, y1)], Stroke::new(1.0, wave));
        }
    } else {
        // Zoomed in: draw the actual samples.
        let first = vs.floor() as usize;
        let last = ((vs + vl).ceil() as usize + 1).min(n);
        let pts: Vec<Pos2> = (first..last).map(|i| Pos2::new(x_of(i as f64), y_of(data[i]))).collect();
        if 1.0 / spp >= 6.0 {
            for p in &pts {
                painter.circle_filled(*p, 2.0, wave);
            }
        }
        painter.add(Shape::line(pts, Stroke::new(1.5, wave)));
    }

    // Labels: visible time span, and a read-out under the mouse.
    let label = FontId::monospace(10.0);
    let dim = Color32::from_gray(140);
    let ms = |i: f64| i / sr * 1000.0;
    painter.text(rect.left_top() + egui::vec2(4.0, 2.0), Align2::LEFT_TOP, format!("{:.1} ms", ms(vs)), label.clone(), dim);
    painter.text(rect.right_top() + egui::vec2(-4.0, 2.0), Align2::RIGHT_TOP, format!("{:.1} ms", ms(vs + vl)), label.clone(), dim);
    let zoom_txt = if spp >= 1.0 {
        format!("{spp:.0} samples/px")
    } else {
        format!("{:.0} px/sample", 1.0 / spp)
    };
    painter.text(rect.left_bottom() + egui::vec2(4.0, -2.0), Align2::LEFT_BOTTOM, zoom_txt, label.clone(), dim);
    if let Some(p) = resp.hover_pos() {
        let i = idx_at(p.x);
        if st.tool == WaveTool::Pencil {
            painter.vline(p.x, rect.y_range(), Stroke::new(1.0, Color32::from_gray(90)));
        }
        painter.text(
            rect.right_bottom() + egui::vec2(-4.0, -2.0),
            Align2::RIGHT_BOTTOM,
            format!("{:.2} ms  {:+.3}", ms(i as f64), data[i]),
            label,
            dim,
        );
    }

    // Scroll bar when zoomed in.
    if vl < n as f64 - 0.5 {
        let max = n as f64 - vl;
        ui.spacing_mut().slider_width = (width - 16.0).max(50.0);
        ui.add(egui::Slider::new(&mut st.view_start, 0.0..=max).show_value(false));
    }

    // ---- Edit tools -----------------------------------------------------------------
    let has_sel = st.selection.is_some();
    let range = st.selection.unwrap_or((0, data.len()));
    let mut op = None;
    ui.horizontal_wrapped(|ui| {
        let scope = if has_sel {
            format!("Selection ({:.1} ms):", (range.1 - range.0) as f64 / sr * 1000.0)
        } else {
            "Whole note:".to_string()
        };
        ui.label(RichText::new(scope).small().color(Color32::GRAY));
        for (label, o, tip) in [
            ("Fade in", WaveOp::FadeIn, "Ramp up from silence"),
            ("Fade out", WaveOp::FadeOut, "Ramp down to silence"),
            ("Normalize", WaveOp::Normalize, "Make the loudest point full level"),
            ("+3 dB", WaveOp::Gain(3.0), "Louder"),
            ("−3 dB", WaveOp::Gain(-3.0), "Quieter"),
            ("Reverse", WaveOp::Reverse, "Play backwards"),
            ("Silence", WaveOp::Silence, "Set to zero"),
        ] {
            if ui.button(label).on_hover_text(tip).clicked() {
                op = Some(o);
            }
        }
        if ui
            .add_enabled(has_sel, egui::Button::new("Crop"))
            .on_hover_text("Keep only the selection")
            .clicked()
        {
            op = Some(WaveOp::Crop);
        }
        if ui
            .add_enabled(has_sel, egui::Button::new("Delete"))
            .on_hover_text("Cut the selection out")
            .clicked()
        {
            op = Some(WaveOp::Delete);
        }
    });
    if let Some(op) = op {
        out.before = Some(data.clone());
        apply_op(data, op, range);
        out.committed = true;
        if matches!(op, WaveOp::Crop | WaveOp::Delete) {
            st.reset_view();
        }
    }

    out
}

/// Tiny non-interactive waveform for lists.
pub fn thumbnail(ui: &mut Ui, data: &[f32], size: egui::Vec2) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_rgb(18, 20, 27));
    if data.is_empty() {
        return resp;
    }
    let cols = rect.width().max(1.0) as usize;
    let per = (data.len() as f32 / cols as f32).max(1.0);
    let (mid, half) = (rect.center().y, rect.height() * 0.5 - 1.0);
    for px in 0..cols {
        let s0 = (px as f32 * per) as usize;
        let s1 = (((px + 1) as f32 * per) as usize).clamp(s0 + 1, data.len());
        if s0 >= data.len() {
            break;
        }
        let peak = data[s0..s1].iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        let x = rect.left() + px as f32 + 0.5;
        painter.line_segment(
            [Pos2::new(x, mid - peak * half), Pos2::new(x, mid + peak * half)],
            Stroke::new(1.0, Color32::from_rgb(110, 195, 255)),
        );
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pencil_line_fills_every_sample_between_points() {
        let mut d = vec![0.0; 10];
        draw_line(&mut d, (8, -1.0), (2, 0.5)); // right-to-left drag
        assert_eq!(d[2], 0.5);
        assert_eq!(d[8], -1.0);
        assert!(d[3] < 0.5 && d[3] > -1.0);
        assert_eq!(d[0], 0.0);
        assert_eq!(d[9], 0.0);
        draw_line(&mut d, (5, 0.2), (50, 0.2)); // past the end is clamped, not a panic
        assert_eq!(d[9], 0.2);
    }

    #[test]
    fn ops_apply_to_the_range_only() {
        let mut d = vec![1.0; 10];
        apply_op(&mut d, WaveOp::Silence, (2, 4));
        assert_eq!(d, [1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);

        let mut d = vec![1.0; 4];
        apply_op(&mut d, WaveOp::FadeIn, (0, 4));
        assert_eq!(d, [0.0, 0.25, 0.5, 0.75]);

        let mut d = vec![1.0; 4];
        apply_op(&mut d, WaveOp::FadeOut, (0, 4));
        assert_eq!(d, [0.75, 0.5, 0.25, 0.0]);

        let mut d = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        apply_op(&mut d, WaveOp::Crop, (1, 3));
        assert_eq!(d, [1.0, 2.0]);

        let mut d = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        apply_op(&mut d, WaveOp::Delete, (1, 3));
        assert_eq!(d, [0.0, 3.0, 4.0]);

        let mut d = vec![0.1, -0.2];
        apply_op(&mut d, WaveOp::Normalize, (0, 2));
        assert!((d[1] + 0.95).abs() < 1e-6);

        let mut d = vec![0.5; 3];
        apply_op(&mut d, WaveOp::Delete, (0, 3));
        assert_eq!(d, [0.0], "never leaves an empty buffer");
    }

    #[test]
    fn zoom_keeps_anchor_in_place_and_stays_in_bounds() {
        let mut st = WaveEditorState::default();
        st.clamp_view(1000);
        assert_eq!(st.view_len, 1000.0);
        st.zoom(4.0, 800.0, 1000);
        assert_eq!(st.view_len, 250.0);
        // 800 was 80% across before; it should still be 80% across.
        assert!(((800.0 - st.view_start) / st.view_len - 0.8).abs() < 1e-9);
        st.zoom(1e6, 0.0, 1000);
        assert_eq!(st.view_len, 8.0);
        st.zoom(1e-6, 0.0, 1000);
        assert_eq!((st.view_start, st.view_len), (0.0, 1000.0));
    }
}
