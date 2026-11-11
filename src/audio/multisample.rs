//! Multi-sampled, multi-velocity-layer instruments (CC0 sources).
//!
//! The single-sample instruments in `samples.rs` stretch one recording across the whole
//! keyboard and only change volume with velocity. Here an instrument is a set of *zones*
//! (one recording every few semitones) with a **soft and a hard layer** per zone; the
//! voice crossfades between layers by velocity, so a soft touch sounds genuinely softer
//! (darker, rounder), not just quieter — the way a real piano or harp behaves.
//!
//! Zones download in a background thread on first use and are cached on disk; until they
//! are ready the instrument falls back to its single sample, so the app never blocks.

use crate::audio::sample_sources::SampleFileFormat;
use crate::audio::samples::{decode_sample_raw, fetch_url_bytes, home_dir, load_wav_with_rate, resample, save_wav_f32};
use crate::model::instrument::SampleKind;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

pub struct Zone {
    pub root_midi: u8,
    pub soft: Arc<Vec<f32>>,
    pub hard: Arc<Vec<f32>>,
}

pub struct ZoneMap {
    pub zones: Vec<Zone>,
}

/// What a voice needs to play one note from a zone map.
pub struct ZonePick {
    pub soft: Arc<Vec<f32>>,
    pub hard: Arc<Vec<f32>>,
    pub root_freq: f32,
    /// 0 = only the soft layer, 1 = only the hard layer.
    pub hard_mix: f32,
}

impl ZoneMap {
    pub fn pick(&self, pitch: u8, velocity: f32) -> Option<ZonePick> {
        let zone = self
            .zones
            .iter()
            .min_by_key(|z| (z.root_midi as i32 - pitch as i32).abs())?;
        let x = ((velocity - 0.3) / 0.5).clamp(0.0, 1.0);
        let hard_mix = x * x * (3.0 - 2.0 * x);
        Some(ZonePick {
            soft: zone.soft.clone(),
            hard: zone.hard.clone(),
            root_freq: 440.0 * 2.0_f32.powf((zone.root_midi as f32 - 69.0) / 12.0),
            hard_mix,
        })
    }
}

struct MultiSpec {
    base_url: &'static str,
    format: SampleFileFormat,
    /// (root midi note, soft-layer file, hard-layer file)
    zones: &'static [(u8, &'static str, &'static str)],
    max_seconds: f32,
}

const PIANO_URL: &str = "https://raw.githubusercontent.com/freepats/upright-piano-KW/main/samples/";
const HARP_URL: &str =
    "https://raw.githubusercontent.com/sgossner/VCSL/master/Chordophones/Composite%20Chordophones/Concert%20Harp/";

// FreePats Upright Piano KW (CC0): zones on C and F#, vL = soft, vH = hard.
// C4 only exists as vL, so it is used for both layers.
const PIANO_ZONES: &[(u8, &str, &str)] = &[
    (24, "C1vL.flac", "C1vH.flac"),
    (30, "F%231vL.flac", "F%231vH.flac"),
    (36, "C2vL.flac", "C2vH.flac"),
    (42, "F%232vL.flac", "F%232vH.flac"),
    (48, "C3vL.flac", "C3vH.flac"),
    (54, "F%233vL.flac", "F%233vH.flac"),
    (60, "C4vL.flac", "C4vL.flac"),
    (66, "F%234vL.flac", "F%234vH.flac"),
    (72, "C5vL.flac", "C5vH.flac"),
    (78, "F%235vL.flac", "F%235vH.flac"),
    (84, "C6vL.flac", "C6vH.flac"),
    (90, "F%236vL.flac", "F%236vH.flac"),
    (96, "C7vL.flac", "C7vH.flac"),
    (102, "F%237vL.flac", "F%237vH.flac"),
    (108, "C8vL.flac", "C8vH.flac"),
];

