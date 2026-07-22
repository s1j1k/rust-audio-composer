use crate::model::instrument::SampleKind;
use std::f32::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct InstrumentSample {
    pub data: Arc<Vec<f32>>,
    pub root_midi: u8,
    pub root_freq: f32,
}

pub struct SampleLibrary {
    samples: std::collections::HashMap<SampleKind, InstrumentSample>,
    cache_dir: PathBuf,
}

impl SampleLibrary {
    pub fn new(sample_rate: f32) -> Self {
        let cache_dir = sample_cache_dir();
        let _ = fs::create_dir_all(&cache_dir);
        let mut map = std::collections::HashMap::new();
        for &kind in SampleKind::all() {
            let sample = Self::load_or_generate_kind(kind, sample_rate, &cache_dir);
            map.insert(kind, sample);
        }
        Self { samples: map, cache_dir }
    }

    pub fn get(&self, kind: SampleKind) -> &InstrumentSample {
        self.samples.get(&kind).expect("sample kind loaded")
    }

    fn load_or_generate_kind(kind: SampleKind, sample_rate: f32, cache_dir: &Path) -> InstrumentSample {
        let path = cache_dir.join(format!("{}.wav", kind.label().to_lowercase()));
        if let Ok(data) = load_wav_f32(&path) {
            if data.len() > 256 {
                return InstrumentSample {
                    data: Arc::new(data),
                    root_midi: 60,
                    root_freq: 261.63,
                };
            }
        }

        let data = generate_sample(kind, sample_rate);
        let _ = save_wav_f32(&path, &data, sample_rate as u32);
        InstrumentSample {
            data: Arc::new(data),
            root_midi: 60,
            root_freq: 261.63,
        }
    }
}

fn sample_cache_dir() -> PathBuf {
    dirs_fallback()
        .join("rust-audio-composer")
        .join("samples")
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn generate_sample(kind: SampleKind, sample_rate: f32) -> Vec<f32> {
    match kind {
        SampleKind::Piano => gen_piano(sample_rate),
        SampleKind::Guitar => karplus_strong(110.0, sample_rate, 0.996, 1.8),
        SampleKind::Bass => gen_bass(sample_rate),
        SampleKind::Strings => gen_strings(sample_rate),
        SampleKind::Flute => gen_flute(sample_rate),
        SampleKind::Brass => gen_brass(sample_rate),
        SampleKind::Organ => gen_organ(sample_rate),
        SampleKind::Pad => gen_pad(sample_rate),
    }
}

fn gen_piano(sr: f32) -> Vec<f32> {
    let dur = (sr * 1.8) as usize;
    let freq = 261.63_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = (-t * 4.5).exp() * (1.0 - (-t * 30.0).exp());
        let h1 = (t * freq * 2.0 * PI).sin();
        let h2 = 0.5 * (t * freq * 2.0 * PI * 2.0).sin();
        let h3 = 0.25 * (t * freq * 2.0 * PI * 3.0).sin();
        let hammer = if t < 0.008 {
            (1.0 - t / 0.008) * pseudo_noise(i as u32) * 0.35
        } else {
            0.0
        };
        *s = (h1 + h2 + h3 + hammer) * env * 0.55;
    }
    normalize(out)
}

fn karplus_strong(freq: f32, sr: f32, decay: f32, seconds: f32) -> Vec<f32> {
    let n = (sr / freq) as usize;
    let total = (sr * seconds) as usize;
    let mut buf: Vec<f32> = (0..n)
        .map(|i| pseudo_noise(i as u32) * 0.9)
        .collect();
    let mut out = Vec::with_capacity(total);
    let mut idx = 0usize;
    while out.len() < total {
        let next = (buf[idx] + buf[(idx + 1) % n]) * 0.5 * decay;
        buf[idx] = next;
        out.push(next);
        idx = (idx + 1) % n;
    }
    normalize(out)
}

