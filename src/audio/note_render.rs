//! Render one note offline into a buffer: the bridge between "synth settings" and a
//! single-note sample you can see and edit.
//!
//! Any future generator (e.g. a diffusion model) only has to produce the same thing this
//! does, a mono `Vec<f32>` of one note at a known pitch, and the waveform editor and
//! sampler playback work unchanged.

use crate::audio::synth::SynthEngine;
use crate::model::instrument::{NoteSample, SynthParams};

/// Longest note we keep (seconds). Long enough for big 808 tails and pad releases.
pub const MAX_NOTE_SECS: f32 = 10.0;

/// Peak level rendered notes are normalized to (same headroom as the bundled samples).
const TARGET_PEAK: f32 = 0.95;

/// Velocity notes are rendered at: firm, but leaves soft/hard timbre layers sounding natural.
const RENDER_VELOCITY: f32 = 0.85;

/// A sensible key-down time for these settings: sustaining sounds get held for a while;
/// sounds that decay to silence get held until the decay finishes.
pub fn default_hold_secs(params: &SynthParams) -> f32 {
    let hold = if params.sustain < 0.05 {
        params.attack + params.decay
    } else {
        params.attack + params.decay + 1.0
    };
    hold.clamp(0.15, 6.0)
}

/// Render `params` as one note at `root_pitch`, held for `hold_secs`, then let its release
/// tail ring out. Trailing silence is trimmed and the result normalized.
pub fn render_note(params: &SynthParams, root_pitch: u8, hold_secs: f32, sample_rate: f32) -> NoteSample {
    let mut synth = SynthEngine::new(sample_rate);
    synth.note_on(root_pitch, RENDER_VELOCITY, params.clone());

    let hold = (hold_secs.clamp(0.02, MAX_NOTE_SECS) * sample_rate) as usize;
    let max = (MAX_NOTE_SECS * sample_rate) as usize;
    let mut data = Vec::with_capacity(hold * 2);
    for i in 0..max {
        if i == hold {
            synth.note_off(root_pitch);
        }
        data.push(synth.next_sample());
        if i > hold && synth.active_voice_count() == 0 {
            break;
        }
    }

    trim_trailing_silence(&mut data, sample_rate);
    normalize(&mut data, TARGET_PEAK);
    NoteSample {
        root_pitch,
        sample_rate: sample_rate as u32,
        data,
        edited: false,
    }
}

/// Drop the near-silent end of a buffer, keeping a short fade so the cut can't click.
pub fn trim_trailing_silence(data: &mut Vec<f32>, sample_rate: f32) {
    let peak = data.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    let floor = (peak * 0.0005).max(1e-5); // about -66 dB below the peak
    if let Some(last) = data.iter().rposition(|s| s.abs() > floor) {
        data.truncate((last + 1).max(1));
    }
    let fade = ((0.005 * sample_rate) as usize).min(data.len());
    let n = data.len();
    for (k, s) in data[n - fade..].iter_mut().enumerate() {
        *s *= 1.0 - (k + 1) as f32 / fade as f32;
    }
}

pub fn normalize(data: &mut [f32], target_peak: f32) {
    let peak = data.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if peak > 1e-6 {
        let g = target_peak / peak;
        data.iter_mut().for_each(|s| *s *= g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::instrument::{preset_nominal_params, GenrePreset};

    #[test]
    fn renders_a_preset_note_with_audio_in_it() {
        let p = preset_nominal_params(GenrePreset::TrapBell);
        let note = render_note(&p, 72, default_hold_secs(&p), 44100.0);
        assert_eq!(note.root_pitch, 72);
        assert!(note.data.len() > 4410, "too short: {}", note.data.len());
        assert!(note.duration_secs() <= MAX_NOTE_SECS + 0.01);
        let peak = note.data.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!((peak - TARGET_PEAK).abs() < 0.01, "peak {peak}");
        assert!(!note.edited);
    }

    #[test]
    fn trimming_keeps_the_sound_and_ends_at_zero() {
        let sr = 1000.0;
        let mut d: Vec<f32> = (0..500).map(|i| (i as f32 * 0.3).sin()).collect();
        d.extend(std::iter::repeat(0.0).take(500));
        trim_trailing_silence(&mut d, sr);
        assert!(d.len() <= 500 && d.len() > 480);
        assert_eq!(*d.last().unwrap(), 0.0);
    }
}
