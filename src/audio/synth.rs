use crate::audio::drums::DrumLibrary;
use crate::audio::samples::{InstrumentSample, SampleLibrary};
use crate::model::drum::DrumKind;
use crate::model::instrument::SynthParams;
use crate::music::theory::midi_to_frequency;
use std::f32::consts::PI;
use std::sync::Arc;

const MAX_VOICES: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnvelopePhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

struct Voice {
    sample: Arc<Vec<f32>>,
    root_freq: f32,
    playback_rate: f32,
    sample_pos: f32,
    amplitude: f32,
    params: SynthParams,
    envelope_phase: EnvelopePhase,
    envelope_level: f32,
    sample_rate: f32,
    filter_state: f32,
}

impl Voice {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample: Arc::new(Vec::new()),
            root_freq: 440.0,
            playback_rate: 1.0,
            sample_pos: 0.0,
            amplitude: 0.0,
            params: SynthParams::default(),
            envelope_phase: EnvelopePhase::Idle,
            envelope_level: 0.0,
            sample_rate,
            filter_state: 0.0,
        }
    }

    fn trigger(
        &mut self,
        pitch: u8,
        velocity: f32,
        params: SynthParams,
        instrument_sample: &InstrumentSample,
    ) {
        let target_freq = midi_to_frequency(pitch);
        self.sample = instrument_sample.data.clone();
        self.root_freq = instrument_sample.root_freq;
        self.playback_rate = target_freq / self.root_freq * (1.0 + params.detune);
        self.sample_pos = 0.0;
        self.amplitude = velocity.clamp(0.0, 1.0);
        self.params = params;
        self.envelope_phase = EnvelopePhase::Attack;
        self.envelope_level = 0.0;
        self.filter_state = 0.0;
    }

    fn release(&mut self) {
        if self.envelope_phase != EnvelopePhase::Idle {
            self.envelope_phase = EnvelopePhase::Release;
        }
    }

    fn is_active(&self) -> bool {
        self.envelope_phase != EnvelopePhase::Idle
    }

    fn sample_at(&self, pos: f32) -> f32 {
        if self.sample.is_empty() {
            return 0.0;
        }
        let idx = pos as usize;
        if idx >= self.sample.len() {
            return 0.0;
        }
        let frac = pos - idx as f32;
        let s0 = self.sample[idx];
        let s1 = self.sample.get(idx + 1).copied().unwrap_or(0.0);
        s0 + (s1 - s0) * frac
    }

    fn next_sample(&mut self) -> f32 {
        if self.envelope_phase == EnvelopePhase::Idle {
            return 0.0;
        }

        let dt = 1.0 / self.sample_rate;
        self.envelope_level = match self.envelope_phase {
            EnvelopePhase::Attack => {
                self.envelope_level + dt / self.params.attack.max(0.001)
            }
            EnvelopePhase::Decay => {
                self.envelope_level - dt / self.params.decay.max(0.001) * (1.0 - self.params.sustain)
            }
            EnvelopePhase::Sustain => self.params.sustain,
            EnvelopePhase::Release => {
                self.envelope_level - dt / self.params.release.max(0.001)
            }
            EnvelopePhase::Idle => 0.0,
        };

        if self.envelope_phase == EnvelopePhase::Attack && self.envelope_level >= 1.0 {
            self.envelope_level = 1.0;
            self.envelope_phase = EnvelopePhase::Decay;
        }
        if self.envelope_phase == EnvelopePhase::Decay && self.envelope_level <= self.params.sustain {
            self.envelope_level = self.params.sustain;
            self.envelope_phase = EnvelopePhase::Sustain;
        }
        if self.envelope_phase == EnvelopePhase::Release && self.envelope_level <= 0.0 {
            self.envelope_phase = EnvelopePhase::Idle;
            return 0.0;
        }

        let raw = self.sample_at(self.sample_pos);
        self.sample_pos += self.playback_rate;

        let cutoff = self.params.filter_cutoff.clamp(0.05, 1.0);
        let alpha = cutoff * 0.35;
        self.filter_state += alpha * (raw - self.filter_state);
        let filtered = raw * (1.0 - cutoff * 0.5) + self.filter_state * (cutoff * 0.5);

        let bright = self.params.brightness.clamp(0.0, 1.0);
        let harmonic =
            (self.sample_pos * self.root_freq / self.sample_rate * 2.0 * PI).sin() * bright * 0.15;
        let mixed = filtered * (1.0 - bright * 0.15) + harmonic;

        if self.sample_pos as usize >= self.sample.len()
            && self.envelope_phase == EnvelopePhase::Sustain
        {
            self.envelope_phase = EnvelopePhase::Release;
        }

        mixed * self.envelope_level * self.amplitude * 0.55
    }
}

