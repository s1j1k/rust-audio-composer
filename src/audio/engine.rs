use crate::model::instrument::{InstrumentProfile, SynthParams};
use super::synth::SynthEngine;
use std::collections::HashMap;

pub struct AudioEngine {
    pub synth: SynthEngine,
    pub master_volume: f32,
    profiles: HashMap<String, InstrumentProfile>,
    active_params: SynthParams,
    sample_rate: f32,
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
            profiles,
            active_params,
            sample_rate,
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
    }

    pub fn active_params(&self) -> &SynthParams {
        &self.active_params
    }

    pub fn play_note(&mut self, pitch: u8, velocity: f32) {
        self.synth
            .note_on(pitch, velocity, self.active_params.clone());
    }

    pub fn stop_note(&mut self, pitch: u8) {
        self.synth.note_off(pitch);
    }

    pub fn stop_all(&mut self) {
        self.synth.all_notes_off();
    }

    pub fn next_sample(&mut self) -> f32 {
        self.synth.next_sample() * self.master_volume
    }
}

fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "Piano" | "Guitar" | "Bass" | "Strings" | "Flute" | "Brass" | "Organ" | "Pad"
    )
}
