pub mod chords;
pub mod genre_guide;
pub mod theory;

pub use chords::{
    detect_key, resolve_chord, roman_to_chord, suggest_progressions, ChordProgression, Key, KeyMode,
    ResolvedChord,
};
pub use theory::{circle_of_fifths, note_name, pitch_to_name, CircleEntry, NOTE_NAMES};