const MAX_DRUM_VOICES: usize = 16;

struct DrumVoice {
    sample: Arc<Vec<f32>>,
    sample_pos: f32,
    amplitude: f32,
    envelope: f32,
    sample_rate: f32,
}

impl DrumVoice {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample: Arc::new(Vec::new()),
            sample_pos: 0.0,
            amplitude: 0.0,
            envelope: 0.0,
            sample_rate,
        }
    }

    fn trigger(&mut self, sample: Arc<Vec<f32>>, velocity: f32) {
        self.sample = sample;
        self.sample_pos = 0.0;
        self.amplitude = velocity.clamp(0.05, 1.0);
        self.envelope = 1.0;
    }

    fn is_active(&self) -> bool {
        self.envelope > 0.001 && !self.sample.is_empty()
    }

    fn next_sample(&mut self) -> f32 {
        if !self.is_active() {
            return 0.0;
        }
        let idx = self.sample_pos as usize;
        if idx >= self.sample.len() {
            self.envelope = 0.0;
            return 0.0;
        }
        let frac = self.sample_pos - idx as f32;
        let s0 = self.sample[idx];
        let s1 = self.sample.get(idx + 1).copied().unwrap_or(0.0);
        let out = (s0 + (s1 - s0) * frac) * self.amplitude * self.envelope;
        self.sample_pos += 1.0;
        if self.sample_pos as usize >= self.sample.len() {
            self.envelope *= 0.92;
        }
        out * 0.7
    }
}

pub struct SynthEngine {
    voices: Vec<Voice>,
    drum_voices: Vec<DrumVoice>,
    sample_rate: f32,
    library: SampleLibrary,
    drum_library: DrumLibrary,
}

impl SynthEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..MAX_VOICES).map(|_| Voice::new(sample_rate)).collect(),
            drum_voices: (0..MAX_DRUM_VOICES)
                .map(|_| DrumVoice::new(sample_rate))
                .collect(),
            sample_rate,
            library: SampleLibrary::new(sample_rate),
            drum_library: DrumLibrary::new(sample_rate),
        }
    }

    pub fn drum_hit(&mut self, kind: DrumKind, velocity: f32) {
        let sample = self.drum_library.get(kind).data.clone();
        if let Some(v) = self.drum_voices.iter_mut().find(|v| !v.is_active()) {
            v.trigger(sample, velocity);
        } else if let Some(v) = self.drum_voices.first_mut() {
            v.trigger(sample, velocity);
        }
    }

    pub fn library(&self) -> &SampleLibrary {
        &self.library
    }

    pub fn note_on(&mut self, pitch: u8, velocity: f32, params: SynthParams) {
        let sample = self.library.get(params.sample_kind);
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
            voice.trigger(pitch, velocity, params, sample);
        } else if let Some(voice) = self.voices.first_mut() {
            voice.trigger(pitch, velocity, params, sample);
        }
    }

    pub fn note_off(&mut self, pitch: u8) {
        let freq = midi_to_frequency(pitch);
        for voice in &mut self.voices {
            if voice.is_active() {
                let voice_freq = voice.root_freq * voice.playback_rate;
                if (voice_freq - freq).abs() < 2.0 {
                    voice.release();
                }
            }
        }
    }

    pub fn all_notes_off(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let melodic: f32 = self.voices.iter_mut().map(|v| v.next_sample()).sum();
        let drums: f32 = self.drum_voices.iter_mut().map(|v| v.next_sample()).sum();
        melodic + drums
    }
}