fn gen_bass(sr: f32) -> Vec<f32> {
    let dur = (sr * 1.4) as usize;
    let freq = 82.41_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = (-t * 2.8).exp() * 0.7 + (-t * 0.8).exp() * 0.3;
        let body = (t * freq * 2.0 * PI).sin();
        let harm = 0.35 * (t * freq * 2.0 * PI * 2.0).sin();
        *s = (body + harm) * env * 0.65;
    }
    normalize(out)
}

fn gen_strings(sr: f32) -> Vec<f32> {
    let dur = (sr * 2.5) as usize;
    let freq = 220.0_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let bow = (1.0 - (-t * 6.0).exp()) * (-t * 0.35).exp();
        let vib = 1.0 + 0.004 * (t * 5.5 * 2.0 * PI).sin();
        let tone = (t * freq * vib * 2.0 * PI).sin()
            + 0.45 * (t * freq * vib * 2.0 * PI * 2.0).sin()
            + 0.2 * (t * freq * vib * 2.0 * PI * 3.0).sin();
        let scrape = pseudo_noise(i as u32) * 0.04 * bow;
        *s = (tone + scrape) * bow * 0.5;
    }
    normalize(out)
}

fn gen_flute(sr: f32) -> Vec<f32> {
    let dur = (sr * 1.6) as usize;
    let freq = 440.0_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = (1.0 - (-t * 8.0).exp()) * (-t * 0.6).exp();
        let breath = pseudo_noise(i as u32) * 0.08;
        let tone = (t * freq * 2.0 * PI).sin() + 0.15 * (t * freq * 2.0 * PI * 2.0).sin();
        *s = (tone + breath) * env * 0.55;
    }
    normalize(out)
}

fn gen_brass(sr: f32) -> Vec<f32> {
    let dur = (sr * 1.2) as usize;
    let freq = 196.0_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = (1.0 - (-t * 12.0).exp()) * (-t * 0.9).exp();
        let tone = (t * freq * 2.0 * PI).sin()
            + 0.6 * (t * freq * 2.0 * PI * 2.0).sin()
            + 0.3 * (t * freq * 2.0 * PI * 3.0).sin();
        *s = tone * env * 0.45;
    }
    normalize(out)
}

fn gen_organ(sr: f32) -> Vec<f32> {
    let dur = (sr * 2.0) as usize;
    let freq = 261.63_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = (1.0 - (-t * 20.0).exp()) * (-t * 0.25).exp();
        let drawbars = [1.0, 0.7, 0.45, 0.3, 0.15];
        let mut tone = 0.0;
        for (h, &amp) in drawbars.iter().enumerate() {
            tone += amp * (t * freq * (h as f32 + 1.0) * 2.0 * PI).sin();
        }
        *s = tone * env * 0.25;
    }
    normalize(out)
}

fn gen_pad(sr: f32) -> Vec<f32> {
    let dur = (sr * 3.0) as usize;
    let freq = 130.81_f32;
    let mut out = vec![0.0; dur];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = (1.0 - (-t * 1.5).exp()) * (-t * 0.18).exp();
        let l1 = (t * freq * 2.0 * PI).sin();
        let l2 = (t * freq * 2.0 * PI * 1.01).sin();
        let l3 = (t * freq * 2.0 * PI * 0.995).sin();
        *s = (l1 + l2 + l3) / 3.0 * env * 0.5;
    }
    normalize(out)
}

fn pseudo_noise(seed: u32) -> f32 {
    let x = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn normalize(mut data: Vec<f32>) -> Vec<f32> {
    let peak = data.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    if peak > 0.0 {
        for s in &mut data {
            *s /= peak * 1.05;
        }
    }
    data
}

fn save_wav_f32(path: &Path, data: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in data {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

fn load_wav_f32(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let samples: Result<Vec<f32>, _> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect();
    samples.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_all_samples() {
        for &kind in SampleKind::all() {
            let data = generate_sample(kind, 44100.0);
            assert!(data.len() > 1000, "{:?} sample too short", kind);
        }
    }
}
