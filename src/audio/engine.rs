use super::genre_synth::Macros;
use super::{effects::EffectProcessor, synth::SynthEngine};
use crate::model::effects::MasterEffects;
use crate::model::instrument::{InstrumentId, InstrumentProfile, SynthParams};
use std::collections::HashMap;

/// How raw touch/click strength maps to note velocity for live playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VelocityCurve {
    /// Easy to play loud — good for light fingers on glass.
    Soft,
    #[default]
    Natural,
    /// Needs a firm touch to get loud — more dynamic range.
    Hard,
    /// Every note at the same level (classic organ / step input).
    Fixed,
}

impl VelocityCurve {
    pub fn all() -> &'static [VelocityCurve] {
        &[Self::Soft, Self::Natural, Self::Hard, Self::Fixed]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Soft => "Soft (light touch)",
            Self::Natural => "Natural",
            Self::Hard => "Hard (wide dynamics)",
            Self::Fixed => "Fixed",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Natural => "natural",
            Self::Hard => "hard",
            Self::Fixed => "fixed",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.key() == key)
    }

    pub fn apply(&self, raw: f32) -> f32 {
        let v = raw.clamp(0.0, 1.0);
        let out = match self {
            Self::Soft => v.powf(0.6),
            Self::Natural => v,
            Self::Hard => v.powf(1.7),
            Self::Fixed => 0.8,
        };
        out.clamp(0.03, 1.0)
    }
}

pub struct AudioEngine {
    pub synth: SynthEngine,
    pub master_volume: f32,
    pub master_effects: MasterEffects,
    profiles: HashMap<String, InstrumentProfile>,
    active_params: SynthParams,
    active_instrument: InstrumentId,
    /// Instrument + track filter used for *live* input (remote devices, MIDI), kept in sync
    /// with the selected track by the UI every frame.
    pub live_instrument: InstrumentId,
    pub live_filter: f32,
    pub velocity_curve: VelocityCurve,
    sample_rate: f32,
    fx_processor: EffectProcessor,
}

