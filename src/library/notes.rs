//! Working out which note a sample is: from its file name ("Cello_A3.wav",
//! "piano-Db4-ff.ogg", "harp_60.wav"), or failing that by measuring its pitch.

/// Octave numbering convention: scientific pitch puts middle C (MIDI 60) at C4. Some
/// sample packs (Yamaha / many DAWs) call it C3; the import screen has an octave shift
/// for those.
pub const MIDDLE_C_OCTAVE: i32 = 4;

/// Note from a file name (without the extension). Prefers a note name like "A#3",
/// "Bb2", "Cs4", "C-1"; falls back to a bare MIDI number between 21 and 108.
pub fn note_from_name(stem: &str) -> Option<u8> {
    let tokens: Vec<&str> = stem
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '#' || c == '-'))
        .filter(|t| !t.is_empty())
        .collect();
    // Whole tokens first ("A3" in "Cello_A3_ff"), last one wins (names often end with
    // the note); then notes glued to other text ("PianoC4").
    for t in tokens.iter().rev() {
        if let Some((m, used)) = parse_note_at(t, 0) {
            if used == t.len() {
                return Some(m);
            }
        }
    }
    for t in tokens.iter().rev() {
        let chars: Vec<char> = t.chars().collect();
        for i in (0..chars.len()).rev() {
            // A note glued to a word must start at a capital or follow a non-letter,
            // so "Grand2" doesn't read as D2.
            let ok_start = i == 0 || !chars[i - 1].is_ascii_alphabetic() || chars[i].is_ascii_uppercase();
            if !ok_start {
                continue;
            }
            let byte = t.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0);
            if let Some((m, used)) = parse_note_at(t, byte) {
                let end = byte + used;
                let followed_ok = t[end..].chars().next().map_or(true, |c| !c.is_ascii_digit());
                if followed_ok {
                    return Some(m);
                }
            }
        }
    }
    for t in tokens.iter().rev() {
        if let Ok(n) = t.parse::<u32>() {
            if (21..=108).contains(&n) && t.len() <= 3 {
                return Some(n as u8);
            }
        }
    }
    None
}

/// Parse a note name starting at byte `at`: letter, optional accidental, octave.
/// Returns the MIDI note and how many bytes were used.
fn parse_note_at(s: &str, at: usize) -> Option<(u8, usize)> {
    let b = s.as_bytes();
    let letter = (*b.get(at)? as char).to_ascii_uppercase();
    let base = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut i = at + 1;
    let mut acc = 0i32;
    match b.get(i).map(|&c| c as char) {
        Some('#') | Some('s') | Some('S') => {
            acc = 1;
            i += 1;
        }
        Some('b') => {
            acc = -1;
            i += 1;
        }
        _ => {}
    }
    let neg = b.get(i) == Some(&b'-');
    if neg {
        i += 1;
    }
    let digit = (*b.get(i)? as char).to_digit(10)? as i32;
    i += 1;
    let octave = if neg { -digit } else { digit };
    if b.get(i).map_or(false, |c| c.is_ascii_digit()) {
        return None; // "C10", "A440": not a note name
    }
    // Scientific pitch: C4 = 60, C-1 = 0.
    let midi = (octave + 1 + (4 - MIDDLE_C_OCTAVE)) * 12 + base + acc;
    (0..=127).contains(&midi).then_some((midi as u8, i - at))
}

/// Estimate the pitch of a recorded note (YIN on a window just after the attack).
/// Returns the nearest MIDI note, or None if no clear pitch is found.
pub fn detect_midi(samples: &[f32], rate: f32) -> Option<u8> {
    let f0 = detect_frequency(samples, rate)?;
    let midi = 69.0 + 12.0 * (f0 / 440.0).log2();
    (0.0..=127.0).contains(&midi).then(|| midi.round() as u8)
}

