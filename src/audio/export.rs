use crate::audio::effects::EffectProcessor;
use crate::audio::synth::SynthEngine;
use crate::model::drum::DrumKind;
use crate::model::effects::MasterEffects;
use crate::model::instrument::{CustomInstrument, InstrumentProfile};
use crate::model::Project;
use std::collections::{HashMap, HashSet};

pub fn render_project_to_buffer(
    project: &Project,
    custom_instruments: &[CustomInstrument],
    sample_rate: f32,
    master_volume: f32,
    master_effects: &MasterEffects,
) -> Vec<f32> {
    let total_samples =
        (project.duration_secs() * sample_rate as f64).ceil() as u64 + (sample_rate as u64);
    let mut output = vec![0.0_f32; total_samples as usize];

    let profiles = build_profiles(custom_instruments);

    for track in &project.tracks {
        if track.muted {
            continue;
        }
        let track_buf = render_track_buffer(project, track, &profiles, sample_rate, master_volume);
        for (o, t) in output.iter_mut().zip(track_buf.iter()) {
            *o += t;
        }
    }

    let mut master_fx = EffectProcessor::new(sample_rate);
    for s in &mut output {
        *s = master_fx.process_master(*s, master_effects);
    }

    output
}

fn build_profiles(custom_instruments: &[CustomInstrument]) -> HashMap<String, InstrumentProfile> {
    let mut profiles: HashMap<String, InstrumentProfile> = crate::model::instrument::default_profiles()
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();
    for custom in custom_instruments {
        profiles.insert(custom.name.clone(), InstrumentProfile::from_custom(custom));
    }
    profiles
}

fn render_track_buffer(
    project: &Project,
    track: &crate::model::Track,
    profiles: &HashMap<String, InstrumentProfile>,
    sample_rate: f32,
    master_volume: f32,
) -> Vec<f32> {
    let total_samples =
        (project.duration_secs() * sample_rate as f64).ceil() as u64 + (sample_rate as u64);
    let mut output = vec![0.0_f32; total_samples as usize];

    let mut synth = SynthEngine::new(sample_rate);
    let mut fx = EffectProcessor::new(sample_rate);
    let profile_name = track.instrument.label();
    let params = profiles
        .get(&profile_name)
        .map(|p| p.params.clone())
        .unwrap_or_else(|| {
            crate::model::instrument::default_params_for_kind(
                crate::model::instrument::SampleKind::Piano,
            )
        });
    let track_filter = track.effects.effective_lowpass();

    let mut scheduled: Vec<(u64, u8, f32, bool)> = Vec::new();
    let is_drum = track.is_drum();

    for note in &track.notes {
        let start =
            (note.start_beat * project.beat_duration_secs() * sample_rate as f64) as u64;
        let vel = note.velocity * track.volume * master_volume;
        scheduled.push((start, note.pitch, vel, false));
        if !is_drum {
            let end =
                (note.end_beat() * project.beat_duration_secs() * sample_rate as f64) as u64;
            scheduled.push((end, note.pitch, vel, true));
        }
    }
    scheduled.sort_by_key(|(s, _, _, _)| *s);

    let mut active: HashSet<u8> = HashSet::new();
    let mut event_idx = 0usize;

    for (i, out) in output.iter_mut().enumerate() {
        let t = i as u64;
        while event_idx < scheduled.len() && scheduled[event_idx].0 <= t {
            let (_, pitch, vel, is_off) = scheduled[event_idx];
            if is_off {
                synth.note_off(pitch);
                active.remove(&pitch);
            } else if is_drum {
                if let Some(kind) = DrumKind::from_midi_pitch(pitch) {
                    synth.drum_hit(kind, vel);
                }
            } else if active.insert(pitch) {
                let mut p = params.clone();
                p.filter_cutoff = (p.filter_cutoff * track_filter).clamp(0.05, 1.0);
                synth.note_on(pitch, vel, p);
            }
            event_idx += 1;
        }
        let dry = synth.next_sample();
        *out = fx.process_track(dry, &track.effects);
    }

    output
}

pub fn export_project_wav(
    project: &Project,
    custom_instruments: &[CustomInstrument],
    path: &std::path::Path,
    sample_rate: f32,
    master_volume: f32,
    master_effects: &MasterEffects,
) -> Result<(), String> {
    let buffer = render_project_to_buffer(
        project,
        custom_instruments,
        sample_rate,
        master_volume,
        master_effects,
    );
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for s in buffer {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}
