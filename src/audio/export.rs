use crate::audio::effects::EffectProcessor;
use crate::audio::engine::trigger_note;
use crate::audio::synth::SynthEngine;
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

    for (index, track) in project.tracks.iter().enumerate() {
        if track.muted {
            continue;
        }
        let track_buf = render_track_buffer(project, index, &profiles, sample_rate, master_volume);
        for (o, t) in output.iter_mut().zip(track_buf.iter()) {
            *o += t;
        }
    }

    let mut master_fx = EffectProcessor::new(sample_rate);
    master_fx.set_tempo(project.bpm);
    master_fx.jump_to(&master_effects.as_track());
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

/// How often (in samples) automation is re-read during export. The effect processor
/// glides between updates, so this is smooth and far finer than the UI frame rate.
const AUTOMATION_STEP: usize = 64;

fn render_track_buffer(
    project: &Project,
    index: usize,
    profiles: &HashMap<String, InstrumentProfile>,
    sample_rate: f32,
    master_volume: f32,
) -> Vec<f32> {
    let track = &project.tracks[index];
    let total_samples =
        (project.duration_secs() * sample_rate as f64).ceil() as u64 + (sample_rate as u64);
    let mut output = vec![0.0_f32; total_samples as usize];

    let mut synth = SynthEngine::new(sample_rate);
    let mut fx = EffectProcessor::new(sample_rate);
    fx.set_tempo(project.bpm);
    let fallback = crate::model::instrument::default_params_for_kind(
        crate::model::instrument::SampleKind::Piano,
    );
    let track_filter = track.effects.effective_lowpass();
    let secs_per_sample = 1.0 / sample_rate as f64;
    let beats_per_sec = project.beats_per_second();
    let (fx0, gain0) = project.track_mix_at(index, 0.0).unwrap_or_default();
    fx.jump_to(&fx0);
    let (mut cur_fx, mut target_gain) = (fx0, gain0);
    let mut gain = gain0;
    let gain_glide = 1.0 - (-1.0 / (0.01 * sample_rate)).exp();

    let mut scheduled: Vec<(u64, u8, f32, bool)> = Vec::new();
    let is_drum = track.is_drum();

    for clip in &track.clips {
        for note in &clip.notes {
            for trigger_beat in track.note_trigger_times(clip, note, project.total_beats) {
                let start =
                    (trigger_beat * project.beat_duration_secs() * sample_rate as f64) as u64;
                // The fader is applied after the effects (like a mixer), not to velocity.
                let vel = note.velocity * master_volume;
                scheduled.push((start, note.pitch, vel, false));
                if !is_drum {
                    let end = ((trigger_beat + note.duration_beats)
                        * project.beat_duration_secs()
                        * sample_rate as f64) as u64;
                    scheduled.push((end, note.pitch, vel, true));
                }
            }
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
            } else if is_drum || active.insert(pitch) {
                trigger_note(
                    &mut synth,
                    profiles,
                    &fallback,
                    pitch,
                    vel,
                    &track.instrument,
                    track_filter,
                );
            }
            event_idx += 1;
        }
        if i % AUTOMATION_STEP == 0 {
            let beat = i as f64 * secs_per_sample * beats_per_sec;
            if let Some((f, g)) = project.track_mix_at(index, beat) {
                cur_fx = f;
                // Mute is handled by skipping the track; keep only fader x crossfader.
                target_gain = g;
            }
        }
        gain += (target_gain - gain) * gain_glide;
        let dry = synth.next_sample();
        *out = fx.process_track(dry, &cur_fx) * gain;
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
    write_wav(path, &buffer, sample_rate)
}

/// Write mono samples (-1..1) as a 16-bit WAV file.
pub fn write_wav(path: &std::path::Path, samples: &[f32], sample_rate: f32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FxParam, Note, NoteClip, XfadeSide};

    /// A project with one piano track holding a long note over 8 beats at 120 bpm.
    fn one_note_project() -> Project {
        let mut p = Project::default();
        p.tracks.truncate(1);
        p.total_beats = 8.0;
        let mut clip = NoteClip::from_legacy_notes(1, Vec::new());
        clip.add_note(Note { pitch: 60, start_beat: 0.0, duration_beats: 8.0, velocity: 0.9 });
        p.tracks[0].add_clip(clip);
        p
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len().max(1) as f32).sqrt()
    }

    #[test]
    fn volume_automation_is_rendered() {
        let mut p = one_note_project();
        let lane = p.tracks[0].lane_mut(FxParam::Volume);
        lane.set_point(0.0, 1.0);
        lane.set_point(2.0, 1.0);
        lane.set_point(2.1, 0.0); // silence from beat 2.1 on
        let sr = 22_050.0;
        let buf = render_project_to_buffer(&p, &[], sr, 0.8, &MasterEffects::default());
        let beat = |b: f64| (b * 0.5 * sr as f64) as usize; // 0.5 s per beat
        let early = rms(&buf[beat(0.5)..beat(1.5)]);
        let late = rms(&buf[beat(3.0)..beat(6.0)]);
        assert!(early > 0.002, "note should be audible early ({early})");
        assert!(late < early * 0.02, "fader automation should silence it ({late} vs {early})");
    }

    #[test]
    fn crossfader_automation_is_rendered() {
        let mut p = one_note_project();
        p.tracks[0].dj.side = XfadeSide::A;
        p.dj.crossfader_lane.set_point(0.0, -1.0);
        p.dj.crossfader_lane.set_point(2.0, -1.0);
        p.dj.crossfader_lane.set_point(2.1, 1.0); // slam to B
        let sr = 22_050.0;
        let buf = render_project_to_buffer(&p, &[], sr, 0.8, &MasterEffects::default());
        let beat = |b: f64| (b * 0.5 * sr as f64) as usize;
        assert!(rms(&buf[beat(3.0)..beat(6.0)]) < rms(&buf[beat(0.5)..beat(1.5)]) * 0.02);
    }
}
