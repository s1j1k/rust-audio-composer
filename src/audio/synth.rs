use crate::audio::drum_kits::KitLibrary;
use crate::audio::drums::DrumLibrary;
use crate::audio::genre_synth::{self, soft_clip, GenreVoice, Macros, MAX_GENRE_VOICES};
use crate::audio::multisample;
use crate::audio::samples::{InstrumentSample, SampleLibrary};
use crate::model::drum::DrumKind;
use crate::model::instrument::{DrumKit, GenrePreset, ProfileSample, SynthParams};
use crate::music::theory::midi_to_frequency;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

const MAX_VOICES: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnvelopePhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Perceptual velocity → gain. Multi-layer instruments already get softer *timbre*
/// from the soft layer, so they use a gentler curve.
fn velocity_gain(velocity: f32, multilayer: bool) -> f32 {
    let v = velocity.clamp(0.0, 1.0);
    if multilayer {
        v.powf(0.85)
    } else {
        v.powf(1.3)
    }
}

/// Velocity → low-pass cutoff (Hz) for sample voices: soft hits are rounder/darker.
fn velocity_cutoff_hz(velocity: f32, filter_cutoff: f32, brightness: f32) -> f32 {
    let v = velocity.clamp(0.0, 1.0);
    let base = 900.0 * (2.0_f32).powf(filter_cutoff.clamp(0.05, 1.0) * 4.5); // 1 kHz .. 20 kHz
    let vel_oct = (v - 0.85) * 3.0; // soft notes lose up to ~2.5 octaves of top end
    let bright_oct = (brightness - 0.5) * 1.5;
    (base * (vel_oct + bright_oct).exp2()).clamp(250.0, 20_000.0)
}

struct Voice {
    pitch: u8,
    key_down: bool,
    pedal_hold: bool,
    age: u64,
    sample: Arc<Vec<f32>>,
    sample_hard: Arc<Vec<f32>>,
    hard_mix: f32,
    multilayer: bool,
    root_freq: f32,
    playback_rate: f32,
    sample_pos: f32,
    velocity: f32,
    amplitude: f32,
    params: SynthParams,
    envelope_phase: EnvelopePhase,
    envelope_level: f32,
    sample_rate: f32,
    lp_state: f32,
    lp_coef: f32,
    base_cutoff: f32,
    pressure: f32,
    pressure_target: f32,
}

