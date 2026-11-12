//! The user's sample instruments: one recorded note per sample, imported from Freesound
//! or from a folder on disk (packs downloaded from soundpacks.com, VCSL, …).
//!
//! Each instrument lives in its own folder under `~/rust-audio-composer/instruments/`:
//! an `instrument.json` listing which note each sample is, where it came from and its
//! licence, plus the samples as mono WAV files. Playing a note picks the nearest sampled
//! note and repitches it, like the built-in multi-sampled piano.
//!
//! Deliberately no loops or beats: only single notes, so music is built from scratch.

pub mod freesound;
pub mod notes;

use crate::audio::multisample::{Zone, ZoneMap};
use crate::audio::samples::{home_dir, load_wav_with_rate, resample, save_wav_f32};
use crate::model::instrument::SynthParams;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

/// Where a sample came from, so it can be credited as its licence requires.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SampleCredit {
    pub title: String,
    pub author: String,
    /// e.g. "Creative Commons 0", "Attribution", "Attribution NonCommercial", or
    /// whatever the pack's licence file says for local imports.
    pub license: String,
    pub url: String,
    /// "Freesound", "Folder import", …
    pub source: String,
}

/// Licences a sample may have to be added to the library: ones that allow using it in
/// music you release or sell. (NonCommercial, Sampling+ and unknown licences are refused.)
pub const LICENCES_FOR_SONGS: &[&str] = &[
    "Creative Commons 0",
    "Attribution",
    "Royalty-free for music",
];

impl SampleCredit {
    /// May this sample be used in a song you release or sell?
    pub fn allowed_in_songs(&self) -> bool {
        LICENCES_FOR_SONGS.contains(&self.license.as_str())
    }

    /// Licences that don't allow commercial use (fine for personal music).
    pub fn non_commercial(&self) -> bool {
        self.license.to_ascii_lowercase().contains("noncommercial")
    }

    pub fn needs_attribution(&self) -> bool {
        let l = self.license.to_ascii_lowercase();
        !(l.contains("creative commons 0") || l.contains("cc0") || l.is_empty())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryZone {
    /// MIDI note the sample was recorded at.
    pub midi: u8,
    /// WAV file name inside the instrument's folder.
    pub file: String,
    pub credit: SampleCredit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryInstrument {
    pub name: String,
    pub zones: Vec<LibraryZone>,
    /// Release time in seconds after the key is let go.
    #[serde(default = "default_release")]
    pub release: f32,
}

fn default_release() -> f32 {
    0.35
}

impl LibraryInstrument {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), zones: Vec::new(), release: default_release() }
    }

    /// "C2 – G5" (or "empty").
    pub fn range_label(&self) -> String {
        let lo = self.zones.iter().map(|z| z.midi).min();
        let hi = self.zones.iter().map(|z| z.midi).max();
        match (lo, hi) {
            (Some(lo), Some(hi)) => format!(
                "{} – {}",
                crate::music::pitch_to_name(lo),
                crate::music::pitch_to_name(hi)
            ),
            _ => "empty".to_string(),
        }
    }

    /// One line per distinct credit, for an attribution list.
    pub fn credits(&self) -> Vec<SampleCredit> {
        let mut out: Vec<SampleCredit> = Vec::new();
        for z in &self.zones {
            if !out.contains(&z.credit) {
                out.push(z.credit.clone());
            }
        }
        out
    }
}

// ---- Storage ---------------------------------------------------------------------

pub fn library_dir() -> PathBuf {
    home_dir().join("rust-audio-composer").join("instruments")
}

/// Folder-safe version of a name.
pub fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "instrument".to_string() } else { s }
}

fn instrument_dir(name: &str) -> PathBuf {
    library_dir().join(slug(name))
}

fn instruments() -> &'static RwLock<Vec<LibraryInstrument>> {
    static LIST: OnceLock<RwLock<Vec<LibraryInstrument>>> = OnceLock::new();
    LIST.get_or_init(|| RwLock::new(load_all_from_disk()))
}

