//! Real-time subtractive / FM synth voices modelled on modern beat-pack staples
//! (808s, Reese basses, garage organs, Rhodes, strings...).
//!
//! Every preset is *expressive*: velocity changes loudness **and** tone (filter, FM
//! "bark", transient noise, drive, attack speed), and continuous pressure (aftertouch from
//! the iPhone remote or a mouse drag) opens the filter, swells the level and deepens vibrato.

use crate::model::instrument::{preset_nominal_params, GenrePreset, SynthParams};
use crate::music::theory::midi_to_frequency;
use std::f32::consts::{PI, TAU};

pub const MAX_GENRE_VOICES: usize = 24;
const CONTROL_RATE: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Wave {
    Sine,
    Saw,
    Square,
    Triangle,
}

#[derive(Clone, Copy, Debug)]
struct Osc {
    wave: Wave,
    level: f32,
    semis: f32,
    unison: u8,
}

const OFF: Osc = Osc {
    wave: Wave::Sine,
    level: 0.0,
    semis: 0.0,
    unison: 1,
};

const fn osc(wave: Wave, level: f32, semis: f32, unison: u8) -> Osc {
    Osc {
        wave,
        level,
        semis,
        unison,
    }
}

#[derive(Clone, Copy, Debug)]
struct Fm {
    ratio: f32,
    index: f32,
    index_decay: f32,
    /// 0 = index ignores velocity, 1 = index fully velocity-scaled.
    vel_index: f32,
    tine_ratio: f32,
    tine_level: f32,
}

#[derive(Clone, Copy, Debug)]
struct Patch {
    osc: [Osc; 2],
    sub: f32,
    noise: f32,
    noise_decay: f32,
    /// When set, osc[0] becomes an FM carrier/modulator pair.
    fm: Option<Fm>,
    pitch_env_semis: f32,
    pitch_env_time: f32,
    cutoff_hz: f32,
    resonance: f32,
    keytrack: f32,
    fenv_oct: f32,
    fenv_decay: f32,
    vel_cutoff_oct: f32,
    vel_attack: f32,
    amp_vel_curve: f32,
    vibrato_rate: f32,
    vibrato_semis: f32,
    vibrato_delay: f32,
    lfo_cutoff_rate: f32,
    lfo_cutoff_oct: f32,
    pressure_cutoff_oct: f32,
    gain: f32,
    mono: bool,
}

const BASE: Patch = Patch {
    osc: [OFF, OFF],
    sub: 0.0,
    noise: 0.0,
    noise_decay: 0.01,
    fm: None,
    pitch_env_semis: 0.0,
    pitch_env_time: 0.05,
    cutoff_hz: 8000.0,
    resonance: 0.0,
    keytrack: 0.0,
    fenv_oct: 0.0,
    fenv_decay: 0.1,
    vel_cutoff_oct: 1.0,
    vel_attack: 0.0,
    amp_vel_curve: 1.2,
    vibrato_rate: 5.0,
    vibrato_semis: 0.0,
    vibrato_delay: 0.0,
    lfo_cutoff_rate: 0.0,
    lfo_cutoff_oct: 0.0,
    pressure_cutoff_oct: 1.2,
    gain: 0.5,
    mono: false,
};

