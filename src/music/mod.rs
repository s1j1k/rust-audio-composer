pub mod chords;
pub mod theory;

pub use chords::{detect_key, suggest_progressions, roman_to_chord, ChordProgression, Key, KeyMode};
pub use theory::{circle_of_fifths, note_name, pitch_to_name, CircleEntry, NOTE_NAMES};
