pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

pub const MAJOR_SCALE: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
pub const MINOR_SCALE: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];

#[derive(Clone, Debug, PartialEq)]
pub struct CircleEntry {
    pub major: &'static str,
    pub minor: &'static str,
    pub angle_deg: f32,
}

pub fn circle_of_fifths() -> Vec<CircleEntry> {
    vec![
        CircleEntry { major: "C", minor: "Am", angle_deg: 0.0 },
        CircleEntry { major: "G", minor: "Em", angle_deg: 30.0 },
        CircleEntry { major: "D", minor: "Bm", angle_deg: 60.0 },
        CircleEntry { major: "A", minor: "F#m", angle_deg: 90.0 },
        CircleEntry { major: "E", minor: "C#m", angle_deg: 120.0 },
        CircleEntry { major: "B", minor: "G#m", angle_deg: 150.0 },
        CircleEntry { major: "F#", minor: "D#m", angle_deg: 180.0 },
        CircleEntry { major: "Db", minor: "Bbm", angle_deg: 210.0 },
        CircleEntry { major: "Ab", minor: "Fm", angle_deg: 240.0 },
        CircleEntry { major: "Eb", minor: "Cm", angle_deg: 270.0 },
        CircleEntry { major: "Bb", minor: "Gm", angle_deg: 300.0 },
        CircleEntry { major: "F", minor: "Dm", angle_deg: 330.0 },
    ]
}

pub fn pitch_to_name(pitch: u8) -> String {
    let octave = (pitch as i32 / 12) - 1;
    let name = NOTE_NAMES[(pitch % 12) as usize];
    format!("{}{}", name, octave)
}

pub fn note_name(pitch: u8) -> &'static str {
    NOTE_NAMES[(pitch % 12) as usize]
}

pub fn name_to_pitch(name: &str, default_octave: i32) -> Option<u8> {
    let name = name.trim();
    let (note_part, octave) = if let Some(last) = name.chars().last() {
        if last.is_ascii_digit() {
            let oct: i32 = last.to_digit(10)? as i32;
            (&name[..name.len() - 1], oct)
        } else {
            (name, default_octave)
        }
    } else {
        (name, default_octave)
    };

    let pc = NOTE_NAMES.iter().position(|&n| n.eq_ignore_ascii_case(note_part))?;
    Some((octave + 1) as u8 * 12 + pc as u8)
}

pub fn midi_to_frequency(pitch: u8) -> f32 {
    440.0 * 2.0_f32.powf((pitch as f32 - 69.0) / 12.0)
}

pub fn frequency_to_midi(freq: f32) -> u8 {
    ((12.0 * (freq / 440.0).log2()) + 69.0).round().clamp(0.0, 127.0) as u8
}

pub fn is_black_key(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_frequency_roundtrip() {
        let pitch = 60;
        let freq = midi_to_frequency(pitch);
        assert_eq!(frequency_to_midi(freq), pitch);
    }
}
