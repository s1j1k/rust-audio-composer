use super::theory::{MAJOR_SCALE, MINOR_SCALE, NOTE_NAMES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyMode {
    Major,
    Minor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub root: u8,
    pub mode: KeyMode,
}

impl Key {
    pub fn name(&self) -> String {
        let root_name = NOTE_NAMES[self.root as usize];
        match self.mode {
            KeyMode::Major => root_name.to_string(),
            KeyMode::Minor => format!("{}m", root_name),
        }
    }

    pub fn scale_pitches(&self) -> Vec<u8> {
        let intervals = match self.mode {
            KeyMode::Major => &MAJOR_SCALE[..],
            KeyMode::Minor => &MINOR_SCALE[..],
        };
        intervals.iter().map(|i| (self.root + i) % 12).collect()
    }

    pub fn contains_pitch_class(&self, pc: u8) -> bool {
        self.scale_pitches().contains(&(pc % 12))
    }
}

#[derive(Clone, Debug)]
pub struct ChordProgression {
    pub name: String,
    pub description: String,
    pub chords: Vec<String>,
}

pub fn detect_key(pitches: &[u8]) -> Option<Key> {
    if pitches.is_empty() {
        return None;
    }

    let mut best: Option<(Key, usize)> = None;

    for root in 0..12u8 {
        for mode in [KeyMode::Major, KeyMode::Minor] {
            let key = Key { root, mode };
            let scale = key.scale_pitches();
            let score = pitches
                .iter()
                .map(|p| scale.contains(&(p % 12)))
                .filter(|&in_scale| in_scale)
                .count();
            let is_better = best.as_ref().map(|(_, s)| score > *s).unwrap_or(true);
            if is_better && score > 0 {
                best = Some((key, score));
            }
        }
    }

    best.map(|(k, _)| k)
}

pub fn suggest_progressions(key: &Key) -> Vec<ChordProgression> {
    let r = NOTE_NAMES[key.root as usize];
    match key.mode {
        KeyMode::Major => vec![
            ChordProgression {
                name: "Pop".into(),
                description: "The most common progression in modern pop.".into(),
                chords: vec![r.into(), "IV".into(), "V".into(), "vi".into()],
            },
            ChordProgression {
                name: "Axis of Awesome".into(),
                description: "I-V-vi-IV — used in countless hit songs.".into(),
                chords: vec![r.into(), "V".into(), "vi".into(), "IV".into()],
            },
            ChordProgression {
                name: "50s Progression".into(),
                description: "Classic doo-wop and early rock.".into(),
                chords: vec![r.into(), "vi".into(), "IV".into(), "V".into()],
            },
            ChordProgression {
                name: "Jazz ii-V-I".into(),
                description: "Foundation of jazz harmony.".into(),
                chords: vec!["ii".into(), "V".into(), r.into()],
            },
        ],
        KeyMode::Minor => vec![
            ChordProgression {
                name: "Natural Minor".into(),
                description: "i-VI-III-VII — common in rock and EDM.".into(),
                chords: vec!["i".into(), "VI".into(), "III".into(), "VII".into()],
            },
            ChordProgression {
                name: "Andalusian Cadence".into(),
                description: "i-VII-VI-V — flamenco and metal.".into(),
                chords: vec!["i".into(), "VII".into(), "VI".into(), "V".into()],
            },
            ChordProgression {
                name: "Minor Pop".into(),
                description: "i-VI-III-VII variant.".into(),
                chords: vec!["i".into(), "VI".into(), "IV".into(), "V".into()],
            },
        ],
    }
}

pub fn roman_to_chord(roman: &str, key: &Key) -> String {
    let major_map = [("I", 0), ("ii", 2), ("iii", 4), ("IV", 5), ("V", 7), ("vi", 9), ("vii°", 11)];
    let minor_map = [("i", 0), ("ii°", 2), ("III", 3), ("iv", 5), ("v", 7), ("VI", 8), ("VII", 10)];

    let map: &[(&str, u8)] = match key.mode {
        KeyMode::Major => &major_map,
        KeyMode::Minor => &minor_map,
    };

    for &(label, interval) in map {
        if label == roman {
            let pc = (key.root + interval) % 12;
            return NOTE_NAMES[pc as usize].to_string();
        }
    }

    NOTE_NAMES[key.root as usize].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_c_major() {
        let pitches = vec![60, 62, 64, 65, 67, 69, 71, 72];
        let key = detect_key(&pitches).unwrap();
        assert_eq!(key.root, 0);
        assert_eq!(key.mode, KeyMode::Major);
    }
}