pub fn detect_frequency(samples: &[f32], rate: f32) -> Option<f32> {
    if samples.is_empty() || rate <= 0.0 {
        return None;
    }
    // Skip the attack transient (to just after the loudest point, +40 ms).
    let peak_at = samples
        .iter()
        .enumerate()
        .take((rate * 0.5) as usize)
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let start = (peak_at + (rate * 0.04) as usize).min(samples.len());
    let min_f = 30.0_f32;
    let max_f = 2000.0_f32;
    let max_tau = (rate / min_f) as usize;
    let min_tau = (rate / max_f).max(2.0) as usize;
    let window = max_tau.max(1024);
    if samples.len() < start + window + max_tau {
        // Short sample: analyse from the beginning instead.
        if samples.len() < window + max_tau {
            return None;
        }
    }
    let start = if samples.len() >= start + window + max_tau { start } else { 0 };
    let x = &samples[start..start + window + max_tau];

    // YIN difference function and cumulative mean normalised difference.
    let mut d = vec![0.0_f32; max_tau + 1];
    for tau in 1..=max_tau {
        let mut sum = 0.0;
        for j in 0..window {
            let diff = x[j] - x[j + tau];
            sum += diff * diff;
        }
        d[tau] = sum;
    }
    let mut cmnd = vec![1.0_f32; max_tau + 1];
    let mut running = 0.0;
    for tau in 1..=max_tau {
        running += d[tau];
        cmnd[tau] = if running > 0.0 { d[tau] * tau as f32 / running } else { 1.0 };
    }
    let threshold = 0.15;
    let mut tau = None;
    let mut t = min_tau;
    while t <= max_tau {
        if cmnd[t] < threshold {
            while t + 1 <= max_tau && cmnd[t + 1] < cmnd[t] {
                t += 1;
            }
            tau = Some(t);
            break;
        }
        t += 1;
    }
    let tau = tau?;
    // Parabolic interpolation for a finer period.
    let better = if tau > 1 && tau < max_tau {
        let (a, b, c) = (cmnd[tau - 1], cmnd[tau], cmnd[tau + 1]);
        let denom = a - 2.0 * b + c;
        if denom.abs() > 1e-9 { tau as f32 + 0.5 * (a - c) / denom } else { tau as f32 }
    } else {
        tau as f32
    };
    Some(rate / better)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names_in_file_names() {
        assert_eq!(note_from_name("Cello_A3"), Some(57));
        assert_eq!(note_from_name("piano-Db4-ff"), Some(61));
        assert_eq!(note_from_name("Flute C#5 legato"), Some(73));
        assert_eq!(note_from_name("harp_Cs2"), Some(37));
        assert_eq!(note_from_name("PianoC4"), Some(60));
        assert_eq!(note_from_name("bass_E1"), Some(28));
        assert_eq!(note_from_name("sub C-1"), Some(0));
        assert_eq!(note_from_name("marimba_60"), Some(60));
        assert_eq!(note_from_name("GrandPiano take2"), None);
        assert_eq!(note_from_name("A440 reference"), None);
    }

    #[test]
    fn detects_pitch_of_a_harmonic_tone() {
        let rate = 44100.0;
        for &(f, midi) in &[(110.0_f32, 45u8), (261.63, 60), (440.0, 69), (987.77, 83)] {
            let s: Vec<f32> = (0..(rate * 0.6) as usize)
                .map(|i| {
                    let t = i as f32 / rate;
                    let env = (-t * 3.0).exp();
                    env * ((std::f32::consts::TAU * f * t).sin()
                        + 0.5 * (std::f32::consts::TAU * 2.0 * f * t).sin()
                        + 0.25 * (std::f32::consts::TAU * 3.0 * f * t).sin())
                })
                .collect();
            assert_eq!(detect_midi(&s, rate), Some(midi), "{f} Hz");
        }
    }

    #[test]
    fn noise_has_no_pitch() {
        let mut x: u32 = 12345;
        let s: Vec<f32> = (0..30000)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 8) as f32 / (1u32 << 24) as f32 - 0.5
            })
            .collect();
        assert_eq!(detect_midi(&s, 44100.0), None);
    }
}
