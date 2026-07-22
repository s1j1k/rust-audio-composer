use crate::model::instrument::{SynthParams, Waveform};
use std::f32::consts::PI;

const MAX_VOICES: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnvelopePhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct Voice {
    frequency: f32,
    phase: f32,
    phase2: f32,
    amplitude: f32,
    params: SynthParams,
    envelope_phase: EnvelopePhase,
    envelope_level: f32,
    sample_rate: f32,
    age: f32,
}

impl Voice {
    fn new(sample_rate: f32) -> Self {
        Self {
            frequency: 440.0,
            phase: 0.0,
            phase2: 0.0,
            amplitude: 0.0,
            params: SynthParams::default(),
            envelope_phase: EnvelopePhase::Idle,
            envelope_level: 0.0,
            sample_rate,
            age: 0.0,
        }
    }

    fn trigger(&mut self, frequency: f32, velocity: f32, params: SynthParams) {
        self.frequency = frequency;
        self.amplitude = velocity.clamp(0.0, 1.0);
        self.params = params;
        self.envelope_phase = EnvelopePhase::Attack;
        self.envelope_level = 0.0;
        self.age = 0.0;
    }

    fn release(&mut self) {
        if self.envelope_phase != EnvelopePhase::Idle {
            self.envelope_phase = EnvelopePhase::Release;
        }
    }

    fn is_active(&self) -> bool {
        self.envelope_phase != EnvelopePhase::Idle
    }

    fn waveform_sample(phase: f32, waveform: Waveform) -> f32 {
        match waveform {
            Waveform::Sine => (phase * 2.0 * PI).sin(),
            Waveform::Triangle => {
                let t = phase % 1.0;
                if t < 0.5 { 4.0 * t - 1.0 } else { 3.0 - 4.0 * t }
            }
            Waveform::Square => if phase % 1.0 < 0.5 { 1.0 } else { -1.0 },
            Waveform::Saw => 2.0 * (phase % 1.0) - 1.0,
        }
    }

    fn next_sample(&mut self) -> f32 {
        if self.envelope_phase == EnvelopePhase::Idle {
            return 0.0;
        }

        let dt = 1.0 / self.sample_rate;
        self.age += dt;

        self.envelope_level = match self.envelope_phase {
            EnvelopePhase::Attack => {
                let rate = if self.params.attack > 0.0 {
                    dt / self.params.attack
                } else {
                    1.0
                };
                self.envelope_level + rate
            }
            EnvelopePhase::Decay => {
                let rate = if self.params.decay > 0.0 {
                    dt / self.params.decay
                } else {
                    1.0
                };
                self.envelope_level - rate * (1.0 - self.params.sustain)
            }
            EnvelopePhase::Sustain => self.params.sustain,
            EnvelopePhase::Release => {
                let rate = if self.params.release > 0.0 {
                    dt / self.params.release
                } else {
                    1.0
                };
                self.envelope_level - rate
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
            self.envelope_level = 0.0;
            self.envelope_phase = EnvelopePhase::Idle;
            return 0.0;
        }

        let detune_freq = self.frequency * (1.0 + self.params.detune);
        let s1 = Self::waveform_sample(self.phase, self.params.waveform);
        let s2 = Self::waveform_sample(self.phase2, self.params.waveform);
        let mix = s1 * (1.0 - self.params.brightness * 0.3) + s2 * self.params.brightness * 0.3;

        self.phase += self.frequency / self.sample_rate;
        self.phase2 += detune_freq / self.sample_rate;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        if self.phase2 >= 1.0 { self.phase2 -= 1.0; }

        mix * self.envelope_level * self.amplitude * 0.3
    }
}

pub struct SynthEngine {
    voices: Vec<Voice>,
    sample_rate: f32,
}

impl SynthEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..MAX_VOICES).map(|_| Voice::new(sample_rate)).collect(),
            sample_rate,
        }
    }

    pub fn note_on(&mut self, pitch: u8, velocity: f32, params: SynthParams) {
        let freq = crate::music::theory::midi_to_frequency(pitch);
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
            voice.trigger(freq, velocity, params);
        } else {
            self.voices[0].trigger(freq, velocity, params);
        }
    }

    pub fn note_off(&mut self, pitch: u8) {
        let freq = crate::music::theory::midi_to_frequency(pitch);
        for voice in &mut self.voices {
            if voice.is_active() && (voice.frequency - freq).abs() < 0.5 {
                voice.release();
            }
        }
    }

    pub fn all_notes_off(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        self.voices.iter_mut().map(|v| v.next_sample()).sum()
    }
}
