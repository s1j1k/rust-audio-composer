use crate::audio::synth::SynthEngine;
use crate::model::instrument::{CustomInstrument, InstrumentProfile, SynthParams};
use crate::model::Project;
use std::collections::{HashMap, HashSet};

pub fn render_project_to_buffer(
    project: &Project,
    custom_instruments: &[CustomInstrument],
    sample_rate: f32,
    master_volume: f32,
) -> Vec<f32> {
    let total_samples =
        (project.duration_secs() * sample_rate as f64).ceil() as u64 + (sample_rate as u64);
    let mut output = vec![0.0_f32; total_samples as usize];

    let mut synth = SynthEngine::new(sample_rate);
    let mut profiles: HashMap<String, InstrumentProfile> = crate::model::instrument::default_profiles()
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();

    for custom in custom_instruments {
        profiles.insert(custom.name.clone(), InstrumentProfile::from_custom(custom));
    }

    let mut scheduled: Vec<(u64, u8, f32, SynthParams, bool)> = Vec::new();

    for track in &project.tracks {
        if track.muted {
            continue;
        }
        let profile_name = track.instrument.label();
        let params = profiles
            .get(&profile_name)
            .map(|p| p.params.clone())
            .unwrap_or_else(|| {
                crate::model::instrument::default_params_for_kind(
                    crate::model::instrument::SampleKind::Piano,
                )
            });

        for note in &track.notes {
            let start =
                (note.start_beat * project.beat_duration_secs() * sample_rate as f64) as u64;
            let end =
                (note.end_beat() * project.beat_duration_secs() * sample_rate as f64) as u64;
            let vel = note.velocity * track.volume * master_volume;
            scheduled.push((start, note.pitch, vel, params.clone(), false));
            scheduled.push((end, note.pitch, vel, params.clone(), true));
        }
    }

    scheduled.sort_by_key(|(s, _, _, _, _)| *s);

    let mut active: HashSet<u8> = HashSet::new();
    let mut event_idx = 0usize;

    for (i, out) in output.iter_mut().enumerate() {
        let t = i as u64;
        while event_idx < scheduled.len() && scheduled[event_idx].0 <= t {
            let (_, pitch, vel, params, is_off) = &scheduled[event_idx];
            if *is_off {
                synth.note_off(*pitch);
                active.remove(pitch);
            } else if active.insert(*pitch) {
                synth.note_on(*pitch, *vel, params.clone());
            }
            event_idx += 1;
        }
        *out = synth.next_sample();
    }

    output
}

pub fn export_project_wav(
    project: &Project,
    custom_instruments: &[CustomInstrument],
    path: &std::path::Path,
    sample_rate: f32,
    master_volume: f32,
) -> Result<(), String> {
    let buffer = render_project_to_buffer(project, custom_instruments, sample_rate, master_volume);
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
