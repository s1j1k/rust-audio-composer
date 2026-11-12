//! Genre cheat-sheets for the Composition Assistant: tempo, arrangement, drums and
//! harmony, plus a few progressions that suit each style.
//!
//! Numbers are typical starting points, not rules. Drum & Bass tempo and arrangement
//! figures follow DnB Academy's "Drum and Bass BPM and Structure" guide
//! (https://app.dnbacademy.net/learn/drum-and-bass-bpm-and-structure).

use crate::model::instrument::Genre;

pub struct GenreGuide {
    pub genre: Genre,
    /// (low, typical, high) BPM.
    pub bpm: (f32, f32, f32),
    pub feel: &'static str,
    pub suggested_key: &'static str,
    pub structure: &'static [&'static str],
    pub drums: &'static [&'static str],
    pub harmony: &'static [&'static str],
    /// (name, chord symbols, description). Symbols are roman numerals for the chosen key.
    pub progressions: &'static [(&'static str, &'static [&'static str], &'static str)],
    pub source: Option<(&'static str, &'static str)>,
}

pub fn guide(genre: Genre) -> GenreGuide {
    match genre {
        Genre::DrumAndBass => GenreGuide {
            genre,
            bpm: (170.0, 174.0, 176.0),
            feel: "Fast two-step drums over a slow-moving bass and chords. Liquid sits around \
                   170–176, neurofunk and jump-up 172–176, rollers 172–175. Classic jungle was \
                   slower (about 160–170).",
            suggested_key: "Minor keys: F minor, G minor and A minor are common.",
            structure: &[
                "Work in 8, 16 and 32-bar blocks so DJs can mix the track.",
                "Intro 16–32 bars: drums and percussion, little or no sub bass.",
                "Build-up 8–16 bars: risers, rolls, a filter opening, then a short gap.",
                "Drop 32–64 bars: often four 16-bar phrases, each slightly different.",
                "Breakdown 16–32 bars: drums out or thinned; chords, pads, vocals.",
                "Drop 2: change something (new bass, new drums, or a halftime switch).",
                "Outro 16–32 bars: mirrors the intro so the next track can come in.",
                "Small changes every 4–8 bars, bigger ones every 16, new sections every 32.",
            ],
            drums: &[
                "Two-step: snare on beats 2 and 4 of each bar.",
                "Halftime: snare on beat 3 only. Same tempo, but it feels half as fast; good for the first half of drop 2.",
                "Use the last bar of each phrase for a fill or a drum drop-out.",
            ],
            harmony: &[
                "Write an 8–16 bar drop loop first, then arrange around it.",
                "Bass as call and response over 2 bars: a call on beat 1, an answer mid-way, a fill at the end of bar 2.",
                "Keep the sub bass out of the intro so it doesn't clash when DJs mix.",
                "Liquid: lush minor chords; neuro: one or two dark chords and a busy bass.",
            ],
            progressions: &[
                ("Liquid roller", &["i", "VI", "III", "VII"], "Warm, moving minor loop for liquid DnB."),
                ("Neuro tension", &["i", "VI"], "Two chords, so the bass carries the movement."),
                ("Soulful lift", &["i", "iv", "VII", "III"], "Rises toward the relative major."),
            ],
            source: Some((
                "DnB Academy: Drum and Bass BPM and Structure",
                "https://app.dnbacademy.net/learn/drum-and-bass-bpm-and-structure",
            )),
        },
        Genre::Trap => GenreGuide {
            genre,
            bpm: (130.0, 140.0, 150.0),
            feel: "Usually counted around 140 with a halftime feel (it bobs like 70). Sparse, \
                   dark melodies over long 808s.",
            suggested_key: "Minor keys: C minor, F minor and A minor work well for 808s.",
            structure: &[
                "Intro 4–8 bars (melody alone), then hook and verse in 8 or 16-bar blocks.",
                "A common layout: intro 8, hook 8, verse 16, hook 8, verse 16, hook 8, outro.",
                "Drop the drums for a bar before the hook to make it hit harder.",
            ],
            drums: &[
                "Snare or clap on beat 3 of each bar (the halftime backbeat).",
                "Hi-hats in 1/8 or 1/16 notes, with fast rolls (1/32, triplets) as fills.",
                "808 follows the kick and holds long notes; glide between pitches.",
            ],
            harmony: &[
                "Two or three chords, often minor, looped for the whole beat.",
                "Natural minor with a flat 2 (Phrygian) gives the darkest sound.",
                "Keep the melody in a high register so it doesn't fight the 808.",
            ],
            progressions: &[
                ("Dark loop", &["i", "VI"], "Two-chord minor loop."),
                ("Moody four", &["i", "VII", "VI", "VII"], "Rocks between the same three chords."),
                ("Sad bounce", &["i", "iv", "VI", "V"], "The major V adds a pull back home."),
            ],
            source: None,
        },
        Genre::HipHop => GenreGuide {
            genre,
            bpm: (80.0, 90.0, 100.0),
            feel: "Laid-back swing. Boom bap sits around 85–95 with a slightly late, \
                   shuffled feel.",
            suggested_key: "Minor or jazzy keys: D minor, E♭ major, A minor.",
            structure: &[
                "Verses of 16 bars and hooks of 8 bars are the usual blocks.",
                "A 4 or 8-bar loop (often a sample or keys) repeats under everything.",
                "Change the drums or drop the bass for the hook to set it apart.",
            ],
            drums: &[
                "Kick on beat 1 (and around the 'and' of 2); snare on 2 and 4.",
                "Swing the hi-hats (push every second 1/16 note slightly late).",
                "Lower velocities on ghost snares make the groove feel played.",
            ],
            harmony: &[
                "Jazz-flavoured chords (ii–V–I, minor 7ths) suit Rhodes and keys.",
                "Loop two to four chords; let the bass walk between them.",
            ],
            progressions: &[
                ("Jazzy ii-V-I", &["ii", "V", "I"], "The classic jazz turnaround."),
                ("Soul loop", &["I", "vi", "ii", "V"], "Smooth, warm loop for Rhodes."),
                ("Minor head-nod", &["i", "iv"], "Two minor chords; simple and heavy."),
            ],
            source: None,
        },
        Genre::Garage => GenreGuide {
            genre,
            bpm: (130.0, 132.0, 135.0),
            feel: "Shuffled, skippy 2-step drums with organ or chord stabs and a rolling bass.",
            suggested_key: "Minor keys with bright stabs: F minor, G minor, C minor.",
            structure: &[
                "8 and 16-bar blocks, with DJ-friendly intros and outros (16 bars).",
                "Intro 16, verse 16, chorus 16, break 8, chorus 16, outro 16.",
            ],
            drums: &[
                "2-step: kick on beat 1 and just after beat 3 (it skips the 4/4 kick).",
                "Snare or clap on 2 and 4; shuffle the hats with swing.",
                "Add a shaker or a short percussion fill at the end of each phrase.",
            ],
            harmony: &[
                "Minor 7th and 9th chord stabs, played short and on the off-beat.",
                "Bass follows the chord roots with octave jumps.",
            ],
            progressions: &[
                ("Garage stabs", &["i", "iv", "VII", "III"], "Bright and bouncy minor loop."),
                ("Two-step soul", &["vi", "IV", "I", "V"], "Major-key, vocal-friendly loop."),
            ],
            source: None,
        },
        Genre::Classical => GenreGuide {
            genre,
            bpm: (60.0, 90.0, 120.0),
            feel: "Tempo follows the character: Adagio ~60–75, Andante ~76–100, \
                   Allegro ~120–150.",
            suggested_key: "Any key. C, G, D, F major and A, D minor are friendly to start with.",
            structure: &[
                "Phrases of 4 and 8 bars that end on a cadence.",
                "Binary (A–B) and ternary (A–B–A) forms are good first structures.",
                "Repeat a melody, then vary it (higher, in the relative minor, or decorated).",
            ],
            drums: &[
                "No drums: rhythm comes from the bass line and accompaniment patterns.",
                "Try an Alberti bass (low–high–middle–high) under a melody.",
            ],
            harmony: &[
                "End phrases on V (a half cadence) or V–I (a perfect cadence).",
                "Move to the relative minor or the dominant key for contrast.",
            ],
            progressions: &[
                ("Perfect cadence", &["I", "IV", "V", "I"], "The most basic complete phrase."),
                ("Circle progression", &["I", "vi", "ii", "V", "I"], "Moves round the circle of fifths."),
                ("Pachelbel-style", &["I", "V", "vi", "iii", "IV", "I", "IV", "V"], "A long, flowing ground bass."),
            ],
            source: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::chords::{resolve_chord, Key, KeyMode};

    #[test]
    fn every_genre_progression_resolves_in_any_key() {
        for &g in Genre::all() {
            let gd = guide(g);
            assert!(gd.bpm.0 <= gd.bpm.1 && gd.bpm.1 <= gd.bpm.2);
            for (name, chords, _) in gd.progressions {
                for root in 0..12 {
                    for mode in [KeyMode::Major, KeyMode::Minor] {
                        let key = Key { root, mode };
                        for c in chords.iter() {
                            assert!(resolve_chord(c, &key).is_some(), "{name}: {c}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn dnb_tempo_matches_source() {
        assert_eq!(guide(Genre::DrumAndBass).bpm, (170.0, 174.0, 176.0));
    }
}
