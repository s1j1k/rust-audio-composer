use crate::model::instrument::{InstrumentPreset, SynthParams};
use crate::music::theory::midi_to_frequency;
use super::synth::SynthEngine;
use std::collections::HashMap;

pub struct AudioEngine {
    pub synth: SynthEngine,
    pub master_volume: f32,
    presets: HashMap<String, InstrumentPreset>,
    active_preset: SynthParams,
}

impl AudioEngine {
    pub fn new(sample_rate: f32) -> Self {
        let mut presets = HashMap::new();
        for preset in crate::model::instrument::default_presets() {
            presets.insert(preset.label(), preset);
        }
        Self {
            synth: SynthEngine::new(sample_rate),
            master_volume: 0.8,
            presets,
            active_preset: InstrumentPreset::piano().params().clone(),
        }
    }

    pub fn add_custom_instrument(&mut self, preset: InstrumentPreset) {
        let label = preset.label();
        self.presets.insert(label, preset);
    }

    pub fn preset_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.presets.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn set_preset(&mut self, name: &str) {
        if let Some(preset) = self.presets.get(name) {
            self.active_preset = preset.params().clone();
        }
    }

    pub fn preset_params(&self, name: &str) -> Option<SynthParams> {
        self.presets.get(name).map(|p| p.params().clone())
    }

    pub fn play_note(&mut self, pitch: u8, velocity: f32) {
        self.synth.note_on(pitch, velocity, self.active_preset.clone());
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

    pub fn frequency_for_pitch(pitch: u8) -> f32 {
        midi_to_frequency(pitch)
    }
}