fn patch(preset: GenrePreset) -> Patch {
    use Wave::*;
    match preset {
        GenrePreset::Bass808 => Patch {
            osc: [osc(Sine, 1.0, 0.0, 1), osc(Sine, 0.12, 12.0, 1)],
            pitch_env_semis: 9.0,
            pitch_env_time: 0.045,
            cutoff_hz: 2600.0,
            vel_cutoff_oct: 0.6,
            amp_vel_curve: 0.9,
            pressure_cutoff_oct: 0.8,
            gain: 0.85,
            mono: true,
            ..BASE
        },
        GenrePreset::TrapBell => Patch {
            osc: [osc(Sine, 1.0, 0.0, 1), osc(Sine, 0.12, 12.0, 1)],
            fm: Some(Fm {
                ratio: 3.5,
                index: 2.6,
                index_decay: 0.9,
                vel_index: 0.8,
                tine_ratio: 0.0,
                tine_level: 0.0,
            }),
            cutoff_hz: 9000.0,
            vel_cutoff_oct: 1.0,
            amp_vel_curve: 1.3,
            gain: 0.42,
            ..BASE
        },
        GenrePreset::DarkPluck => Patch {
            osc: [osc(Saw, 0.7, 0.0, 3), osc(Square, 0.25, -12.0, 1)],
            cutoff_hz: 340.0,
            resonance: 0.25,
            keytrack: 0.5,
            fenv_oct: 4.0,
            fenv_decay: 0.16,
            vel_cutoff_oct: 1.5,
            gain: 0.4,
            ..BASE
        },
        GenrePreset::DustyRhodes => Patch {
            osc: [osc(Sine, 1.0, 0.0, 1), OFF],
            fm: Some(Fm {
                ratio: 1.0,
                index: 1.5,
                index_decay: 0.45,
                vel_index: 1.0,
                tine_ratio: 14.0,
                tine_level: 0.06,
            }),
            cutoff_hz: 5200.0,
            keytrack: 0.3,
            vel_cutoff_oct: 1.2,
            vibrato_rate: 0.55,
            vibrato_semis: 0.07,
            amp_vel_curve: 1.5,
            gain: 0.55,
            ..BASE
        },
        GenrePreset::BoomBapBass => Patch {
            osc: [osc(Triangle, 0.8, 0.0, 1), osc(Saw, 0.12, 0.0, 1)],
            sub: 0.6,
            cutoff_hz: 900.0,
            resonance: 0.1,
            fenv_oct: 1.5,
            fenv_decay: 0.1,
            vel_cutoff_oct: 0.8,
            gain: 0.6,
            mono: true,
            ..BASE
        },
        GenrePreset::ReeseBass => Patch {
            osc: [osc(Saw, 0.55, 0.0, 2), osc(Saw, 0.45, 0.0, 2)],
            sub: 0.5,
            cutoff_hz: 650.0,
            resonance: 0.3,
            keytrack: 0.3,
            lfo_cutoff_rate: 0.25,
            lfo_cutoff_oct: 0.6,
            vel_cutoff_oct: 0.8,
            amp_vel_curve: 1.0,
            pressure_cutoff_oct: 2.0,
            gain: 0.4,
            mono: true,
            ..BASE
        },
        GenrePreset::SubBass => Patch {
            osc: [osc(Sine, 1.0, 0.0, 1), osc(Triangle, 0.08, 0.0, 1)],
            cutoff_hz: 320.0,
            vel_cutoff_oct: 0.3,
            amp_vel_curve: 0.8,
            gain: 0.85,
            mono: true,
            ..BASE
        },
        GenrePreset::LiquidPad => Patch {
            osc: [osc(Saw, 0.5, 0.0, 3), osc(Saw, 0.2, 12.0, 2)],
            cutoff_hz: 1600.0,
            resonance: 0.15,
            keytrack: 0.3,
            lfo_cutoff_rate: 0.15,
            lfo_cutoff_oct: 0.4,
            vibrato_rate: 4.5,
            vibrato_semis: 0.05,
            vibrato_delay: 0.5,
            vel_attack: 0.5,
            vel_cutoff_oct: 0.8,
            pressure_cutoff_oct: 2.0,
            gain: 0.28,
            ..BASE
        },
        GenrePreset::GarageOrgan => Patch {
            osc: [osc(Square, 0.5, 0.0, 1), osc(Sine, 0.45, 12.0, 1)],
            sub: 0.25,
            noise: 0.05,
            noise_decay: 0.008,
            cutoff_hz: 3200.0,
            resonance: 0.1,
            keytrack: 0.4,
            fenv_oct: 1.2,
            fenv_decay: 0.08,
            vel_cutoff_oct: 1.0,
            gain: 0.4,
            ..BASE
        },
        GenrePreset::TwoStepStab => Patch {
            osc: [osc(Saw, 0.6, 0.0, 3), osc(Square, 0.25, -12.0, 1)],
            cutoff_hz: 700.0,
            resonance: 0.3,
            keytrack: 0.4,
            fenv_oct: 3.0,
            fenv_decay: 0.2,
            vel_cutoff_oct: 1.2,
            gain: 0.34,
            ..BASE
        },
        GenrePreset::GarageBass => Patch {
            osc: [osc(Square, 0.6, 0.0, 1), osc(Saw, 0.3, 0.0, 1)],
            sub: 0.5,
            cutoff_hz: 260.0,
            resonance: 0.55,
            fenv_oct: 2.8,
            fenv_decay: 0.13,
            vel_cutoff_oct: 1.0,
            amp_vel_curve: 1.0,
            pressure_cutoff_oct: 2.0,
            gain: 0.5,
            mono: true,
            ..BASE
        },
        GenrePreset::ChamberStrings => Patch {
            osc: [osc(Saw, 0.55, 0.0, 3), osc(Saw, 0.12, 12.0, 2)],
            noise: 0.015,
            noise_decay: 0.15,
            cutoff_hz: 2400.0,
            resonance: 0.08,
            keytrack: 0.6,
            vibrato_rate: 5.3,
            vibrato_semis: 0.12,
            vibrato_delay: 0.35,
            vel_attack: 0.8,
            vel_cutoff_oct: 1.2,
            pressure_cutoff_oct: 1.6,
            gain: 0.28,
            ..BASE
        },
        GenrePreset::Pizzicato => Patch {
            osc: [osc(Triangle, 0.6, 0.0, 1), osc(Saw, 0.3, 0.0, 1)],
            cutoff_hz: 600.0,
            resonance: 0.1,
            keytrack: 0.7,
            fenv_oct: 3.0,
            fenv_decay: 0.06,
            vel_cutoff_oct: 1.2,
            amp_vel_curve: 1.3,
            gain: 0.5,
            ..BASE
        },
    }
}