impl Voice {
    fn new(sample_rate: f32) -> Self {
        Self {
            pitch: 0,
            key_down: false,
            pedal_hold: false,
            age: 0,
            sample: Arc::new(Vec::new()),
            sample_hard: Arc::new(Vec::new()),
            hard_mix: 0.0,
            multilayer: false,
            root_freq: 440.0,
            playback_rate: 1.0,
            sample_pos: 0.0,
            velocity: 0.0,
            amplitude: 0.0,
            params: SynthParams::default(),
            envelope_phase: EnvelopePhase::Idle,
            envelope_level: 0.0,
            sample_rate,
            lp_state: 0.0,
            lp_coef: 1.0,
            base_cutoff: 20_000.0,
            pressure: 0.0,
            pressure_target: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn trigger(
        &mut self,
        pitch: u8,
        velocity: f32,
        params: SynthParams,
        soft: Arc<Vec<f32>>,
        hard: Arc<Vec<f32>>,
        hard_mix: f32,
        root_freq: f32,
        multilayer: bool,
        age: u64,
    ) {
        let target_freq = midi_to_frequency(pitch);
        self.pitch = pitch;
        self.key_down = true;
        self.pedal_hold = false;
        self.age = age;
        self.sample = soft;
        self.sample_hard = hard;
        self.hard_mix = hard_mix;
        self.multilayer = multilayer;
        self.root_freq = root_freq;
        self.playback_rate = target_freq / self.root_freq * (1.0 + params.detune);
        self.sample_pos = 0.0;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.amplitude = velocity_gain(self.velocity, multilayer);
        self.base_cutoff =
            velocity_cutoff_hz(self.velocity, params.filter_cutoff, params.brightness);
        self.params = params;
        self.envelope_phase = EnvelopePhase::Attack;
        self.envelope_level = 0.0;
        self.lp_state = 0.0;
        self.pressure = 0.0;
        self.pressure_target = 0.0;
        self.update_filter();
    }

    fn update_filter(&mut self) {
        let fc = (self.base_cutoff * (self.pressure * 1.5).exp2()).min(self.sample_rate * 0.45);
        self.lp_coef = 1.0 - (-TAU * fc / self.sample_rate).exp();
    }

    fn release(&mut self) {
        self.key_down = false;
        self.pedal_hold = false;
        if self.envelope_phase != EnvelopePhase::Idle {
            self.envelope_phase = EnvelopePhase::Release;
        }
    }

    fn is_active(&self) -> bool {
        self.envelope_phase != EnvelopePhase::Idle
    }

    fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity.clamp(0.0, 1.0);
        self.amplitude = velocity_gain(self.velocity, self.multilayer);
        self.base_cutoff =
            velocity_cutoff_hz(self.velocity, self.params.filter_cutoff, self.params.brightness);
        self.update_filter();
    }

    #[inline]
    fn read(buf: &[f32], pos: f32) -> f32 {
        let idx = pos as usize;
        if idx >= buf.len() {
            return 0.0;
        }
        let frac = pos - idx as f32;
        let s0 = buf[idx];
        let s1 = buf.get(idx + 1).copied().unwrap_or(0.0);
        s0 + (s1 - s0) * frac
    }

    fn sample_at(&self, pos: f32) -> f32 {
        if self.hard_mix <= 0.001 || Arc::ptr_eq(&self.sample, &self.sample_hard) {
            return Self::read(&self.sample, pos);
        }
        if self.hard_mix >= 0.999 {
            return Self::read(&self.sample_hard, pos);
        }
        // Equal-power crossfade between velocity layers.
        let a = (self.hard_mix * PI * 0.5).cos();
        let b = (self.hard_mix * PI * 0.5).sin();
        Self::read(&self.sample, pos) * a + Self::read(&self.sample_hard, pos) * b
    }

    fn sample_len(&self) -> usize {
        self.sample.len().max(self.sample_hard.len())
    }

    fn next_sample(&mut self) -> f32 {
        if self.envelope_phase == EnvelopePhase::Idle {
            return 0.0;
        }

        let dt = 1.0 / self.sample_rate;
        self.envelope_level = match self.envelope_phase {
            EnvelopePhase::Attack => {
                // Harder hits speak a little faster.
                let attack = self.params.attack.max(0.001) * (1.15 - 0.3 * self.velocity);
                self.envelope_level + dt / attack
            }
            EnvelopePhase::Decay => {
                self.envelope_level
                    - dt / self.params.decay.max(0.001) * (1.0 - self.params.sustain)
            }
            EnvelopePhase::Sustain => self.params.sustain,
            EnvelopePhase::Release => self.envelope_level - dt / self.params.release.max(0.001),
            EnvelopePhase::Idle => 0.0,
        };

        if self.envelope_phase == EnvelopePhase::Attack && self.envelope_level >= 1.0 {
            self.envelope_level = 1.0;
            self.envelope_phase = EnvelopePhase::Decay;
        }
        if self.envelope_phase == EnvelopePhase::Decay && self.envelope_level <= self.params.sustain
        {
            self.envelope_level = self.params.sustain;
            self.envelope_phase = EnvelopePhase::Sustain;
        }
        if self.envelope_phase == EnvelopePhase::Release && self.envelope_level <= 0.0 {
            self.envelope_phase = EnvelopePhase::Idle;
            return 0.0;
        }

        if (self.pressure_target - self.pressure).abs() > 0.001 {
            self.pressure += (self.pressure_target - self.pressure) * 0.002;
            self.update_filter();
        }

        let raw = self.sample_at(self.sample_pos);
        self.sample_pos += self.playback_rate;

        self.lp_state += self.lp_coef * (raw - self.lp_state);
        let mut out = self.lp_state;

        if self.params.drive > 0.001 {
            let g = 1.0 + self.params.drive * (0.6 + 0.8 * self.velocity) * 6.0;
            out = soft_clip(out * g) / g * (1.0 + self.params.drive);
        }

        if self.sample_pos as usize >= self.sample_len() {
            self.envelope_phase = EnvelopePhase::Idle;
            return 0.0;
        }

        let swell = 1.0 + 0.3 * self.pressure;
        out * self.envelope_level * self.amplitude * 0.55 * swell
    }
}

const MAX_DRUM_VOICES: usize = 24;

struct DrumVoice {
    sample: Arc<Vec<f32>>,
    kind: Option<DrumKind>,
    sample_pos: usize,
    amplitude: f32,
    envelope: f32,
    choke: bool,
    lp_coef: f32,
    lp_state: f32,
    sample_rate: f32,
}

impl DrumVoice {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample: Arc::new(Vec::new()),
            kind: None,
            sample_pos: 0,
            amplitude: 0.0,
            envelope: 0.0,
            choke: false,
            lp_coef: 1.0,
            lp_state: 0.0,
            sample_rate,
        }
    }

    fn trigger(&mut self, sample: Arc<Vec<f32>>, kind: DrumKind, velocity: f32) {
        let v = velocity.clamp(0.02, 1.0);
        self.sample = sample;
        self.kind = Some(kind);
        self.sample_pos = 0;
        self.amplitude = v.powf(1.4);
        self.envelope = 1.0;
        self.choke = false;
        // Ghost notes are darker; full hits are unfiltered.
        let fc = (2_000.0 * (v * 3.4).exp2()).min(self.sample_rate * 0.45);
        self.lp_coef = 1.0 - (-TAU * fc / self.sample_rate).exp();
        self.lp_state = 0.0;
    }

    fn is_active(&self) -> bool {
        self.envelope > 0.001 && !self.sample.is_empty()
    }

    fn next_sample(&mut self) -> f32 {
        if !self.is_active() {
            return 0.0;
        }
        let Some(&s) = self.sample.get(self.sample_pos) else {
            self.envelope = 0.0;
            return 0.0;
        };
        self.sample_pos += 1;
        if self.choke {
            self.envelope *= 0.995;
        }
        self.lp_state += self.lp_coef * (s - self.lp_state);
        self.lp_state * self.amplitude * self.envelope * 0.7
    }
}

