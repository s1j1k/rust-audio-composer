//! Synthesized genre drum kits: Trap 808, Boom Bap, D&B/Jungle and UK Garage.
//!
//! Each hit is rendered once per sample rate into a buffer (cheap: a few hundred ms of
//! audio per sound). Velocity is applied at playback time by `synth::DrumVoice`, which
//! scales level *and* darkens soft hits, so ghost notes sound like ghost notes.

use crate::model::drum::DrumKind;
use crate::model::instrument::DrumKit;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex, OnceLock};

pub struct KitLibrary {
    sounds: HashMap<(DrumKit, DrumKind), Arc<Vec<f32>>>,
}

impl KitLibrary {
    fn render_all(sr: f32) -> Self {
        let mut sounds = HashMap::new();
        for &kit in DrumKit::all() {
            for &kind in DrumKind::all() {
                sounds.insert((kit, kind), Arc::new(render(kit, kind, sr)));
            }
        }
        Self { sounds }
    }

    /// Shared, lazily rendered library for a sample rate.
    pub fn shared(sr: f32) -> Arc<KitLibrary> {
        static CACHE: OnceLock<Mutex<HashMap<u32, Arc<KitLibrary>>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(sr as u32)
            .or_insert_with(|| Arc::new(Self::render_all(sr)))
            .clone()
    }

    pub fn get(&self, kit: DrumKit, kind: DrumKind) -> Arc<Vec<f32>> {
        self.sounds
            .get(&(kit, kind))
            .cloned()
            .unwrap_or_else(|| Arc::new(Vec::new()))
    }
}

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One-pole filters used for shaping noise.
struct OnePole {
    a: f32,
    z: f32,
}