// VCSL Concert Harp (CC0): mezzo-forte = soft layer, forte = hard layer.
const HARP_ZONES: &[(u8, &str, &str)] = &[
    (38, "KSHarp_D2_mf1.wav", "KSHarp_D2_f1.wav"),
    (45, "KSHarp_A2_mf1.wav", "KSHarp_A2_f1.wav"),
    (55, "KSHarp_G3_mf1.wav", "KSHarp_G3_f1.wav"),
    (62, "KSHarp_D4_mf1.wav", "KSHarp_D4_f1.wav"),
    (69, "KSHarp_A4_mf1.wav", "KSHarp_A4_f1.wav"),
    (79, "KSHarp_G5_mf1.wav", "KSHarp_G5_f1.wav"),
    (86, "KSHarp_D6_mf1.wav", "KSHarp_D6_f1.wav"),
];

fn spec_for(kind: SampleKind) -> Option<MultiSpec> {
    match kind {
        SampleKind::Piano => Some(MultiSpec {
            base_url: PIANO_URL,
            format: SampleFileFormat::Flac,
            zones: PIANO_ZONES,
            max_seconds: 7.0,
        }),
        SampleKind::Harp => Some(MultiSpec {
            base_url: HARP_URL,
            format: SampleFileFormat::Wav,
            zones: HARP_ZONES,
            max_seconds: 5.0,
        }),
        _ => None,
    }
}

pub fn has_multisample(kind: SampleKind) -> bool {
    spec_for(kind).is_some()
}

#[derive(Clone)]
enum LoadState {
    Loading { done: usize, total: usize },
    Ready(Arc<ZoneMap>),
    Failed,
}

type Cache = Mutex<HashMap<(SampleKind, u32), LoadState>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Serialises downloads so two engines at different sample rates don't fetch twice.
fn download_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Returns the zone map if it is loaded; otherwise starts loading it (once) and returns None.
pub fn zones(kind: SampleKind, sample_rate: f32) -> Option<Arc<ZoneMap>> {
    spec_for(kind)?;
    let key = (kind, sample_rate as u32);
    let mut map = cache().lock().unwrap_or_else(|e| e.into_inner());
    match map.get(&key) {
        Some(LoadState::Ready(z)) => Some(z.clone()),
        Some(_) => None,
        None => {
            let total = spec_for(kind).map(|s| s.zones.len()).unwrap_or(0);
            map.insert(key, LoadState::Loading { done: 0, total });
            std::thread::Builder::new()
                .name(format!("multisample-{}", kind.label()))
                .spawn(move || load(kind, sample_rate))
                .ok();
            None
        }
    }
}

/// Human-readable loading status for the UI ("Piano: 6/15 zones…").
pub fn status(kind: SampleKind, sample_rate: f32) -> Option<String> {
    let map = cache().lock().unwrap_or_else(|e| e.into_inner());
    match map.get(&(kind, sample_rate as u32))? {
        LoadState::Loading { done, total } => Some(format!(
            "{}: downloading velocity layers {}/{}…",
            kind.label(),
            done,
            total
        )),
        LoadState::Ready(z) => Some(format!(
            "{}: {} zones × 2 velocity layers ready",
            kind.label(),
            z.zones.len()
        )),
        LoadState::Failed => Some(format!(
            "{}: multi-sample download failed (using single sample)",
            kind.label()
        )),
    }
}

fn cache_dir(kind: SampleKind) -> PathBuf {
    home_dir()
        .join("rust-audio-composer")
        .join("multisamples")
        .join(kind.label().to_lowercase().replace(' ', "-"))
}

fn load_layer(spec: &MultiSpec, file: &str, kind: SampleKind, sr: f32) -> Option<Vec<f32>> {
    let dir = cache_dir(kind);
    let _ = fs::create_dir_all(&dir);
    let cached = dir.join(format!("{}.wav", file.replace("%23", "s")));
    let (data, rate) = match load_wav_with_rate(&cached) {
        Ok(ok) if ok.0.len() > 256 => ok,
        _ => {
            let _guard = download_lock().lock().unwrap_or_else(|e| e.into_inner());
            // Another thread may have fetched it while we waited.
            if let Ok(ok) = load_wav_with_rate(&cached) {
                if ok.0.len() > 256 {
                    return Some(trim(resample(&ok.0, ok.1, sr), spec.max_seconds, sr));
                }
            }
            let bytes = fetch_url_bytes(&format!("{}{}", spec.base_url, file))?;
            let (raw, rate) = decode_sample_raw(&bytes, spec.format)?;
            // Keep a few seconds at source rate; that is all a voice ever plays.
            let raw = trim(raw, spec.max_seconds, rate);
            let _ = save_wav_f32(&cached, &raw, rate as u32);
            (raw, rate)
        }
    };
    Some(trim(resample(&data, rate, sr), spec.max_seconds, sr))
}