impl AudioEngine {
    pub fn new(sample_rate: f32) -> Self {
        let mut profiles = HashMap::new();
        for profile in crate::model::instrument::default_profiles() {
            profiles.insert(profile.name.clone(), profile);
        }
        let active_params = profiles["Piano"].params.clone();
        Self {
            synth: SynthEngine::new(sample_rate),
            master_volume: 0.8,
            master_effects: MasterEffects::default(),
            profiles,
            active_params,
            active_instrument: InstrumentId::Piano,
            live_instrument: InstrumentId::Piano,
            live_filter: 1.0,
            velocity_curve: VelocityCurve::Natural,
            sample_rate,
            fx_processor: EffectProcessor::new(sample_rate),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn add_profile(&mut self, profile: InstrumentProfile) {
        let name = profile.name.clone();
        self.profiles.insert(name, profile);
    }

    pub fn profile_names(&self) -> Vec<String> {
        // Names starting with "__" are internal (e.g. the designer's preview slot).
        let mut names: Vec<_> = self
            .profiles
            .keys()
            .filter(|n| !n.starts_with("__"))
            .cloned()
            .collect();
        names.sort_by(|a, b| {
            let a_custom = !is_builtin_name(a);
            let b_custom = !is_builtin_name(b);
            match (a_custom, b_custom) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });
        names
    }

    /// Names of user-made (AI designer) instruments only.
    pub fn custom_profile_names(&self) -> Vec<String> {
        self.profile_names()
            .into_iter()
            .filter(|n| !is_builtin_name(n))
            .collect()
    }

    pub fn set_profile(&mut self, name: &str) {
        if let Some(profile) = self.profiles.get(name) {
            self.active_params = profile.params.clone();
        }
        self.active_instrument = InstrumentId::from_label(name);
    }

    pub fn play_note_for_instrument(
        &mut self,
        pitch: u8,
        velocity: f32,
        instrument: &InstrumentId,
        track_filter: f32,
    ) {
        match instrument {
            InstrumentId::Drums => {
                if let Some(kind) = crate::model::drum::DrumKind::from_midi_pitch(pitch) {
                    self.synth.drum_hit(kind, velocity);
                }
            }
            InstrumentId::Kit(kit) => {
                if let Some(kind) = drum_for_pitch(pitch) {
                    self.synth.kit_hit(*kit, kind, velocity);
                }
            }
            InstrumentId::Preset(preset) => {
                let macros = Macros::for_preset(*preset, None, track_filter);
                self.synth.preset_note_on(*preset, pitch, velocity, macros);
            }
            InstrumentId::Sampled(name) => {
                let mut params = crate::library::synth_params(name);
                params.filter_cutoff = (params.filter_cutoff * track_filter).clamp(0.05, 1.0);
                self.synth.note_on(pitch, velocity, params);
            }
            other => {
                let profile_name = other.label();
                let mut params = self
                    .profiles
                    .get(&profile_name)
                    .map(|p| p.params.clone())
                    .unwrap_or_else(|| self.active_params.clone());
                if let Some(preset) = params.preset {
                    let macros = Macros::for_preset(preset, Some(&params), track_filter);
                    self.synth.preset_note_on(preset, pitch, velocity, macros);
                } else {
                    params.filter_cutoff = (params.filter_cutoff * track_filter).clamp(0.05, 1.0);
                    self.synth.note_on(pitch, velocity, params);
                }
            }
        }
    }

    pub fn play_note(&mut self, pitch: u8, velocity: f32) {
        let inst = self.active_instrument.clone();
        self.play_note_for_instrument(pitch, velocity, &inst, 1.0);
    }

    /// Live note from a remote device: applies the velocity curve and plays on the
    /// selected track's instrument immediately (no UI frame in between).
    /// Returns the velocity actually used.
    pub fn live_note_on(&mut self, pitch: u8, raw_velocity: f32) -> f32 {
        let vel = self.velocity_curve.apply(raw_velocity);
        let inst = self.live_instrument.clone();
        let filter = self.live_filter;
        self.play_note_for_instrument(pitch, vel, &inst, filter);
        vel
    }

    /// Preview a sample in the instrument library.
    pub fn audition(&mut self, sample: std::sync::Arc<Vec<f32>>) {
        self.synth.audition(sample);
    }

    pub fn stop_note(&mut self, pitch: u8) {
        self.synth.note_off(pitch);
    }

    pub fn set_note_velocity(&mut self, pitch: u8, velocity: f32) {
        self.synth.set_note_velocity(pitch, velocity);
    }

    pub fn set_note_pressure(&mut self, pitch: u8, pressure: f32) {
        self.synth.set_note_pressure(pitch, pressure);
    }

    pub fn set_sustain(&mut self, down: bool) {
        self.synth.set_sustain(down);
    }

    pub fn stop_all(&mut self) {
        self.synth.all_notes_off();
    }

    pub fn next_sample(&mut self) -> f32 {
        let dry = self.synth.next_sample() * self.master_volume;
        self.fx_processor.process_master(dry, &self.master_effects)
    }
}

/// Drum kits respond to the GM drum map, and also to a simple "one drum per white/black key"
/// layout in every octave, so they are playable from any octave on a phone or the piano.
pub fn drum_for_pitch(pitch: u8) -> Option<crate::model::drum::DrumKind> {
    use crate::model::drum::DrumKind;
    if let Some(k) = DrumKind::from_midi_pitch(pitch) {
        return Some(k);
    }
    Some(match pitch % 12 {
        0 => DrumKind::Kick,
        1 => DrumKind::Shaker,
        2 => DrumKind::Snare,
        3 => DrumKind::Clap,
        4 => DrumKind::Snare,
        5 => DrumKind::LowTom,
        6 => DrumKind::ClosedHat,
        7 => DrumKind::MidTom,
        8 => DrumKind::ClosedHat,
        9 => DrumKind::HighTom,
        10 => DrumKind::OpenHat,
        _ => DrumKind::Cymbal,
    })
}

fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "Piano"
            | "Guitar"
            | "Bass"
            | "Strings"
            | "Flute"
            | "Brass"
            | "Organ"
            | "Pad"
            | "Drums"
            | "Concert Harp"
    ) || crate::model::instrument::GenrePreset::from_label(name).is_some()
        || crate::model::instrument::DrumKit::from_label(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_curves_are_monotonic_and_bounded() {
        for &c in VelocityCurve::all() {
            let mut prev = 0.0;
            for i in 0..=20 {
                let v = c.apply(i as f32 / 20.0);
                assert!((0.03..=1.0).contains(&v));
                assert!(v + 1e-6 >= prev, "{:?} not monotonic", c);
                prev = v;
            }
        }
        assert!(VelocityCurve::Soft.apply(0.5) > VelocityCurve::Natural.apply(0.5));
        assert!(VelocityCurve::Hard.apply(0.5) < VelocityCurve::Natural.apply(0.5));
    }

    /// A key press on the default instrument must reach the speakers: this is the path the
    /// virtual piano uses (`play_note_for_instrument` then `next_sample` in the audio callback).
    #[test]
    fn piano_key_press_is_audible() {
        let mut eng = AudioEngine::new(44_100.0);
        eng.play_note_for_instrument(60, 0.8, &InstrumentId::Piano, 1.0);
        let energy: f32 = (0..4_410).map(|_| eng.next_sample().abs()).sum();
        assert!(energy > 1.0, "piano note was silent (energy {energy})");
    }

    #[test]
    fn drum_map_covers_every_key() {
        for p in 0..=127u8 {
            assert!(drum_for_pitch(p).is_some());
        }
    }
}