pub struct SynthEngine {
    voices: Vec<Voice>,
    genre_voices: Vec<GenreVoice>,
    drum_voices: Vec<DrumVoice>,
    sample_rate: f32,
    library: Arc<SampleLibrary>,
    drum_library: Arc<DrumLibrary>,
    kits: Arc<KitLibrary>,
    sustain_pedal: bool,
    age: u64,
}

impl SynthEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..MAX_VOICES).map(|_| Voice::new(sample_rate)).collect(),
            genre_voices: (0..MAX_GENRE_VOICES)
                .map(|_| GenreVoice::new(sample_rate))
                .collect(),
            drum_voices: (0..MAX_DRUM_VOICES)
                .map(|_| DrumVoice::new(sample_rate))
                .collect(),
            sample_rate,
            library: SampleLibrary::shared(sample_rate),
            drum_library: DrumLibrary::shared(sample_rate),
            kits: KitLibrary::shared(sample_rate),
            sustain_pedal: false,
            age: 0,
        }
    }

    fn next_age(&mut self) -> u64 {
        self.age += 1;
        self.age
    }

    fn drum_voice_slot(&mut self) -> usize {
        self.drum_voices
            .iter()
            .position(|v| !v.is_active())
            .unwrap_or_else(|| {
                // Steal the quietest voice.
                self.drum_voices
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        (a.1.envelope * a.1.amplitude)
                            .partial_cmp(&(b.1.envelope * b.1.amplitude))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            })
    }

    fn play_drum(&mut self, sample: Arc<Vec<f32>>, kind: DrumKind, velocity: f32) {
        // Closed hat chokes a ringing open hat, like a real hi-hat.
        if kind == DrumKind::ClosedHat {
            for v in &mut self.drum_voices {
                if v.kind == Some(DrumKind::OpenHat) && v.is_active() {
                    v.choke = true;
                }
            }
        }
        let slot = self.drum_voice_slot();
        self.drum_voices[slot].trigger(sample, kind, velocity);
    }

    pub fn drum_hit(&mut self, kind: DrumKind, velocity: f32) {
        let sample = self.drum_library.get(kind).data.clone();
        self.play_drum(sample, kind, velocity);
    }

    pub fn kit_hit(&mut self, kit: DrumKit, kind: DrumKind, velocity: f32) {
        let sample = self.kits.get(kit, kind);
        self.play_drum(sample, kind, velocity);
    }

    pub fn library(&self) -> &SampleLibrary {
        &self.library
    }

    fn sample_voice_slot(&mut self) -> usize {
        self.voices
            .iter()
            .position(|v| !v.is_active())
            .unwrap_or_else(|| {
                // Prefer stealing released voices, then the oldest.
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| (v.key_down || v.pedal_hold, v.age))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            })
    }

    pub fn note_on(&mut self, pitch: u8, velocity: f32, params: SynthParams) {
        if let Some(preset) = params.preset {
            let macros = Macros::for_preset(preset, Some(&params), 1.0);
            self.preset_note_on(preset, pitch, velocity, macros);
            return;
        }
        let age = self.next_age();
        let kind = params.sample_kind;
        let (soft, hard, mix, root, multi) =
            match multisample::zones(kind, self.sample_rate).and_then(|z| z.pick(pitch, velocity))
            {
                Some(z) => (z.soft, z.hard, z.hard_mix, z.root_freq, true),
                None => {
                    let s: &InstrumentSample = self.library.get(kind);
                    (s.data.clone(), s.data.clone(), 0.0, s.root_freq, false)
                }
            };
        self.start_sample_voice(pitch, velocity, params, soft, hard, mix, root, multi, age);
    }

    /// Play a custom instrument's own one-note sample, pitch-shifted to `pitch`.
    ///
    /// The sample already contains its attack/decay and tone, so the voice envelope is
    /// neutral (instant attack, full sustain); only `release` (key-up fade) and the track
    /// filter are applied, plus the usual velocity → level/brightness response.
    pub fn sample_note_on(
        &mut self,
        pitch: u8,
        velocity: f32,
        sample: &ProfileSample,
        release: f32,
        filter_cutoff: f32,
    ) {
        let age = self.next_age();
        // The voice steps through the buffer at target_freq / root_freq per output sample.
        // Scaling the root by engine_rate / sample_rate also corrects for a sample recorded
        // at a different rate (e.g. a 48 kHz WAV on a 44.1 kHz device).
        let root = midi_to_frequency(sample.root_pitch) * self.sample_rate / sample.sample_rate.max(1.0);
        let params = SynthParams {
            attack: 0.0015,
            decay: 0.02,
            sustain: 1.0,
            release: release.clamp(0.02, 6.0),
            brightness: 0.5,
            detune: 0.0,
            filter_cutoff: filter_cutoff.clamp(0.05, 1.0),
            drive: 0.0,
            ..SynthParams::default()
        };
        let data = sample.data.clone();
        self.start_sample_voice(pitch, velocity, params, data.clone(), data, 0.0, root, false, age);
    }

    #[allow(clippy::too_many_arguments)]
    fn start_sample_voice(
        &mut self,
        pitch: u8,
        velocity: f32,
        params: SynthParams,
        soft: Arc<Vec<f32>>,
        hard: Arc<Vec<f32>>,
        mix: f32,
        root: f32,
        multi: bool,
        age: u64,
    ) {
        // Re-striking a held/ringing note: let the old one fade quickly (piano-style).
        for v in &mut self.voices {
            if v.is_active() && v.pitch == pitch {
                v.release();
            }
        }
        let slot = self.sample_voice_slot();
        self.voices[slot].trigger(pitch, velocity, params, soft, hard, mix, root, multi, age);
    }

    pub fn preset_note_on(&mut self, preset: GenrePreset, pitch: u8, velocity: f32, macros: Macros) {
        let age = self.next_age();
        let mono = genre_synth::is_mono(preset);
        for v in &mut self.genre_voices {
            if v.is_active() && v.preset == Some(preset) && (mono || v.pitch == pitch) {
                v.choke();
            }
        }
        let slot = self
            .genre_voices
            .iter()
            .position(|v| !v.is_active())
            .unwrap_or_else(|| {
                self.genre_voices
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        let ka = (a.1.key_down, a.1.age());
                        let kb = (b.1.key_down, b.1.age());
                        ka.cmp(&kb)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });
        self.genre_voices[slot].trigger(preset, pitch, velocity, macros, age);
    }

    pub fn note_off(&mut self, pitch: u8) {
        let pedal = self.sustain_pedal;
        for voice in &mut self.voices {
            if voice.is_active() && voice.pitch == pitch && voice.key_down {
                if pedal {
                    voice.key_down = false;
                    voice.pedal_hold = true;
                } else {
                    voice.release();
                }
            }
        }
        for voice in &mut self.genre_voices {
            if voice.is_active() && voice.pitch == pitch && voice.key_down {
                if pedal {
                    voice.key_down = false;
                    voice.pedal_hold = true;
                } else {
                    voice.release();
                }
            }
        }
    }

    /// Sustain pedal (CC64). Releasing it lets go of every note held only by the pedal.
    pub fn set_sustain(&mut self, down: bool) {
        self.sustain_pedal = down;
        if !down {
            for v in &mut self.voices {
                if v.pedal_hold {
                    v.release();
                }
            }
            for v in &mut self.genre_voices {
                if v.pedal_hold {
                    v.release();
                }
            }
        }
    }

    pub fn sustain(&self) -> bool {
        self.sustain_pedal
    }

    pub fn set_note_velocity(&mut self, pitch: u8, velocity: f32) {
        for voice in &mut self.voices {
            if voice.is_active() && voice.pitch == pitch && voice.key_down {
                voice.set_velocity(velocity);
            }
        }
        for voice in &mut self.genre_voices {
            if voice.is_active() && voice.pitch == pitch && voice.key_down {
                voice.set_velocity(velocity);
            }
        }
    }

    /// Continuous pressure / aftertouch (0..1) on a held note.
    pub fn set_note_pressure(&mut self, pitch: u8, pressure: f32) {
        let p = pressure.clamp(0.0, 1.0);
        for voice in &mut self.voices {
            if voice.is_active() && voice.pitch == pitch && voice.key_down {
                voice.pressure_target = p;
            }
        }
        for voice in &mut self.genre_voices {
            if voice.is_active() && voice.pitch == pitch && voice.key_down {
                voice.set_pressure(p);
            }
        }
    }

    pub fn all_notes_off(&mut self) {
        self.sustain_pedal = false;
        for voice in &mut self.voices {
            voice.release();
        }
        for voice in &mut self.genre_voices {
            voice.release();
        }
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
            + self.genre_voices.iter().filter(|v| v.is_active()).count()
            + self.drum_voices.iter().filter(|v| v.is_active()).count()
    }

    pub fn next_sample(&mut self) -> f32 {
        let melodic: f32 = self.voices.iter_mut().map(|v| v.next_sample()).sum();
        let synth: f32 = self.genre_voices.iter_mut().map(|v| v.next_sample()).sum();
        let drums: f32 = self.drum_voices.iter_mut().map(|v| v.next_sample()).sum();
        melodic + synth + drums
    }
}
