use crate::audio::drum_sources::{self, DrumSource};
use crate::audio::samples::{decode_bytes_to_mono, fetch_url_bytes, load_wav_f32, save_wav_f32};
use crate::model::drum::DrumKind;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DRUM_MANIFEST: &str = "freepats-drums-cc0-v1";

#[derive(Clone)]
pub struct DrumSample {
    pub data: Arc<Vec<f32>>,
    pub from_foss: bool,
}

pub struct DrumLibrary {
    samples: std::collections::HashMap<DrumKind, DrumSample>,
}

impl DrumLibrary {
    pub fn new(sample_rate: f32) -> Self {
        let cache_dir = drum_cache_dir();
        let _ = fs::create_dir_all(&cache_dir);
        ensure_manifest(&cache_dir);
        let mut map = std::collections::HashMap::new();
        for &kind in DrumKind::all() {
            let source = drum_sources::source_for_kind(kind);
            map.insert(kind, load_or_fetch(kind, source, sample_rate, &cache_dir));
        }
        Self { samples: map }
    }

    pub fn get(&self, kind: DrumKind) -> &DrumSample {
        self.samples.get(&kind).expect("drum kind loaded")
    }
}

fn load_or_fetch(
    kind: DrumKind,
    source: DrumSource,
    target_rate: f32,
    cache_dir: &Path,
) -> DrumSample {
    let name = kind.label().to_lowercase().replace(' ', "-");
    let cache_wav = cache_dir.join(format!("{}.foss.wav", name));
    let attribution_path = cache_dir.join(format!("{}.attribution.txt", name));

    if let Ok(data) = load_wav_f32(&cache_wav) {
        if data.len() > 64 {
            return DrumSample {
                data: Arc::new(data),
                from_foss: true,
            };
        }
    }

    if let Some(data) = fetch_and_cache(source, target_rate, &cache_wav, &attribution_path) {
        return data;
    }

    let data = generate_fallback(kind, target_rate);
    let _ = save_wav_f32(&cache_wav, &data, target_rate as u32);
    DrumSample {
        data: Arc::new(data),
        from_foss: false,
    }
}

fn fetch_and_cache(
    source: DrumSource,
    target_rate: f32,
    cache_wav: &Path,
    attribution_path: &Path,
) -> Option<DrumSample> {
    let bytes = fetch_url_bytes(source.url)?;
    let data = decode_bytes_to_mono(&bytes, source.format, target_rate)?;
    if data.len() < 64 {
        return None;
    }
    let _ = save_wav_f32(cache_wav, &data, target_rate as u32);
    let _ = fs::write(
        attribution_path,
        format!(
            "Drum: {}\nLicense: {}\nAttribution: {}\nSource: {}\nURL: {}\n",
            source.kind.label(),
            source.license,
            source.attribution,
            source.project_url,
            source.url,
        ),
    );
    Some(DrumSample {
        data: Arc::new(data),
        from_foss: true,
    })
}

fn ensure_manifest(cache_dir: &Path) {
    let path = cache_dir.join("manifest.txt");
    let current = fs::read_to_string(&path).unwrap_or_default();
    if current.trim() == DRUM_MANIFEST {
        return;
    }
    let _ = fs::create_dir_all(cache_dir);
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            if entry.path() != path {
                let _ = fs::remove_file(entry.path())
                    .or_else(|_| fs::remove_dir_all(entry.path()));
            }
        }
    }
    let _ = fs::write(&path, DRUM_MANIFEST);
}

fn drum_cache_dir() -> PathBuf {
    dirs_home()
        .join("rust-audio-composer")
        .join("drums")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn generate_fallback(kind: DrumKind, sr: f32) -> Vec<f32> {
    let len = (sr * match kind {
        DrumKind::Kick => 0.35,
        DrumKind::Snare => 0.25,
        DrumKind::ClosedHat => 0.08,
        DrumKind::OpenHat => 0.4,
        DrumKind::LowTom | DrumKind::MidTom | DrumKind::HighTom => 0.3,
        DrumKind::Clap => 0.15,
        DrumKind::Cymbal => 0.8,
        DrumKind::Shaker => 0.12,
    }) as usize;
    let freq = match kind {
        DrumKind::Kick => 55.0,
        DrumKind::Snare => 180.0,
        DrumKind::ClosedHat | DrumKind::OpenHat => 8000.0,
        DrumKind::LowTom => 90.0,
        DrumKind::MidTom => 130.0,
        DrumKind::HighTom => 180.0,
        DrumKind::Clap => 1200.0,
        DrumKind::Cymbal => 6000.0,
        DrumKind::Shaker => 5000.0,
    };
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / sr;
        let env = (-t * match kind {
            DrumKind::Kick => 8.0,
            DrumKind::Cymbal | DrumKind::OpenHat => 3.0,
            _ => 12.0,
        })
        .exp();
        let noise = pseudo_noise(i as u32);
        let tone = (t * freq * std::f32::consts::TAU).sin();
        let s = match kind {
            DrumKind::Kick => tone * env,
            DrumKind::Snare | DrumKind::Clap => tone * 0.4 * env + noise * 0.6 * env,
            DrumKind::ClosedHat | DrumKind::OpenHat | DrumKind::Shaker | DrumKind::Cymbal => {
                noise * env
            }
            _ => tone * env + noise * 0.15 * env,
        };
        out.push(s);
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