fn load_all_from_disk() -> Vec<LibraryInstrument> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(library_dir()) else { return out };
    for entry in entries.flatten() {
        let json = entry.path().join("instrument.json");
        if let Ok(text) = fs::read_to_string(&json) {
            if let Ok(inst) = serde_json::from_str::<LibraryInstrument>(&text) {
                out.push(inst);
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Every installed sample instrument.
pub fn list() -> Vec<LibraryInstrument> {
    instruments().read().map(|l| l.clone()).unwrap_or_default()
}

pub fn names() -> Vec<String> {
    list().into_iter().map(|i| i.name).collect()
}

pub fn get(name: &str) -> Option<LibraryInstrument> {
    list().into_iter().find(|i| i.name == name)
}

fn save(inst: &LibraryInstrument) -> Result<(), String> {
    let dir = instrument_dir(&inst.name);
    fs::create_dir_all(&dir).map_err(|e| format!("Can't create {}: {e}", dir.display()))?;
    let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
    fs::write(dir.join("instrument.json"), json).map_err(|e| e.to_string())?;
    if let Ok(mut list) = instruments().write() {
        match list.iter_mut().find(|i| i.name == inst.name) {
            Some(existing) => *existing = inst.clone(),
            None => {
                list.push(inst.clone());
                list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }
    }
    invalidate(&inst.name);
    Ok(())
}

/// Remove an instrument and its samples from disk.
pub fn delete(name: &str) -> Result<(), String> {
    let dir = instrument_dir(name);
    if dir.starts_with(library_dir()) && dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    if let Ok(mut list) = instruments().write() {
        list.retain(|i| i.name != name);
    }
    invalidate(name);
    Ok(())
}

pub fn set_release(name: &str, release: f32) -> Result<(), String> {
    let mut inst = get(name).ok_or("No such instrument")?;
    inst.release = release.clamp(0.02, 4.0);
    save(&inst)
}

/// Remove one sampled note from an instrument.
pub fn remove_zone(name: &str, midi: u8) -> Result<(), String> {
    let mut inst = get(name).ok_or("No such instrument")?;
    if let Some(z) = inst.zones.iter().find(|z| z.midi == midi) {
        let _ = fs::remove_file(instrument_dir(name).join(&z.file));
    }
    inst.zones.retain(|z| z.midi != midi);
    save(&inst)
}

/// A name that isn't taken yet by a library instrument or by one of `others`.
pub fn unique_name(wanted: &str, others: &[String]) -> String {
    let taken = |n: &str| names().iter().any(|x| x == n) || others.iter().any(|x| x == n);
    let base = if wanted.trim().is_empty() { "My instrument" } else { wanted.trim() };
    if !taken(base) {
        return base.to_string();
    }
    (2..).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap()
}

/// Add (or replace) the sample for `midi` in instrument `name`, creating the instrument
/// if needed. `samples` is mono audio at `rate`; it is trimmed, faded and normalised.
pub fn add_sample(
    name: &str,
    midi: u8,
    samples: Vec<f32>,
    rate: f32,
    credit: SampleCredit,
) -> Result<(), String> {
    if !credit.allowed_in_songs() {
        return Err(format!(
            "\"{}\" has the licence \"{}\", which doesn't allow use in songs",
            credit.title, credit.license
        ));
    }
    let mut inst = get(name).unwrap_or_else(|| LibraryInstrument::new(name));
    let cleaned = prepare_note_sample(samples, rate);
    if cleaned.len() < (rate * 0.02) as usize {
        return Err(format!("\"{}\" is silent or too short", credit.title));
    }
    let file = format!("{}-{}.wav", midi, slug(&credit.title).chars().take(40).collect::<String>());
    let dir = instrument_dir(name);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if let Some(old) = inst.zones.iter().find(|z| z.midi == midi) {
        if old.file != file {
            let _ = fs::remove_file(dir.join(&old.file));
        }
    }
    save_wav_f32(&dir.join(&file), &cleaned, rate.round() as u32)?;
    inst.zones.retain(|z| z.midi != midi);
    inst.zones.push(LibraryZone { midi, file, credit });
    inst.zones.sort_by_key(|z| z.midi);
    save(&inst)
}

/// Trim leading silence, cap the length, normalise the peak and fade the tail, so
/// samples from different sources sit at similar levels and start on the beat.
pub fn prepare_note_sample(mut samples: Vec<f32>, rate: f32) -> Vec<f32> {
    let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if peak < 1e-4 {
        return Vec::new();
    }
    let gate = peak * 0.02;
    let start = samples.iter().position(|s| s.abs() >= gate).unwrap_or(0);
    // Keep a couple of milliseconds before the onset so the attack isn't clipped.
    let start = start.saturating_sub((rate * 0.002) as usize);
    samples.drain(..start);
    let max_len = (rate * 12.0) as usize;
    samples.truncate(max_len);
    // Drop a silent tail.
    let end = samples.iter().rposition(|s| s.abs() >= peak * 0.001).map(|i| i + 1).unwrap_or(0);
    samples.truncate(end);
    let gain = 0.9 / peak;
    for s in &mut samples {
        *s *= gain;
    }
    let fade = ((rate * 0.02) as usize).min(samples.len());
    let n = samples.len();
    for i in 0..fade {
        samples[n - fade + i] *= 1.0 - (i as f32 / fade as f32);
    }
    samples
}

// ---- Decoding ----------------------------------------------------------------------

/// Decode any supported audio file (WAV, FLAC, OGG Vorbis, MP3, AIFF) to mono samples.
pub fn decode_to_mono(bytes: Vec<u8>, extension_hint: Option<&str>) -> Result<(Vec<f32>, f32), String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = extension_hint {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("Unsupported audio file ({e})"))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or("No audio track in file")?
        .clone();
    let rate = track.codec_params.sample_rate.ok_or("Unknown sample rate")? as f32;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Can't decode ({e})"))?;

    let mut mono = Vec::new();
    let mut buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break, // end of stream (or a truncated file: keep what we have)
        };
        if packet.track_id() != track.id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        };
        let spec = *decoded.spec();
        let channels = spec.channels.count().max(1);
        let sb = buf.get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
        if sb.capacity() < decoded.capacity() {
            *sb = SampleBuffer::new(decoded.capacity() as u64, spec);
        }
        sb.copy_interleaved_ref(decoded);
        for frame in sb.samples().chunks(channels) {
            mono.push(frame.iter().sum::<f32>() / channels as f32);
        }
        if mono.len() as f32 > rate * 30.0 {
            break; // single notes only; don't decode a whole song by accident
        }
    }
    if mono.is_empty() {
        return Err("The file contains no audio".to_string());
    }
    Ok((mono, rate))
}