/// Designer-facing controls, derived from `SynthParams` relative to the preset's nominal values.
#[derive(Clone, Copy, Debug)]
pub struct Macros {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub cutoff_oct: f32,
    pub fenv_mul: f32,
    pub detune_cents: f32,
    pub drive: f32,
}

impl Macros {
    pub fn for_preset(preset: GenrePreset, params: Option<&SynthParams>, track_filter: f32) -> Self {
        let nominal = preset_nominal_params(preset);
        let p = params.unwrap_or(&nominal);
        let track_oct = track_filter.clamp(0.05, 1.0).log2() * 1.5;
        Self {
            attack: p.attack.max(0.0005),
            decay: p.decay.max(0.01),
            sustain: p.sustain.clamp(0.0, 1.0),
            release: p.release.max(0.01),
            cutoff_oct: (p.filter_cutoff - nominal.filter_cutoff) * 5.0 + track_oct,
            fenv_mul: (0.5 + p.brightness).clamp(0.2, 1.8),
            detune_cents: p.detune * 1000.0,
            drive: p.drive.clamp(0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Release,
}

#[derive(Clone, Copy, Debug)]
struct Env {
    stage: Stage,
    level: f32,
    attack: f32,
    decay_coef: f32,
    sustain: f32,
    release_coef: f32,
}

impl Env {
    fn idle() -> Self {
        Self {
            stage: Stage::Idle,
            level: 0.0,
            attack: 0.01,
            decay_coef: 0.999,
            sustain: 1.0,
            release_coef: 0.999,
        }
    }

    fn coef(seconds: f32, sr: f32) -> f32 {
        // Time constant chosen so the stage is ~95 % complete after `seconds`.
        (-3.0 / (seconds.max(0.001) * sr)).exp()
    }

    fn trigger(&mut self, attack: f32, decay: f32, sustain: f32, release: f32, sr: f32) {
        self.stage = Stage::Attack;
        self.attack = attack.max(0.0005);
        self.decay_coef = Self::coef(decay, sr);
        self.sustain = sustain;
        self.release_coef = Self::coef(release, sr);
    }

    fn release(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    fn quick_release(&mut self, sr: f32) {
        self.release_coef = Self::coef(0.015, sr);
        self.release();
    }

    fn next(&mut self, dt: f32) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += dt / self.attack;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = self.sustain + (self.level - self.sustain) * self.decay_coef;
            }
            Stage::Release => {
                self.level *= self.release_coef;
                if self.level < 1.0e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[inline]
fn wave_sample(wave: Wave, phase: f32, dt: f32) -> f32 {
    match wave {
        Wave::Sine => (phase * TAU).sin(),
        Wave::Saw => 2.0 * phase - 1.0 - poly_blep(phase, dt),
        Wave::Square => {
            let mut s = if phase < 0.5 { 1.0 } else { -1.0 };
            s += poly_blep(phase, dt);
            s -= poly_blep((phase + 0.5) % 1.0, dt);
            s
        }
        Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
    }
}

/// Cheap tanh approximation (accurate to ~1 % in the useful range).
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x > 3.0 {
        1.0
    } else if x < -3.0 {
        -1.0
    } else {
        x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
    }
}

const MAX_PHASES: usize = 6;

pub struct GenreVoice {
    pub preset: Option<GenrePreset>,
    pub pitch: u8,
    pub key_down: bool,
    pub pedal_hold: bool,
    age: u64,
    sr: f32,
    patch: Patch,
    macros: Macros,
    base_freq: f32,
    vel: f32,
    amp_scale: f32,
    fm_index_scale: f32,
    drive_eff: f32,
    vel_cutoff_oct: f32,
    env: Env,
    t: f32,
    phases: [f32; MAX_PHASES],
    incs: [f32; MAX_PHASES],
    sub_phase: f32,
    mod_phase: f32,
    tine_phase: f32,
    carrier_inc: f32,
    lfo_phase: f32,
    vib_phase: f32,
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
    ctrl: u32,
    pressure_target: f32,
    pressure: f32,
    noise_state: u32,
}

impl GenreVoice {
    pub fn new(sr: f32) -> Self {
        Self {
            preset: None,
            pitch: 0,
            key_down: false,
            pedal_hold: false,
            age: 0,
            sr,
            patch: BASE,
            macros: Macros::for_preset(GenrePreset::SubBass, None, 1.0),
            base_freq: 440.0,
            vel: 0.0,
            amp_scale: 0.0,
            fm_index_scale: 1.0,
            drive_eff: 0.0,
            vel_cutoff_oct: 0.0,
            env: Env::idle(),
            t: 0.0,
            phases: [0.0; MAX_PHASES],
            incs: [0.0; MAX_PHASES],
            sub_phase: 0.0,
            mod_phase: 0.0,
            tine_phase: 0.0,
            carrier_inc: 0.0,
            lfo_phase: 0.0,
            vib_phase: 0.0,
            ic1: 0.0,
            ic2: 0.0,
            a1: 1.0,
            a2: 0.0,
            a3: 0.0,
            k: 2.0,
            ctrl: 0,
            pressure_target: 0.0,
            pressure: 0.0,
            noise_state: 0x1234_5678,
        }
    }

    pub fn is_active(&self) -> bool {
        self.env.stage != Stage::Idle
    }

    pub fn level(&self) -> f32 {
        self.env.level
    }

    pub fn age(&self) -> u64 {
        self.age
    }

    pub fn trigger(
        &mut self,
        preset: GenrePreset,
        pitch: u8,
        velocity: f32,
        macros: Macros,
        age: u64,
    ) {
        let vel = velocity.clamp(0.0, 1.0);
        let p = patch(preset);
        self.preset = Some(preset);
        self.pitch = pitch;
        self.key_down = true;
        self.pedal_hold = false;
        self.age = age;
        self.patch = p;
        self.macros = macros;
        self.base_freq = midi_to_frequency(pitch);
        self.vel = vel;
        self.amp_scale = vel.max(0.02).powf(p.amp_vel_curve);
        self.fm_index_scale = (1.0 - p.fm.map(|f| f.vel_index).unwrap_or(0.0))
            + p.fm.map(|f| f.vel_index).unwrap_or(0.0) * vel * 1.4;
        self.drive_eff = macros.drive * (0.6 + 0.8 * vel);
        self.vel_cutoff_oct = p.vel_cutoff_oct * (vel - 0.6);
        let attack = macros.attack * (1.0 - p.vel_attack * vel * 0.7);
        self.env.level = 0.0;
        self.env
            .trigger(attack, macros.decay, macros.sustain, macros.release, self.sr);
        self.t = 0.0;
        // Random-ish start phases for unison voices so stacks don't phase-cancel on attack.
        for (i, ph) in self.phases.iter_mut().enumerate() {
            *ph = ((age as u32).wrapping_mul(2654435761).wrapping_add(i as u32 * 40503) % 1000)
                as f32
                / 1000.0;
        }
        // Sines/FM start at zero phase to keep the click-free punch of 808s and bells.
        if p.osc[0].wave == Wave::Sine {
            self.phases[0] = 0.0;
        }
        self.sub_phase = 0.0;
        self.mod_phase = 0.0;
        self.tine_phase = 0.0;
        self.vib_phase = 0.0;
        self.ic1 = 0.0;
        self.ic2 = 0.0;
        self.ctrl = 0;
        self.pressure = 0.0;
        self.pressure_target = 0.0;
        self.noise_state ^= (pitch as u32) << 16 | age as u32;
        self.update_control();
    }

    pub fn release(&mut self) {
        self.key_down = false;
        self.pedal_hold = false;
        self.env.release();
    }

    pub fn choke(&mut self) {
        self.key_down = false;
        self.pedal_hold = false;
        self.env.quick_release(self.sr);
    }

    pub fn set_pressure(&mut self, p: f32) {
        self.pressure_target = p.clamp(0.0, 1.0);
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        let vel = velocity.clamp(0.0, 1.0);
        self.vel = vel;
        self.amp_scale = vel.max(0.02).powf(self.patch.amp_vel_curve);
    }

    fn unison_offsets(n: u8) -> &'static [f32] {
        match n {
            0 | 1 => &[0.0],
            2 => &[-0.5, 0.5],
            _ => &[-1.0, 0.0, 1.0],
        }
    }

    fn update_control(&mut self) {
        let p = &self.patch;
        let ctrl_dt = CONTROL_RATE as f32 / self.sr;

        // Pitch modulation (semitones): pitch envelope + delayed vibrato (+ pressure depth).
        let pitch_env = p.pitch_env_semis * (-self.t / p.pitch_env_time.max(0.001)).exp();
        let vib_fade = if p.vibrato_delay > 0.0 {
            ((self.t - p.vibrato_delay) / 0.4).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.vib_phase = (self.vib_phase + p.vibrato_rate * ctrl_dt) % 1.0;
        let vib_depth = p.vibrato_semis * vib_fade + p.vibrato_semis.max(0.04) * self.pressure;
        let vibrato = (self.vib_phase * TAU).sin() * vib_depth;
        let common = pitch_env + vibrato;

        let nyquist_dt = |f: f32| (f / self.sr).min(0.49);
        let detune = self.macros.detune_cents / 100.0; // in semitones
        let mut idx = 0;
        for (o, spec) in p.osc.iter().enumerate() {
            let offsets = Self::unison_offsets(spec.unison);
            for &u in offsets {
                if idx >= MAX_PHASES {
                    break;
                }
                // Single-voice oscillators still get a little spread between osc A and osc B.
                let spread = if spec.unison > 1 {
                    u * detune
                } else if o == 0 {
                    -0.5 * detune
                } else {
                    0.5 * detune
                };
                let semis = spec.semis + common + spread;
                self.incs[idx] = nyquist_dt(self.base_freq * (semis / 12.0).exp2());
                idx += 1;
            }
        }
        self.carrier_inc = nyquist_dt(self.base_freq * (common / 12.0).exp2());

        // Filter cutoff in octaves around the patch cutoff.
        self.lfo_phase = (self.lfo_phase + p.lfo_cutoff_rate * ctrl_dt) % 1.0;
        let key_oct = p.keytrack * ((self.base_freq / 261.63).log2());
        let fenv = p.fenv_oct * self.macros.fenv_mul * (-self.t / p.fenv_decay.max(0.001)).exp();
        let lfo = p.lfo_cutoff_oct * (self.lfo_phase * TAU).sin();
        let pressure_oct = p.pressure_cutoff_oct * self.pressure;
        let oct = key_oct + fenv + lfo + pressure_oct + self.vel_cutoff_oct + self.macros.cutoff_oct;
        let fc = (p.cutoff_hz * oct.exp2()).clamp(30.0, self.sr * 0.45);
        let g = (PI * fc / self.sr).tan();
        self.k = 2.0 - 2.0 * p.resonance.clamp(0.0, 0.95);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    #[inline]
    fn noise(&mut self) -> f32 {
        let mut x = self.noise_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.noise_state = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    pub fn next_sample(&mut self) -> f32 {
        if self.env.stage == Stage::Idle {
            return 0.0;
        }
        let dt = 1.0 / self.sr;
        if self.ctrl == 0 {
            self.pressure += (self.pressure_target - self.pressure) * 0.08;
            self.update_control();
        }
        self.ctrl = (self.ctrl + 1) % CONTROL_RATE;

        let p = self.patch;
        let mut s = 0.0;
        let mut idx = 0;
        for (o, spec) in p.osc.iter().enumerate() {
            let n = Self::unison_offsets(spec.unison).len();
            let per = if n > 1 { 1.0 / (n as f32).sqrt() } else { 1.0 };
            for _ in 0..n {
                if idx >= MAX_PHASES {
                    break;
                }
                let inc = self.incs[idx];
                let ph = self.phases[idx];
                if spec.level > 0.0 {
                    let v = if o == 0 && p.fm.is_some() {
                        let fm = p.fm.unwrap();
                        let floor = 0.25;
                        let idx_env = fm.index
                            * self.fm_index_scale
                            * self.macros.fenv_mul
                            * (floor + (1.0 - floor) * (-self.t / fm.index_decay).exp());
                        let m = (self.mod_phase * TAU).sin();
                        self.mod_phase = (self.mod_phase + self.carrier_inc * fm.ratio) % 1.0;
                        let mut c = (ph * TAU + idx_env * m).sin();
                        if fm.tine_level > 0.0 {
                            c += fm.tine_level
                                * self.vel
                                * (self.tine_phase * TAU).sin()
                                * (-self.t / 0.12).exp();
                            self.tine_phase =
                                (self.tine_phase + (self.carrier_inc * fm.tine_ratio).min(0.49)) % 1.0;
                        }
                        c
                    } else {
                        wave_sample(spec.wave, ph, inc)
                    };
                    s += v * spec.level * per;
                }
                self.phases[idx] = (ph + inc) % 1.0;
                idx += 1;
            }
        }
        if p.sub > 0.0 {
            s += (self.sub_phase * TAU).sin() * p.sub;
            self.sub_phase = (self.sub_phase + self.carrier_inc * 0.5) % 1.0;
        }
        if p.noise > 0.0 {
            let n = self.noise();
            s += n * p.noise * (0.4 + self.vel) * (-self.t / p.noise_decay).exp();
        }

        // TPT state-variable low-pass.
        let v3 = s - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        let mut out = v2;

        if self.drive_eff > 0.001 {
            let g = 1.0 + self.drive_eff * 6.0;
            out = soft_clip(out * g) / g * (1.0 + self.drive_eff);
        }

        let env = self.env.next(dt);
        self.t += dt;
        let swell = 1.0 + 0.4 * self.pressure;
        out * env * self.amp_scale * p.gain * swell
    }
}

/// Is this preset monophonic (bass-style: a new note cuts the previous one)?
pub fn is_mono(preset: GenrePreset) -> bool {
    patch(preset).mono
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(preset: GenrePreset, vel: f32, secs: f32) -> Vec<f32> {
        let sr = 48_000.0;
        let mut v = GenreVoice::new(sr);
        v.trigger(preset, 48, vel, Macros::for_preset(preset, None, 1.0), 1);
        (0..(sr * secs) as usize).map(|_| v.next_sample()).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn every_preset_makes_finite_sound() {
        for &p in GenrePreset::all() {
            let out = render(p, 0.8, 0.6);
            assert!(out.iter().all(|s| s.is_finite()), "{:?} produced NaN", p);
            assert!(rms(&out) > 0.005, "{:?} is silent", p);
            assert!(out.iter().all(|s| s.abs() < 2.0), "{:?} clips hard", p);
        }
    }

    #[test]
    fn harder_hits_are_louder() {
        for &p in GenrePreset::all() {
            let soft = rms(&render(p, 0.25, 0.4));
            let hard = rms(&render(p, 1.0, 0.4));
            assert!(hard > soft * 1.2, "{:?}: soft {soft} hard {hard}", p);
        }
    }

    #[test]
    fn release_reaches_silence() {
        let sr = 48_000.0;
        let mut v = GenreVoice::new(sr);
        let p = GenrePreset::ChamberStrings;
        v.trigger(p, 60, 0.7, Macros::for_preset(p, None, 1.0), 1);
        for _ in 0..(sr * 0.5) as usize {
            v.next_sample();
        }
        v.release();
        for _ in 0..(sr * 3.0) as usize {
            v.next_sample();
        }
        assert!(!v.is_active());
    }
}
