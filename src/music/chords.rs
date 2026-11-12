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
    match key.mode {
        KeyMode::Major => vec![
            ChordProgression {
                name: "Pop".into(),
                description: "The most common progression in modern pop.".into(),
                chords: vec!["I".into(), "IV".into(), "V".into(), "vi".into()],
            },
            ChordProgression {
                name: "Axis of Awesome".into(),
                description: "I-V-vi-IV — used in countless hit songs.".into(),
                chords: vec!["I".into(), "V".into(), "vi".into(), "IV".into()],
            },
            ChordProgression {
                name: "50s Progression".into(),
                description: "Classic doo-wop and early rock.".into(),
                chords: vec!["I".into(), "vi".into(), "IV".into(), "V".into()],
            },
            ChordProgression {
                name: "Jazz ii-V-I".into(),
                description: "Foundation of jazz harmony.".into(),
                chords: vec!["ii".into(), "V".into(), "I".into()],
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
                description: "i-VI-IV-V: a brighter minor loop (the IV and V are major).".into(),
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

/// A chord symbol resolved in a key: root pitch class and intervals above it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedChord {
    pub root: u8,
    pub intervals: Vec<u8>,
}

impl ResolvedChord {
    /// Name like "Am", "F", "B°".
    pub fn name(&self) -> String {
        let root = NOTE_NAMES[self.root as usize];
        match self.intervals.as_slice() {
            [0, 3, 6] => format!("{root}°"),
            [0, 3, 7] => format!("{root}m"),
            _ => root.to_string(),
        }
    }

    /// MIDI pitches in root position, kept around middle C (roots from F#3 to F4) so
    /// consecutive chords sit in the same register.
    pub fn voicing(&self) -> Vec<u8> {
        let base = if self.root >= 6 { 48 + self.root } else { 60 + self.root };
        self.intervals.iter().map(|i| base + i).collect()
    }
}

/// Resolve a chord symbol in `key`.
///
/// Roman numerals pick the scale degree of the key (in minor, the natural minor scale);
/// their case sets the quality: `IV` major, `iv` minor, `vii°` / `ii°` diminished. So `V`
/// in A minor is E major (the "harmonic minor" dominant) while `v` is E minor.
/// A plain note name ("C", "F#") is a major triad on that note.
pub fn resolve_chord(symbol: &str, key: &Key) -> Option<ResolvedChord> {
    const MAJOR: [u8; 3] = [0, 4, 7];
    const MINOR: [u8; 3] = [0, 3, 7];
    const DIM: [u8; 3] = [0, 3, 6];

    if let Some(pc) = NOTE_NAMES.iter().position(|&n| n == symbol) {
        return Some(ResolvedChord { root: pc as u8, intervals: MAJOR.to_vec() });
    }

    let dim = symbol.ends_with('°');
    let numeral = symbol.trim_end_matches('°');
    let degree = match numeral.to_ascii_uppercase().as_str() {
        "I" => 0,
        "II" => 1,
        "III" => 2,
        "IV" => 3,
        "V" => 4,
        "VI" => 5,
        "VII" => 6,
        _ => return None,
    };
    let scale = match key.mode {
        KeyMode::Major => &MAJOR_SCALE,
        KeyMode::Minor => &MINOR_SCALE,
    };
    let root = (key.root + scale[degree]) % 12;
    let upper = numeral.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(true);
    let intervals = if dim {
        DIM
    } else if upper {
        MAJOR
    } else {
        MINOR
    };
    Some(ResolvedChord { root, intervals: intervals.to_vec() })
}

/// A chord recognised from notes that sound together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedChord {
    /// Name such as "Am", "F", "G7", "C/E" (slash = the lowest note isn't the root).
    pub name: String,
    pub root: u8,
    /// Pitch class of the lowest sounding note: the natural bass note.
    pub bass: u8,
    /// Chord tones as pitch classes, root first.
    pub tones: Vec<u8>,
}

impl DetectedChord {
    /// "A C E": the chord tones as note names.
    pub fn tone_names(&self) -> String {
        self.tones
            .iter()
            .map(|&pc| NOTE_NAMES[pc as usize])
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Name the chord formed by `pitches` (MIDI notes sounding at the same time).
/// Needs at least three different notes; returns None when they don't form a
/// recognisable triad, sus chord or seventh chord. When the same notes could be read two
/// ways (C6 = Am7), the reading whose root is the lowest note wins.
pub fn identify_chord(pitches: &[u8]) -> Option<DetectedChord> {
    // (intervals above the root, suffix)
    const SHAPES: &[(&[u8], &str)] = &[
        (&[0, 4, 7], ""),
        (&[0, 3, 7], "m"),
        (&[0, 3, 6], "°"),
        (&[0, 4, 8], "+"),
        (&[0, 2, 7], "sus2"),
        (&[0, 5, 7], "sus4"),
        (&[0, 4, 7, 10], "7"),
        (&[0, 4, 7, 11], "maj7"),
        (&[0, 3, 7, 10], "m7"),
        (&[0, 3, 6, 10], "m7♭5"),
        (&[0, 3, 6, 9], "°7"),
        (&[0, 4, 7, 9], "6"),
        (&[0, 3, 7, 9], "m6"),
        (&[0, 4, 7, 2], "add9"),
    ];
    let bass = pitches.iter().min()? % 12;
    let mut pcs: Vec<u8> = pitches.iter().map(|p| p % 12).collect();
    pcs.sort_unstable();
    pcs.dedup();
    if pcs.len() < 3 {
        return None;
    }

    let mut best: Option<(u8, &str, &[u8])> = None;
    for &root in &pcs {
        let mut rel: Vec<u8> = pcs.iter().map(|&pc| (pc + 12 - root) % 12).collect();
        rel.sort_unstable();
        for &(shape, suffix) in SHAPES {
            let mut s = shape.to_vec();
            s.sort_unstable();
            if s == rel {
                let better = match best {
                    None => true,
                    // Prefer the reading rooted on the bass note.
                    Some((r, _, _)) => r != bass && root == bass,
                };
                if better {
                    best = Some((root, suffix, shape));
                }
            }
        }
    }

    let (root, suffix, shape) = best?;
    let mut name = format!("{}{}", NOTE_NAMES[root as usize], suffix);
    if bass != root {
        name.push('/');
        name.push_str(NOTE_NAMES[bass as usize]);
    }
    Some(DetectedChord {
        name,
        root,
        bass,
        tones: shape.iter().map(|i| (root + i) % 12).collect(),
    })
}

/// Chords over time from `(start, end, pitch)` note spans (in beats). Each change in
/// which notes are sounding starts a new segment; consecutive segments with the same
/// chord are merged. Segments where no chord is recognised are left out.
pub fn chord_segments(notes: &[(f64, f64, u8)]) -> Vec<(f64, f64, DetectedChord)> {
    let mut edges: Vec<f64> = notes.iter().flat_map(|&(s, e, _)| [s, e]).collect();
    edges.sort_by(|a, b| a.total_cmp(b));
    edges.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let mut out: Vec<(f64, f64, DetectedChord)> = Vec::new();
    for w in edges.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b - a < 1e-6 {
            continue;
        }
        let mid = (a + b) * 0.5;
        let sounding: Vec<u8> =
            notes.iter().filter(|&&(s, e, _)| s <= mid && mid < e).map(|&(_, _, p)| p).collect();
        let Some(chord) = identify_chord(&sounding) else { continue };
        match out.last_mut() {
            Some(last) if last.2 == chord && (last.1 - a).abs() < 1e-6 => last.1 = b,
            _ => out.push((a, b, chord)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_common_chords_and_inversions() {
        let name = |p: &[u8]| identify_chord(p).map(|c| c.name);
        assert_eq!(name(&[57, 60, 64]).as_deref(), Some("Am"));
        assert_eq!(name(&[60, 64, 67]).as_deref(), Some("C"));
        assert_eq!(name(&[52, 60, 67]).as_deref(), Some("C/E"));
        assert_eq!(name(&[55, 59, 62, 65]).as_deref(), Some("G7"));
        assert_eq!(name(&[59, 62, 65]).as_deref(), Some("B°"));
        // Bass A under C-E-G reads as Am7, bass C reads as C6.
        assert_eq!(name(&[45, 60, 64, 67]).as_deref(), Some("Am7"));
        assert_eq!(name(&[48, 57, 64, 67]).as_deref(), Some("C6"));
        assert_eq!(name(&[60, 64]), None);
        let am = identify_chord(&[57, 60, 64]).unwrap();
        assert_eq!(am.tone_names(), "A C E");
        assert_eq!(am.bass, 9);
    }

    #[test]
    fn segments_follow_chord_changes() {
        // Am for a bar, then F for a bar (with a held bass note over both halves of F).
        let notes = [
            (0.0, 4.0, 57), (0.0, 4.0, 60), (0.0, 4.0, 64),
            (4.0, 8.0, 53), (4.0, 8.0, 57), (4.0, 6.0, 60), (6.0, 8.0, 60),
        ];
        let segs: Vec<(f64, f64, String)> =
            chord_segments(&notes).into_iter().map(|(a, b, c)| (a, b, c.name)).collect();
        assert_eq!(segs, vec![(0.0, 4.0, "Am".to_string()), (4.0, 8.0, "F".to_string())]);
    }

    fn names(symbols: &[&str], key: &Key) -> Vec<String> {
        symbols.iter().map(|s| resolve_chord(s, key).unwrap().name()).collect()
    }

    #[test]
    fn minor_key_chords_are_minor_where_they_should_be() {
        let a_minor = Key { root: 9, mode: KeyMode::Minor };
        assert_eq!(names(&["i", "VI", "III", "VII"], &a_minor), ["Am", "F", "C", "G"]);
        assert_eq!(names(&["iv", "v", "V", "ii°"], &a_minor), ["Dm", "Em", "E", "B°"]);
    }

    #[test]
    fn major_key_chords() {
        let c = Key { root: 0, mode: KeyMode::Major };
        assert_eq!(names(&["I", "ii", "iii", "IV", "V", "vi", "vii°"], &c), ["C", "Dm", "Em", "F", "G", "Am", "B°"]);
        assert_eq!(names(&["G"], &c), ["G"]);
    }

    #[test]
    fn every_suggested_progression_resolves() {
        for root in 0..12 {
            for mode in [KeyMode::Major, KeyMode::Minor] {
                let key = Key { root, mode };
                for p in suggest_progressions(&key) {
                    for c in &p.chords {
                        assert!(resolve_chord(c, &key).is_some(), "{c} in {}", key.name());
                    }
                }
            }
        }
    }

    #[test]
    fn voicing_stays_near_middle_c() {
        for root in 0..12 {
            let v = ResolvedChord { root, intervals: vec![0, 4, 7] }.voicing();
            assert!(v[0] >= 54 && v[0] <= 65, "{v:?}");
        }
    }

    #[test]
    fn test_detect_c_major() {
        let pitches = vec![60, 62, 64, 65, 67, 69, 71, 72];
        let key = detect_key(&pitches).unwrap();
        assert_eq!(key.root, 0);
        assert_eq!(key.mode, KeyMode::Major);
    }
}