// ---- Playback (zone maps) ------------------------------------------------------------

#[derive(Clone)]
enum LoadState {
    Loading,
    Ready(Arc<ZoneMap>),
    Failed,
}

type Cache = Mutex<HashMap<(String, u32), LoadState>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn invalidate(name: &str) {
    if let Ok(mut c) = cache().lock() {
        c.retain(|(n, _), _| n != name);
    }
}

fn build_zone_map(name: &str, sample_rate: f32) -> Option<ZoneMap> {
    let inst = get(name)?;
    let dir = instrument_dir(name);
    let mut zones = Vec::new();
    for z in &inst.zones {
        if let Ok((data, rate)) = load_wav_with_rate(&dir.join(&z.file)) {
            let data = Arc::new(resample(&data, rate, sample_rate));
            zones.push(Zone { root_midi: z.midi, soft: data.clone(), hard: data });
        }
    }
    if zones.is_empty() {
        None
    } else {
        Some(ZoneMap { zones })
    }
}

/// The loaded zones for `name`, or None while they load in the background (the voice
/// falls back to the built-in piano for those first notes).
pub fn zones(name: &str, sample_rate: f32) -> Option<Arc<ZoneMap>> {
    let key = (name.to_string(), sample_rate as u32);
    let mut c = cache().lock().unwrap_or_else(|e| e.into_inner());
    match c.get(&key) {
        Some(LoadState::Ready(z)) => Some(z.clone()),
        Some(_) => None,
        None => {
            c.insert(key.clone(), LoadState::Loading);
            let name = name.to_string();
            std::thread::Builder::new()
                .name(format!("library-{name}"))
                .spawn(move || {
                    let state = match build_zone_map(&name, sample_rate) {
                        Some(z) => LoadState::Ready(Arc::new(z)),
                        None => LoadState::Failed,
                    };
                    if let Ok(mut c) = cache().lock() {
                        c.insert(key, state);
                    }
                })
                .ok();
            None
        }
    }
}

