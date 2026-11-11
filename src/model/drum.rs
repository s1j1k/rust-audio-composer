use serde::{Deserialize, Serialize};

/// General MIDI-style drum map pitches used in the sequencer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrumKind {
    Kick,
    Snare,
    ClosedHat,
    OpenHat,
    LowTom,
    MidTom,
    HighTom,
    Clap,
    Cymbal,
    Shaker,
}

impl DrumKind {
    pub fn all() -> &'static [DrumKind] {
        &[
            Self::Kick,
            Self::Snare,
            Self::ClosedHat,
            Self::OpenHat,
            Self::LowTom,
            Self::MidTom,
            Self::HighTom,
            Self::Clap,
            Self::Cymbal,
            Self::Shaker,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Kick => "Kick",
            Self::Snare => "Snare",
            Self::ClosedHat => "Closed Hi-Hat",
            Self::OpenHat => "Open Hi-Hat",
            Self::LowTom => "Low Tom",
            Self::MidTom => "Mid Tom",
            Self::HighTom => "High Tom",
            Self::Clap => "Clap",
            Self::Cymbal => "Cymbal",
            Self::Shaker => "Shaker",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Kick => "🥾",
            Self::Snare => "🪘",
            Self::ClosedHat => "🎩",
            Self::OpenHat => "💫",
            Self::LowTom => "🟤",
            Self::MidTom => "🟠",
            Self::HighTom => "🟡",
            Self::Clap => "👏",
            Self::Cymbal => "🔔",
            Self::Shaker => "🫨",
        }
    }

    pub fn midi_pitch(&self) -> u8 {
        match self {
            Self::Kick => 36,
            Self::Snare => 38,
            Self::ClosedHat => 42,
            Self::OpenHat => 46,
            Self::LowTom => 41,
            Self::MidTom => 43,
            Self::HighTom => 45,
            Self::Clap => 39,
            Self::Cymbal => 49,
            Self::Shaker => 54,
        }
    }

    pub fn from_midi_pitch(pitch: u8) -> Option<Self> {
        Self::all().iter().copied().find(|d| d.midi_pitch() == pitch)
    }
}

pub const SEQUENCER_STEPS: u8 = 16;

pub fn step_to_beat(step: u8, beats_per_bar: f64, steps_per_bar: u8) -> f64 {
    (step as f64 / steps_per_bar as f64) * beats_per_bar
}

pub fn beat_to_step(beat: f64, beats_per_bar: f64, steps_per_bar: u8) -> u8 {
    let step = ((beat / beats_per_bar) * steps_per_bar as f64).round() as i32;
    step.clamp(0, steps_per_bar as i32 - 1) as u8
}