fn trim(mut data: Vec<f32>, max_seconds: f32, sr: f32) -> Vec<f32> {
    let max = (max_seconds * sr) as usize;
    if data.len() > max {
        data.truncate(max);
        let fade = ((0.25 * sr) as usize).min(max);
        let start = max - fade;
        for (i, s) in data[start..].iter_mut().enumerate() {
            *s *= 1.0 - i as f32 / fade as f32;
        }
    }
    data
}

fn set_progress(kind: SampleKind, sr: f32, done: usize, total: usize) {
    let mut map = cache().lock().unwrap_or_else(|e| e.into_inner());
    map.insert((kind, sr as u32), LoadState::Loading { done, total });
}

fn load(kind: SampleKind, sr: f32) {
    let Some(spec) = spec_for(kind) else { return };
    let total = spec.zones.len();
    let mut zones = Vec::new();
    for (i, &(root, soft_file, hard_file)) in spec.zones.iter().enumerate() {
        let soft = load_layer(&spec, soft_file, kind, sr);
        let hard = if hard_file == soft_file {
            soft.clone()
        } else {
            load_layer(&spec, hard_file, kind, sr)
        };
        let (soft, hard) = match (soft, hard) {
            (Some(s), Some(h)) => (s, h),
            (Some(s), None) => (s.clone(), s),
            (None, Some(h)) => (h.clone(), h),
            (None, None) => {
                set_progress(kind, sr, i + 1, total);
                continue;
            }
        };
        // Normalise both layers by the *hard* layer's peak so the natural level
        // difference between soft and hard playing is preserved.
        let peak = hard
            .iter()
            .chain(soft.iter())
            .fold(0.0f32, |m, s| m.max(s.abs()))
            .max(1e-6);
        let g = 0.95 / peak;
        zones.push(Zone {
            root_midi: root,
            soft: Arc::new(soft.into_iter().map(|s| s * g).collect()),
            hard: Arc::new(hard.into_iter().map(|s| s * g).collect()),
        });
        set_progress(kind, sr, i + 1, total);
    }
    let state = if zones.len() * 2 >= total {
        LoadState::Ready(Arc::new(ZoneMap { zones }))
    } else {
        LoadState::Failed
    };
    let mut map = cache().lock().unwrap_or_else(|e| e.into_inner());
    map.insert((kind, sr as u32), state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_nearest_zone_and_crossfade() {
        let z = |root| Zone {
            root_midi: root,
            soft: Arc::new(vec![0.1; 10]),
            hard: Arc::new(vec![0.9; 10]),
        };
        let map = ZoneMap {
            zones: vec![z(48), z(60), z(72)],
        };
        let soft = map.pick(61, 0.1).unwrap();
        assert!((soft.root_freq - 261.63).abs() < 0.1);
        assert_eq!(soft.hard_mix, 0.0);
        let hard = map.pick(70, 1.0).unwrap();
        assert!((hard.root_freq - 523.25).abs() < 0.1);
        assert_eq!(hard.hard_mix, 1.0);
        let mid = map.pick(70, 0.55).unwrap();
        assert!(mid.hard_mix > 0.2 && mid.hard_mix < 0.8);
    }

    #[test]
    fn zone_tables_are_sorted() {
        for kind in [SampleKind::Piano, SampleKind::Harp] {
            let spec = spec_for(kind).unwrap();
            assert!(spec.zones.windows(2).all(|w| w[0].0 < w[1].0));
        }
    }
}