/// Load synchronously (for WAV export, which can't wait for a background thread).
pub fn load_blocking(name: &str, sample_rate: f32) {
    let key = (name.to_string(), sample_rate as u32);
    if matches!(cache().lock().ok().and_then(|c| c.get(&key).cloned()), Some(LoadState::Ready(_))) {
        return;
    }
    if let Some(z) = build_zone_map(name, sample_rate) {
        if let Ok(mut c) = cache().lock() {
            c.insert(key, LoadState::Ready(Arc::new(z)));
        }
    }
}

/// Voice settings for a library instrument: the sample plays as recorded (no extra decay
/// or filtering) until the key is released.
pub fn synth_params(name: &str) -> SynthParams {
    let release = get(name).map(|i| i.release).unwrap_or_else(default_release);
    SynthParams {
        attack: 0.002,
        decay: 0.1,
        sustain: 1.0,
        release,
        brightness: 1.0,
        filter_cutoff: 1.0,
        library: Some(name.to_string()),
        ..SynthParams::default()
    }
}

/// Folder to remember the Freesound API key in (outside any project or repository).
pub fn settings_path() -> PathBuf {
    home_dir().join("rust-audio-composer").join("settings.json")
}

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    freesound_api_key: String,
}

pub fn load_api_key() -> String {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Settings>(&t).ok())
        .map(|s| s.freesound_api_key)
        .unwrap_or_default()
}

pub fn save_api_key(key: &str) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let s = Settings { freesound_api_key: key.trim().to_string() };
    fs::write(&path, serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Read a file and decode it, using its extension as a format hint.
pub fn decode_file(path: &Path) -> Result<(Vec<f32>, f32), String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
    decode_to_mono(bytes, ext.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_folder_safe() {
        assert_eq!(slug("Felt Piano (soft)!"), "felt-piano-soft");
        assert_eq!(slug("???"), "instrument");
    }

    #[test]
    fn prepare_trims_normalises_and_fades() {
        let rate = 1000.0;
        let mut s = vec![0.0; 100];
        s.extend((0..500).map(|i| 0.5 * (i as f32 * 0.3).sin()));
        s.extend(vec![0.0; 200]);
        let out = prepare_note_sample(s, rate);
        assert!(out.len() < 520 && out.len() > 480, "{}", out.len());
        let peak = out.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.9).abs() < 0.05);
        assert!(out.last().unwrap().abs() < 0.05);
    }

    #[test]
    fn only_song_safe_licences_are_accepted() {
        let with = |l: &str| SampleCredit { license: l.into(), ..Default::default() };
        assert!(with("Creative Commons 0").allowed_in_songs());
        assert!(with("Attribution").allowed_in_songs());
        assert!(!with("Attribution NonCommercial").allowed_in_songs());
        assert!(!with("Sampling+").allowed_in_songs());
        assert!(!with("").allowed_in_songs());
        let err = add_sample("x", 60, vec![0.5; 100], 1000.0, with("Attribution NonCommercial"));
        assert!(err.unwrap_err().contains("doesn't allow"));
    }

    #[test]
    fn credits_flag_licences() {
        let c = SampleCredit { license: "Attribution NonCommercial".into(), ..Default::default() };
        assert!(c.non_commercial() && c.needs_attribution());
        let c0 = SampleCredit { license: "Creative Commons 0".into(), ..Default::default() };
        assert!(!c0.non_commercial() && !c0.needs_attribution());
    }

    #[test]
    fn decodes_a_wav() {
        let rate = 22050u32;
        let path = std::env::temp_dir().join("chordaw-decode-test.wav");
        let tone: Vec<f32> = (0..2205).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        save_wav_f32(&path, &tone, rate).unwrap();
        let (mono, r) = decode_file(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(r, rate as f32);
        assert!((mono.len() as i64 - 2205).abs() < 10);
    }
}
