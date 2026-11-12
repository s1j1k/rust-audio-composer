//! Track icons from the Phosphor icon set (https://phosphoricons.com, MIT licence),
//! bundled as a font by the `egui-phosphor` crate. The same font is served to the phone
//! remote, so a track looks the same on both screens.

use super::instrument::InstrumentId;
use egui_phosphor::regular as ph;
use serde::{Deserialize, Serialize};

macro_rules! icons {
    ($( $variant:ident => $key:literal, $glyph:expr, $label:literal; )*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum TrackIcon { $( $variant, )* }

        impl TrackIcon {
            pub fn all() -> &'static [TrackIcon] { &[ $( TrackIcon::$variant, )* ] }
            pub fn key(self) -> &'static str { match self { $( TrackIcon::$variant => $key, )* } }
            /// The Phosphor glyph (one private-use codepoint).
            pub fn glyph(self) -> &'static str { match self { $( TrackIcon::$variant => $glyph, )* } }
            pub fn label(self) -> &'static str { match self { $( TrackIcon::$variant => $label, )* } }
        }
    };
}

icons! {
    Piano => "piano", ph::PIANO_KEYS, "Keys";
    Guitar => "guitar", ph::GUITAR, "Guitar";
    Microphone => "microphone", ph::MICROPHONE_STAGE, "Vocals";
    // Phosphor has no drum: the desktop paints this one with `ui::drum_icons` and the
    // phone shows its own drum emoji. The glyph is only a fallback.
    Drums => "drums", ph::METRONOME, "Drums";
    Vinyl => "vinyl", ph::VINYL_RECORD, "Vinyl";
    Metronome => "metronome", ph::METRONOME, "Metronome";
    Clap => "clap", ph::HANDS_CLAPPING, "Claps";
    Waveform => "waveform", ph::WAVEFORM, "Waveform";
    Sine => "sine", ph::WAVE_SINE, "Sine";
    Saw => "saw", ph::WAVE_SAWTOOTH, "Saw";
    Square => "square", ph::WAVE_SQUARE, "Square";
    Notes => "notes", ph::MUSIC_NOTES, "Notes";
    Note => "note", ph::MUSIC_NOTE, "Note";
    Headphones => "headphones", ph::HEADPHONES, "Headphones";
    Speaker => "speaker", ph::SPEAKER_HIFI, "Speaker";
    Radio => "radio", ph::RADIO, "Radio";
    Megaphone => "megaphone", ph::MEGAPHONE, "Horns";
    Bell => "bell", ph::BELL, "Bell";
    Equalizer => "equalizer", ph::EQUALIZER, "EQ";
    Cassette => "cassette", ph::CASSETTE_TAPE, "Tape";
    Disco => "disco", ph::DISCO_BALL, "Disco";
    Sparkle => "sparkle", ph::SPARKLE, "Sparkle";
    Star => "star", ph::STAR, "Star";
    Lightning => "lightning", ph::LIGHTNING, "Lightning";
    Fire => "fire", ph::FIRE, "Fire";
    Heart => "heart", ph::HEART, "Heart";
    Moon => "moon", ph::MOON_STARS, "Night";
    Sun => "sun", ph::SUN, "Sun";
    Waves => "waves", ph::WAVES, "Waves";
    Wind => "wind", ph::WIND, "Wind";
    Drop => "drop", ph::DROP, "Drop";
    Leaf => "leaf", ph::LEAF, "Leaf";
    Flower => "flower", ph::FLOWER, "Flower";
    Snowflake => "snowflake", ph::SNOWFLAKE, "Ice";
    Planet => "planet", ph::PLANET, "Planet";
    Rocket => "rocket", ph::ROCKET, "Rocket";
    Robot => "robot", ph::ROBOT, "Robot";
    Ghost => "ghost", ph::GHOST, "Ghost";
    Alien => "alien", ph::ALIEN, "Alien";
    Crown => "crown", ph::CROWN, "Crown";
    Diamond => "diamond", ph::DIAMOND, "Diamond";
    Skull => "skull", ph::SKULL, "Skull";
    Cat => "cat", ph::CAT, "Cat";
    Bird => "bird", ph::BIRD, "Bird";
}

impl TrackIcon {
    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().copied().find(|i| i.key() == key)
    }

    /// Icon a track gets until the user picks one, based on its instrument.
    pub fn default_for(instrument: &InstrumentId) -> Self {
        match instrument {
            InstrumentId::Piano | InstrumentId::Organ => Self::Piano,
            InstrumentId::Guitar => Self::Guitar,
            InstrumentId::Bass => Self::Sine,
            InstrumentId::Strings => Self::Notes,
            InstrumentId::Flute => Self::Wind,
            InstrumentId::Brass => Self::Megaphone,
            InstrumentId::Pad => Self::Waves,
            InstrumentId::Harp => Self::Sparkle,
            InstrumentId::Drums | InstrumentId::Kit(_) => Self::Drums,
            InstrumentId::Preset(_) => Self::Saw,
            InstrumentId::Custom(_) => Self::Robot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_round_trip() {
        let mut seen = std::collections::HashSet::new();
        for &i in TrackIcon::all() {
            assert!(seen.insert(i.key()), "duplicate key {}", i.key());
            assert_eq!(TrackIcon::from_key(i.key()), Some(i));
            assert_eq!(serde_json::to_string(&i).unwrap(), format!("\"{}\"", i.key()));
            assert_eq!(i.glyph().chars().count(), 1);
        }
    }
}
