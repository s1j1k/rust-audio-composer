//! Effects over time: automation lanes.
//!
//! A lane is a list of `(beat, value)` breakpoints for one mix parameter. Between points the
//! value ramps linearly; before the first point it holds the first value and after the last
//! point it holds the last one. A lane with no points does nothing: the control's fixed
//! value applies instead. That keeps old projects (no lanes) sounding exactly as before.

use super::effects::TrackEffects;
use serde::{Deserialize, Serialize};

/// A mix parameter that can change over time on a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FxParam {
    /// Channel fader (track volume).
    Volume,
    EqHigh,
    EqMid,
    EqLow,
    /// DJ colour filter: left = low-pass (muffled), right = high-pass (thin), centre = off.
    Filter,
    Echo,
    Reverb,
    Phaser,
    Distance,
}

impl FxParam {
    pub fn all() -> &'static [FxParam] {
        &[
            Self::Volume,
            Self::Filter,
            Self::EqHigh,
            Self::EqMid,
            Self::EqLow,
            Self::Echo,
            Self::Reverb,
            Self::Phaser,
            Self::Distance,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Volume => "Volume (fader)",
            Self::EqHigh => "EQ high",
            Self::EqMid => "EQ mid",
            Self::EqLow => "EQ low",
            Self::Filter => "Filter",
            Self::Echo => "Echo",
            Self::Reverb => "Reverb",
            Self::Phaser => "Phaser",
            Self::Distance => "Distance",
        }
    }

    /// Short name for knob captions and the phone.
    pub fn short(self) -> &'static str {
        match self {
            Self::Volume => "VOL",
            Self::EqHigh => "HI",
            Self::EqMid => "MID",
            Self::EqLow => "LOW",
            Self::Filter => "FILTER",
            Self::Echo => "ECHO",
            Self::Reverb => "VERB",
            Self::Phaser => "PHASE",
            Self::Distance => "FAR",
        }
    }

    /// Wire / file key (matches the serde name).
    pub fn key(self) -> &'static str {
        match self {
            Self::Volume => "volume",
            Self::EqHigh => "eq_high",
            Self::EqMid => "eq_mid",
            Self::EqLow => "eq_low",
            Self::Filter => "filter",
            Self::Echo => "echo",
            Self::Reverb => "reverb",
            Self::Phaser => "phaser",
            Self::Distance => "distance",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().copied().find(|p| p.key() == key)
    }

    /// Value range of the parameter.
    pub fn range(self) -> (f32, f32) {
        match self {
            Self::EqHigh | Self::EqMid | Self::EqLow | Self::Filter => (-1.0, 1.0),
            _ => (0.0, 1.0),
        }
    }

    /// Centre-detent controls (EQ, filter) rest in the middle of their range.
    pub fn bipolar(self) -> bool {
        self.range().0 < 0.0
    }

    /// Neutral value (what double-click resets to).
    pub fn neutral(self) -> f32 {
        match self {
            Self::Volume => 0.8,
            _ => 0.0,
        }
    }

    pub fn clamp(self, v: f32) -> f32 {
        let (lo, hi) = self.range();
        v.clamp(lo, hi)
    }

    /// Read this parameter from a track's static settings.
    pub fn get(self, fx: &TrackEffects, volume: f32) -> f32 {
        match self {
            Self::Volume => volume,
            Self::EqHigh => fx.eq_high,
            Self::EqMid => fx.eq_mid,
            Self::EqLow => fx.eq_low,
            Self::Filter => fx.filter,
            Self::Echo => fx.echo,
            Self::Reverb => fx.reverb,
            Self::Phaser => fx.phaser,
            Self::Distance => fx.distance,
        }
    }

    /// Write this parameter into a track's settings.
    pub fn set(self, fx: &mut TrackEffects, volume: &mut f32, v: f32) {
        let v = self.clamp(v);
        match self {
            Self::Volume => *volume = v,
            Self::EqHigh => fx.eq_high = v,
            Self::EqMid => fx.eq_mid = v,
            Self::EqLow => fx.eq_low = v,
            Self::Filter => fx.filter = v,
            Self::Echo => fx.echo = v,
            Self::Reverb => fx.reverb = v,
            Self::Phaser => fx.phaser = v,
            Self::Distance => fx.distance = v,
        }
    }

    /// Human-readable value for tooltips.
    pub fn describe(self, v: f32) -> String {
        match self {
            Self::EqHigh | Self::EqMid | Self::EqLow => {
                if v <= -0.98 {
                    "kill".to_string()
                } else {
                    format!("{:+.1} dB", 20.0 * crate::model::effects::eq_gain(v).log10())
                }
            }
            Self::Filter => {
                if v.abs() < crate::model::effects::FILTER_DEAD_ZONE {
                    "off".to_string()
                } else if v < 0.0 {
                    format!("low-pass {:.0}%", -v * 100.0)
                } else {
                    format!("high-pass {:.0}%", v * 100.0)
                }
            }
            _ => format!("{:.0}%", v * 100.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationPoint {
    pub beat: f64,
    pub value: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AutomationLane {
    pub points: Vec<AutomationPoint>,
}

/// Points closer than this (in beats) are treated as the same spot.
pub const POINT_EPSILON: f64 = 1.0 / 64.0;
/// Recorded knob moves keep at most one point per this many beats.
pub const RECORD_RESOLUTION: f64 = 1.0 / 32.0;

impl AutomationLane {
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Value at `beat`, or `None` if the lane has no points.
    pub fn value_at(&self, beat: f64) -> Option<f32> {
        let pts = &self.points;
        let first = pts.first()?;
        if beat <= first.beat {
            return Some(first.value);
        }
        let last = pts.last()?;
        if beat >= last.beat {
            return Some(last.value);
        }
        // First point strictly after `beat`; it has a predecessor because beat > first.beat.
        let i = pts.partition_point(|p| p.beat <= beat);
        let (a, b) = (pts[i - 1], pts[i]);
        let span = b.beat - a.beat;
        if span <= 1e-9 {
            return Some(b.value);
        }
        let t = ((beat - a.beat) / span) as f32;
        Some(a.value + (b.value - a.value) * t)
    }

    fn sort(&mut self) {
        self.points
            .sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Add a point, replacing any point within [`POINT_EPSILON`] of the same beat.
    /// Returns the index of the point.
    pub fn set_point(&mut self, beat: f64, value: f32) -> usize {
        let beat = beat.max(0.0);
        if let Some(i) = self.nearest_point(beat, POINT_EPSILON) {
            self.points[i] = AutomationPoint { beat, value };
            self.sort();
        } else {
            self.points.push(AutomationPoint { beat, value });
            self.sort();
        }
        self.points
            .iter()
            .position(|p| p.beat == beat)
            .unwrap_or(0)
    }

    /// Index of the point nearest `beat` within `max_dist` beats.
    pub fn nearest_point(&self, beat: f64, max_dist: f64) -> Option<usize> {
        self.points
            .iter()
            .enumerate()
            .map(|(i, p)| (i, (p.beat - beat).abs()))
            .filter(|(_, d)| *d <= max_dist)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }

    pub fn remove_point(&mut self, index: usize) {
        if index < self.points.len() {
            self.points.remove(index);
        }
    }

    /// Move a point, keeping it between its neighbours so the lane stays sorted.
    pub fn move_point(&mut self, index: usize, beat: f64, value: f32) {
        let n = self.points.len();
        if index >= n {
            return;
        }
        let lo = if index > 0 { self.points[index - 1].beat } else { 0.0 };
        let hi = if index + 1 < n { self.points[index + 1].beat } else { f64::MAX };
        self.points[index] = AutomationPoint {
            beat: beat.clamp(lo, hi),
            value,
        };
    }

    /// Remove every point with `from < beat <= to`.
    pub fn clear_range(&mut self, from: f64, to: f64) {
        self.points.retain(|p| !(p.beat > from && p.beat <= to));
    }

    /// Record one step of a live knob move: overwrite whatever was in `(from, to]` and
    /// end the segment at `(to, value)`. Consecutive steps closer than
    /// [`RECORD_RESOLUTION`] are merged so a 60 fps UI doesn't flood the lane, and a
    /// held (unchanging) knob keeps just two points for the flat stretch.
    pub fn record_step(&mut self, from: f64, to: f64, value: f32) {
        if to < from {
            return;
        }
        self.clear_range(from, to);
        // The point at `from` is the previous step of this same gesture (if any).
        let prev = self.points.iter().position(|p| (p.beat - from).abs() < 1e-9);
        if let Some(i) = prev {
            let before = i.checked_sub(1).map(|j| self.points[j]);
            let p = self.points[i];
            let flat = (p.value - value).abs() < 1e-4
                && before.map(|b| (b.value - value).abs() < 1e-4).unwrap_or(false);
            let too_close = to - p.beat < RECORD_RESOLUTION
                && before.map(|b| p.beat - b.beat < RECORD_RESOLUTION * 2.0).unwrap_or(false);
            if flat || too_close {
                // Slide the previous point forward instead of adding another.
                self.points[i] = AutomationPoint { beat: to, value };
                return;
            }
        }
        self.points.push(AutomationPoint { beat: to, value });
        self.sort();
    }
}

/// A lane tied to one track parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackLane {
    pub param: FxParam,
    #[serde(flatten)]
    pub lane: AutomationLane,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane(points: &[(f64, f32)]) -> AutomationLane {
        AutomationLane {
            points: points
                .iter()
                .map(|&(beat, value)| AutomationPoint { beat, value })
                .collect(),
        }
    }

    #[test]
    fn empty_lane_has_no_value() {
        assert_eq!(AutomationLane::default().value_at(3.0), None);
    }

    #[test]
    fn interpolates_and_holds_ends() {
        let l = lane(&[(4.0, 0.0), (8.0, 1.0), (12.0, 0.5)]);
        assert_eq!(l.value_at(0.0), Some(0.0)); // before first: hold
        assert_eq!(l.value_at(4.0), Some(0.0));
        assert!((l.value_at(6.0).unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(l.value_at(8.0), Some(1.0));
        assert!((l.value_at(10.0).unwrap() - 0.75).abs() < 1e-6);
        assert_eq!(l.value_at(100.0), Some(0.5)); // after last: hold
    }

    #[test]
    fn vertical_step_takes_later_value() {
        // Two points on the same beat make an instant jump.
        let l = lane(&[(0.0, 0.0), (4.0, 0.2), (4.0, 0.9), (8.0, 0.9)]);
        assert!((l.value_at(4.0001).unwrap() - 0.9).abs() < 1e-3);
        assert!((l.value_at(3.9999).unwrap() - 0.2).abs() < 1e-3);
    }

    #[test]
    fn set_point_replaces_nearby_and_keeps_order() {
        let mut l = AutomationLane::default();
        l.set_point(8.0, 1.0);
        l.set_point(2.0, 0.0);
        l.set_point(8.001, 0.5);
        assert_eq!(l.points.len(), 2);
        assert_eq!(l.points[0].beat, 2.0);
        assert_eq!(l.points[1].value, 0.5);
    }

    #[test]
    fn move_point_cannot_cross_neighbours() {
        let mut l = lane(&[(0.0, 0.0), (4.0, 0.5), (8.0, 1.0)]);
        l.move_point(1, 20.0, 0.3);
        assert_eq!(l.points[1].beat, 8.0);
        l.move_point(1, -5.0, 0.3);
        assert_eq!(l.points[1].beat, 0.0);
    }

    #[test]
    fn recording_overwrites_old_points_in_its_path() {
        let mut l = lane(&[(0.0, 0.0), (2.0, 1.0), (3.0, 1.0), (10.0, 0.0)]);
        // A gesture from beat 1 to beat 5 at 0.25.
        l.set_point(1.0, 0.25);
        let mut b = 1.0;
        while b < 5.0 {
            let next = b + 1.0 / 60.0;
            l.record_step(b, next, 0.25);
            b = next;
        }
        // The old 2.0 and 3.0 points are gone; the flat stretch is just two points.
        assert!(l.points.iter().all(|p| !(p.beat > 1.0 && p.beat < 4.9)));
        assert!((l.value_at(3.0).unwrap() - 0.25).abs() < 1e-6);
        // The point after the gesture survives.
        assert_eq!(l.points.last().unwrap().beat, 10.0);
        assert!(l.points.len() <= 4, "{:?}", l.points);
    }

    #[test]
    fn recording_a_ramp_is_thinned() {
        let mut l = AutomationLane::default();
        l.set_point(0.0, 0.0);
        let mut b = 0.0;
        for i in 1..=240 {
            let next = i as f64 / 60.0; // 4 beats at 60 steps per beat
            l.record_step(b, next, next as f32 / 4.0);
            b = next;
        }
        assert!(l.points.len() <= 4 * 32 + 2, "{} points", l.points.len());
        assert!((l.value_at(2.0).unwrap() - 0.5).abs() < 0.02);
    }

    #[test]
    fn param_keys_round_trip() {
        for &p in FxParam::all() {
            assert_eq!(FxParam::from_key(p.key()), Some(p));
            let json = serde_json::to_string(&p).unwrap();
            assert_eq!(json, format!("\"{}\"", p.key()));
        }
    }

    #[test]
    fn track_lane_serializes_flat() {
        let tl = TrackLane {
            param: FxParam::Filter,
            lane: lane(&[(1.0, -0.5)]),
        };
        let json = serde_json::to_string(&tl).unwrap();
        assert_eq!(json, r#"{"param":"filter","points":[{"beat":1.0,"value":-0.5}]}"#);
        let back: TrackLane = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tl);
    }
}
