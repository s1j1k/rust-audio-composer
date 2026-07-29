use crate::model::instrument::{InstrumentId, InstrumentProfile, SampleKind, SynthParams};
use super::{effects::EffectProcessor, synth::SynthEngine};
use crate::model::effects::MasterEffects;
use std::collections::HashMap;

pub struct AudioEngine {
    pub synth: SynthEngine,
    pub master_volume: f32,
    pub master_effects: MasterEffects,
    profiles: HashMap<String, InstrumentProfile>,
    active_params: SynthParams,
    active_instrument: InstrumentId,
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
        let mut names: Vec<_> = self.profiles.keys().cloned().collect();
        names.sort_by(|a, b| {
            let a_custom = a.starts_with("Custom") || !is_builtin_name(a);
            let b_custom = b.starts_with("Custom") || !is_builtin_name(b);
            match (a_custom, b_custom) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });
        names
    }

    pub fn set_profile(&mut self, name: &str) {
        if let Some(profile) = self.profiles.get(name) {
            self.active_params = profile.params.clone();
        }
        self.active_instrument = Self::instrument_id_from_name(name);
    }

    fn instrument_id_from_name(name: &str) -> InstrumentId {
        if name == "Drums" {
            return InstrumentId::Drums;
        }
        SampleKind::all()
            .iter()
            .find(|k| k.label() == name)
            .map(|k| InstrumentId::from_sample_kind(*k))
            .unwrap_or_else(|| InstrumentId::Custom(name.to_string()))
    }

    pub fn play_note_for_instrument(
        &mut self,
        pitch: u8,
        velocity: f32,
        instrument: &InstrumentId,
        track_filter: f32,
    ) {
        if *instrument == InstrumentId::Drums {
            if let Some(kind) = crate::model::drum::DrumKind::from_midi_pitch(pitch) {
                self.synth.drum_hit(kind, velocity);
            }
            return;
        }
        let profile_name = instrument.label();
        let mut params = self
            .profiles
            .get(&profile_name)
            .map(|p| p.params.clone())
            .unwrap_or_else(|| self.active_params.clone());
        params.filter_cutoff = (params.filter_cutoff * track_filter).clamp(0.05, 1.0);
        self.synth.note_on(pitch, velocity, params);
    }

    pub fn play_note(&mut self, pitch: u8, velocity: f32) {
        let inst = self.active_instrument.clone();
        self.play_note_for_instrument(pitch, velocity, &inst, 1.0);
    }

    pub fn stop_note(&mut self, pitch: u8) {
        self.synth.note_off(pitch);
    }

    pub fn set_note_velocity(&mut self, pitch: u8, velocity: f32) {
        self.synth.set_note_velocity(pitch, velocity);
    }

    pub fn stop_all(&mut self) {
        self.synth.all_notes_off();
    }

    pub fn next_sample(&mut self) -> f32 {
        let dry = self.synth.next_sample() * self.master_volume;
        self.fx_processor
            .process_master(dry, &self.master_effects)
    }
}

fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "Piano" | "Guitar" | "Bass" | "Strings" | "Flute" | "Brass" | "Organ" | "Pad" | "Drums"
    )
}