impl OnePole {
    fn new(fc: f32, sr: f32) -> Self {
        Self {
            a: 1.0 - (-TAU * fc / sr).exp(),
            z: 0.0,
        }
    }
    fn lp(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
    fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
}

#[derive(Clone, Copy)]
struct KickSpec {
    f_start: f32,
    f_end: f32,
    pitch_tau: f32,
    amp_tau: f32,
    length: f32,
    click: f32,
    drive: f32,
}

#[derive(Clone, Copy)]
struct SnareSpec {
    tone: f32,
    tone_tau: f32,
    noise_tau: f32,
    noise_level: f32,
    noise_hp: f32,
    noise_lp: f32,
    length: f32,
}

struct Voicing {
    kick: KickSpec,
    snare: SnareSpec,
    hat_tau: f32,
    open_hat_tau: f32,
    hat_scale: f32,
    clap_tail: f32,
    tom_base: f32,
    cymbal_tau: f32,
    lofi: bool,
}

fn voicing(kit: DrumKit) -> Voicing {
    match kit {
        DrumKit::Trap808 => Voicing {
            kick: KickSpec {
                f_start: 160.0,
                f_end: 47.0,
                pitch_tau: 0.03,
                amp_tau: 0.55,
                length: 1.6,
                click: 0.25,
                drive: 0.35,
            },
            snare: SnareSpec {
                tone: 205.0,
                tone_tau: 0.05,
                noise_tau: 0.09,
                noise_level: 0.8,
                noise_hp: 1500.0,
                noise_lp: 9000.0,
                length: 0.3,
            },
            hat_tau: 0.022,
            open_hat_tau: 0.28,
            hat_scale: 1.0,
            clap_tail: 0.16,
            tom_base: 95.0,
            cymbal_tau: 0.9,
            lofi: false,
        },
        DrumKit::BoomBap => Voicing {
            kick: KickSpec {
                f_start: 120.0,
                f_end: 56.0,
                pitch_tau: 0.025,
                amp_tau: 0.16,
                length: 0.45,
                click: 0.6,
                drive: 0.6,
            },
            snare: SnareSpec {
                tone: 180.0,
                tone_tau: 0.07,
                noise_tau: 0.13,
                noise_level: 1.0,
                noise_hp: 900.0,
                noise_lp: 6500.0,
                length: 0.4,
            },
            hat_tau: 0.035,
            open_hat_tau: 0.22,
            hat_scale: 0.85,
            clap_tail: 0.12,
            tom_base: 85.0,
            cymbal_tau: 0.7,
            lofi: true,
        },
        DrumKit::Jungle => Voicing {
            kick: KickSpec {
                f_start: 230.0,
                f_end: 55.0,
                pitch_tau: 0.018,
                amp_tau: 0.12,
                length: 0.35,
                click: 0.8,
                drive: 0.45,
            },
            snare: SnareSpec {
                tone: 255.0,
                tone_tau: 0.04,
                noise_tau: 0.07,
                noise_level: 1.0,
                noise_hp: 2200.0,
                noise_lp: 11000.0,
                length: 0.25,
            },
            hat_tau: 0.018,
            open_hat_tau: 0.18,
            hat_scale: 1.15,
            clap_tail: 0.1,
            tom_base: 120.0,
            cymbal_tau: 1.1,
            lofi: false,
        },
        DrumKit::TwoStep => Voicing {
            kick: KickSpec {
                f_start: 140.0,
                f_end: 52.0,
                pitch_tau: 0.03,
                amp_tau: 0.2,
                length: 0.5,
                click: 0.35,
                drive: 0.25,
            },
            snare: SnareSpec {
                tone: 330.0,
                tone_tau: 0.025,
                noise_tau: 0.06,
                noise_level: 0.7,
                noise_hp: 2500.0,
                noise_lp: 10000.0,
                length: 0.22,
            },
            hat_tau: 0.02,
            open_hat_tau: 0.15,
            hat_scale: 1.1,
            clap_tail: 0.14,
            tom_base: 140.0,
            cymbal_tau: 0.6,
            lofi: false,
        },
    }
}

fn render(kit: DrumKit, kind: DrumKind, sr: f32) -> Vec<f32> {
    let v = voicing(kit);
    let seed = 0x9E37_79B9 ^ ((kit as u32) << 8) ^ (kind as u32);
    let mut out = match kind {
        DrumKind::Kick => kick(v.kick, sr, seed),
        DrumKind::Snare => snare(v.snare, sr, seed),
        DrumKind::Clap => clap(v.clap_tail, sr, seed),
        DrumKind::ClosedHat => hat(v.hat_tau, v.hat_scale, sr, seed),
        DrumKind::OpenHat => hat(v.open_hat_tau, v.hat_scale, sr, seed),
        DrumKind::LowTom => tom(v.tom_base, sr),
        DrumKind::MidTom => tom(v.tom_base * 1.33, sr),
        DrumKind::HighTom => tom(v.tom_base * 1.78, sr),
        DrumKind::Cymbal => cymbal(v.cymbal_tau, v.hat_scale, sr, seed),
        DrumKind::Shaker => shaker(sr, seed),
    };
    if v.lofi {
        lofi(&mut out, sr);
    }
    normalize(&mut out, 0.92);
    out
}

fn kick(k: KickSpec, sr: f32, seed: u32) -> Vec<f32> {
    let n = (k.length * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    let mut noise = Noise(seed);
    let mut hp = OnePole::new(2500.0, sr);
    for i in 0..n {
        let t = i as f32 / sr;
        let f = k.f_end + (k.f_start - k.f_end) * (-t / k.pitch_tau).exp();
        phase = (phase + f / sr) % 1.0;
        let body = (phase * TAU).sin() * (-t / k.amp_tau).exp();
        let click = hp.hp(noise.next()) * k.click * (-t / 0.0025).exp();
        let g = 1.0 + k.drive * 4.0;
        let s = ((body + click) * g).tanh() / g.tanh();
        // 3 ms fade at the end avoids a click on long 808 tails.
        let tail = ((n - i) as f32 / (0.003 * sr)).min(1.0);
        out.push(s * tail);
    }
    out
}

fn snare(s: SnareSpec, sr: f32, seed: u32) -> Vec<f32> {
    let n = (s.length * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut noise = Noise(seed);
    let mut hp = OnePole::new(s.noise_hp, sr);
    let mut lp = OnePole::new(s.noise_lp, sr);
    let (mut p1, mut p2) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / sr;
        let bend = 1.0 + 0.5 * (-t / 0.01).exp();
        p1 = (p1 + s.tone * bend / sr) % 1.0;
        p2 = (p2 + s.tone * 1.48 * bend / sr) % 1.0;
        let tone = ((p1 * TAU).sin() + 0.5 * (p2 * TAU).sin()) * (-t / s.tone_tau).exp();
        let nz = lp.lp(hp.hp(noise.next())) * s.noise_level * (-t / s.noise_tau).exp();
        out.push(tone * 0.7 + nz * 1.6);
    }
    out
}

fn clap(tail: f32, sr: f32, seed: u32) -> Vec<f32> {
    let n = ((0.03 + tail * 3.0) * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut noise = Noise(seed);
    let mut hp = OnePole::new(900.0, sr);
    let mut lp = OnePole::new(5000.0, sr);
    for i in 0..n {
        let t = i as f32 / sr;
        let mut env = 0.0f32;
        for burst in [0.0, 0.009, 0.018] {
            if t >= burst {
                env = env.max((-(t - burst) / 0.004).exp());
            }
        }
        if t >= 0.024 {
            env = env.max(0.7 * (-(t - 0.024) / tail).exp());
        }
        out.push(lp.lp(hp.hp(noise.next())) * env * 2.0);
    }
    out
}

/// 808-style metallic hat: six detuned square waves plus noise, high-passed.
fn metallic(t: f32, scale: f32) -> f32 {
    const F: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
    F.iter()
        .map(|f| if (t * f * scale * 1.8).fract() < 0.5 { 1.0 } else { -1.0 })
        .sum::<f32>()
        / 6.0
}

fn hat(tau: f32, scale: f32, sr: f32, seed: u32) -> Vec<f32> {
    let n = ((tau * 6.0).max(0.05) * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut noise = Noise(seed);
    let mut hp1 = OnePole::new(7000.0, sr);
    let mut hp2 = OnePole::new(7000.0, sr);
    for i in 0..n {
        let t = i as f32 / sr;
        let raw = metallic(t, scale) * 0.6 + noise.next() * 0.5;
        let env = (-t / tau).exp() * (1.0 - (-t / 0.0005).exp());
        out.push(hp2.hp(hp1.hp(raw)) * env * 2.2);
    }
    out
}

fn cymbal(tau: f32, scale: f32, sr: f32, seed: u32) -> Vec<f32> {
    let n = ((tau * 4.0) * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut noise = Noise(seed);
    let mut hp = OnePole::new(4500.0, sr);
    for i in 0..n {
        let t = i as f32 / sr;
        let raw = metallic(t, scale * 0.8) * 0.5 + noise.next() * 0.6;
        out.push(hp.hp(raw) * (-t / tau).exp() * 1.6);
    }
    out
}

fn tom(base: f32, sr: f32) -> Vec<f32> {
    let n = (0.45 * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / sr;
        let f = base * (1.0 + 0.6 * (-t / 0.04).exp());
        phase = (phase + f / sr) % 1.0;
        out.push((phase * TAU).sin() * (-t / 0.16).exp());
    }
    out
}

fn shaker(sr: f32, seed: u32) -> Vec<f32> {
    let n = (0.12 * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut noise = Noise(seed);
    let mut hp = OnePole::new(6000.0, sr);
    for i in 0..n {
        let t = i as f32 / sr;
        let env = (t / 0.006).min(1.0) * (-t / 0.035).exp();
        out.push(hp.hp(noise.next()) * env * 1.8);
    }
    out
}

/// Dusty sampler character: 12-bit-ish quantise, sample-and-hold, gentle low-pass + warmth.
fn lofi(buf: &mut [f32], sr: f32) {
    let mut lp = OnePole::new(7500.0, sr);
    let mut held = 0.0;
    for (i, s) in buf.iter_mut().enumerate() {
        if i % 2 == 0 {
            held = (*s * 2048.0).round() / 2048.0;
        }
        *s = (lp.lp(held) * 1.3).tanh();
    }
}

fn normalize(buf: &mut [f32], target: f32) {
    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let g = target / peak;
        for s in buf.iter_mut() {
            *s *= g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_sound_renders() {
        let lib = KitLibrary::render_all(44_100.0);
        for &kit in DrumKit::all() {
            for &kind in DrumKind::all() {
                let s = lib.get(kit, kind);
                assert!(s.len() > 1000, "{:?}/{:?} too short", kit, kind);
                assert!(s.iter().all(|x| x.is_finite()));
                let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(peak > 0.5 && peak <= 1.0, "{:?}/{:?} peak {peak}", kit, kind);
            }
        }
    }
}
