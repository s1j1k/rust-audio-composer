use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SampleKind {
    Piano,
    Guitar,
    Bass,
    Strings,
    Flute,
    Brass,
    Organ,
    Pad,
    Harp,
}

impl SampleKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Piano => "Piano",
            Self::Guitar => "Guitar",
            Self::Bass => "Bass",
            Self::Strings => "Strings",
            Self::Flute => "Flute",
            Self::Brass => "Brass",
            Self::Organ => "Organ",
            Self::Pad => "Pad",
            Self::Harp => "Concert Harp",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Piano => "🎹",
            Self::Guitar => "🎸",
            Self::Bass => "🎸",
            Self::Strings => "🎻",
            Self::Flute => "🪈",
            Self::Brass => "🎺",
            Self::Organ => "🎹",
            Self::Pad => "🌊",
            Self::Harp => "🪕",
        }
    }

    pub fn all() -> &'static [SampleKind] {
        &[
            Self::Piano,
            Self::Guitar,
            Self::Bass,
            Self::Strings,
            Self::Flute,
            Self::Brass,
            Self::Organ,
            Self::Pad,
            Self::Harp,
        ]
    }
}

/// Musical style used to group the beat-making presets in pickers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Genre {
    Trap,
    HipHop,
    DrumAndBass,
    Garage,
    Classical,
}

impl Genre {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Trap => "Trap",
            Self::HipHop => "Hip Hop",
            Self::DrumAndBass => "Drum & Bass",
            Self::Garage => "UK Garage",
            Self::Classical => "Classical",
        }
    }

    pub fn all() -> &'static [Genre] {
        &[
            Self::Trap,
            Self::HipHop,
            Self::DrumAndBass,
            Self::Garage,
            Self::Classical,
        ]
    }
}

/// Synthesized, velocity-sensitive instruments modelled on modern beat-pack staples.
/// Rendered in real time by `audio::genre_synth` (no downloads needed).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GenrePreset {
    Bass808,
    TrapBell,
    DarkPluck,
    DustyRhodes,
    BoomBapBass,
    ReeseBass,
    SubBass,
    LiquidPad,
    GarageOrgan,
    TwoStepStab,
    GarageBass,
    ChamberStrings,
    Pizzicato,
}

impl GenrePreset {
    pub fn all() -> &'static [GenrePreset] {
        &[
            Self::Bass808,
            Self::TrapBell,
            Self::DarkPluck,
            Self::DustyRhodes,
            Self::BoomBapBass,
            Self::ReeseBass,
            Self::SubBass,
            Self::LiquidPad,
            Self::GarageOrgan,
            Self::TwoStepStab,
            Self::GarageBass,
            Self::ChamberStrings,
            Self::Pizzicato,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Bass808 => "808 Bass",
            Self::TrapBell => "Trap Bell",
            Self::DarkPluck => "Dark Pluck",
            Self::DustyRhodes => "Dusty Rhodes",
            Self::BoomBapBass => "Boom Bap Bass",
            Self::ReeseBass => "Reese Bass",
            Self::SubBass => "Sub Bass",
            Self::LiquidPad => "Liquid Pad",
            Self::GarageOrgan => "Garage Organ",
            Self::TwoStepStab => "2-Step Chord Stab",
            Self::GarageBass => "Garage Bass",
            Self::ChamberStrings => "Chamber Strings",
            Self::Pizzicato => "Pizzicato",
        }
    }

    pub fn genre(&self) -> Genre {
        match self {
            Self::Bass808 | Self::TrapBell | Self::DarkPluck => Genre::Trap,
            Self::DustyRhodes | Self::BoomBapBass => Genre::HipHop,
            Self::ReeseBass | Self::SubBass | Self::LiquidPad => Genre::DrumAndBass,
            Self::GarageOrgan | Self::TwoStepStab | Self::GarageBass => Genre::Garage,
            Self::ChamberStrings | Self::Pizzicato => Genre::Classical,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Bass808
            | Self::BoomBapBass
            | Self::ReeseBass
            | Self::SubBass
            | Self::GarageBass => "🔊",
            Self::TrapBell => "🔔",
            Self::DarkPluck | Self::TwoStepStab => "⚡",
            Self::DustyRhodes | Self::GarageOrgan => "🎹",
            Self::LiquidPad => "🌊",
            Self::ChamberStrings | Self::Pizzicato => "🎻",
        }
    }

    /// Is this mainly a bass instrument? (Used to pick preview notes.)
    pub fn is_bass(&self) -> bool {
        matches!(
            self,
            Self::Bass808 | Self::BoomBapBass | Self::ReeseBass | Self::SubBass | Self::GarageBass
        )
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::all().iter().copied().find(|p| p.label() == label)
    }
}

/// Synthesized genre drum kits (the FreePats kit stays available as `InstrumentId::Drums`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrumKit {
    Trap808,
    BoomBap,
    Jungle,
    TwoStep,
}

impl DrumKit {
    pub fn all() -> &'static [DrumKit] {
        &[Self::Trap808, Self::BoomBap, Self::Jungle, Self::TwoStep]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Trap808 => "Trap 808 Kit",
            Self::BoomBap => "Boom Bap Kit",
            Self::Jungle => "D&B / Jungle Kit",
            Self::TwoStep => "UK Garage Kit",
        }
    }

    pub fn genre(&self) -> Genre {
        match self {
            Self::Trap808 => Genre::Trap,
            Self::BoomBap => Genre::HipHop,
            Self::Jungle => Genre::DrumAndBass,
            Self::TwoStep => Genre::Garage,
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::all().iter().copied().find(|k| k.label() == label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentId {
    Piano,
    Guitar,
    Bass,
    Strings,
    Flute,
    Brass,
    Organ,
    Pad,
    Drums,
    Custom(String),
    Harp,
    Preset(GenrePreset),
    Kit(DrumKit),
    /// A sample instrument from the user's library (imported note samples).
    Sampled(String),
}

impl InstrumentId {
    pub fn label(&self) -> String {
        match self {
            Self::Piano => "Piano".to_string(),
            Self::Guitar => "Guitar".to_string(),
            Self::Bass => "Bass".to_string(),
            Self::Strings => "Strings".to_string(),
            Self::Flute => "Flute".to_string(),
            Self::Brass => "Brass".to_string(),
            Self::Organ => "Organ".to_string(),
            Self::Pad => "Pad".to_string(),
            Self::Drums => "Drums".to_string(),
            Self::Custom(name) => name.clone(),
            Self::Harp => "Concert Harp".to_string(),
            Self::Preset(p) => p.label().to_string(),
            Self::Kit(k) => k.label().to_string(),
            Self::Sampled(name) => name.clone(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Piano => "🎹",
            Self::Guitar => "🎸",
            Self::Bass => "🎸",
            Self::Strings => "🎻",
            Self::Flute => "🪈",
            Self::Brass => "🎺",
            Self::Organ => "🎹",
            Self::Pad => "🌊",
            Self::Drums => "♪",
            Self::Custom(_) => "✨",
            Self::Harp => "🪕",
            Self::Preset(p) => p.icon(),
            Self::Kit(_) => "♪",
            Self::Sampled(_) => "♫",
        }
    }

    pub fn is_drum_track(&self) -> bool {
        matches!(self, Self::Drums | Self::Kit(_))
    }

    /// Resolve a picker label back to an instrument (unknown names become `Custom`).
    pub fn from_label(label: &str) -> Self {
        if label == "Drums" {
            return Self::Drums;
        }
        if let Some(kind) = SampleKind::all().iter().find(|k| k.label() == label) {
            return Self::from_sample_kind(*kind);
        }
        if let Some(p) = GenrePreset::from_label(label) {
            return Self::Preset(p);
        }
        if let Some(k) = DrumKit::from_label(label) {
            return Self::Kit(k);
        }
        if crate::library::names().iter().any(|n| n == label) {
            return Self::Sampled(label.to_string());
        }
        Self::Custom(label.to_string())
    }

    pub fn sample_kind(&self) -> Option<SampleKind> {
        match self {
            Self::Piano => Some(SampleKind::Piano),
            Self::Guitar => Some(SampleKind::Guitar),
            Self::Bass => Some(SampleKind::Bass),
            Self::Strings => Some(SampleKind::Strings),
            Self::Flute => Some(SampleKind::Flute),
            Self::Brass => Some(SampleKind::Brass),
            Self::Organ => Some(SampleKind::Organ),
            Self::Pad => Some(SampleKind::Pad),
            Self::Harp => Some(SampleKind::Harp),
            Self::Drums | Self::Custom(_) | Self::Preset(_) | Self::Kit(_) | Self::Sampled(_) => None,
        }
    }

    pub fn from_sample_kind(kind: SampleKind) -> Self {
        match kind {
            SampleKind::Piano => Self::Piano,
            SampleKind::Guitar => Self::Guitar,
            SampleKind::Bass => Self::Bass,
            SampleKind::Strings => Self::Strings,
            SampleKind::Flute => Self::Flute,
            SampleKind::Brass => Self::Brass,
            SampleKind::Organ => Self::Organ,
            SampleKind::Pad => Self::Pad,
            SampleKind::Harp => Self::Harp,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthParams {
    pub sample_kind: SampleKind,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub brightness: f32,
    pub detune: f32,
    pub filter_cutoff: f32,
    /// Saturation amount 0..1 (grit / warmth). Older project files default to 0.
    #[serde(default)]
    pub drive: f32,
    /// When set, the instrument is rendered by the genre synth engine instead of a sample.
    #[serde(default)]
    pub preset: Option<GenrePreset>,
    /// When set, notes play from this sample instrument in the user's library.
    #[serde(default)]
    pub library: Option<String>,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            sample_kind: SampleKind::Piano,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
            brightness: 0.5,
            detune: 0.0,
            filter_cutoff: 1.0,
            drive: 0.0,
            preset: None,
            library: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomInstrument {
    pub name: String,
    pub description: String,
    pub base_kind: SampleKind,
    pub params: SynthParams,
}

impl CustomInstrument {
    /// Human-readable name of whatever this instrument is built on.
    pub fn base_label(&self) -> String {
        match self.params.preset {
            Some(p) => p.label().to_string(),
            None => self.base_kind.label().to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentProfile {
    pub name: String,
    pub params: SynthParams,
}

impl InstrumentProfile {
    pub fn builtin(kind: SampleKind) -> Self {
        Self {
            name: kind.label().to_string(),
            params: default_params_for_kind(kind),
        }
    }

    pub fn from_custom(custom: &CustomInstrument) -> Self {
        Self {
            name: custom.name.clone(),
            params: custom.params.clone(),
        }
    }
}

pub fn default_params_for_kind(kind: SampleKind) -> SynthParams {
    let (attack, decay, sustain, release, brightness, detune, filter_cutoff) = match kind {
        SampleKind::Piano => (0.002, 0.35, 0.15, 0.45, 0.65, 0.0, 0.85),
        SampleKind::Guitar => (0.001, 0.55, 0.02, 0.35, 0.75, 0.003, 0.7),
        SampleKind::Bass => (0.008, 0.25, 0.55, 0.18, 0.25, 0.0, 0.45),
        SampleKind::Strings => (0.12, 0.15, 0.85, 0.55, 0.5, 0.004, 0.6),
        SampleKind::Flute => (0.04, 0.1, 0.75, 0.25, 0.35, 0.001, 0.55),
        SampleKind::Brass => (0.025, 0.08, 0.8, 0.2, 0.7, 0.002, 0.65),
        SampleKind::Organ => (0.01, 0.05, 0.9, 0.08, 0.55, 0.001, 0.75),
        SampleKind::Pad => (0.25, 0.3, 0.85, 0.9, 0.4, 0.005, 0.5),
        SampleKind::Harp => (0.001, 1.6, 0.0, 0.9, 0.55, 0.0, 0.9),
    };
    SynthParams {
        sample_kind: kind,
        attack,
        decay,
        sustain,
        release,
        brightness,
        detune,
        filter_cutoff,
        drive: 0.0,
        preset: None,
        library: None,
    }
}

pub fn default_profiles() -> Vec<InstrumentProfile> {
    let mut profiles: Vec<InstrumentProfile> = SampleKind::all()
        .iter()
        .map(|&k| InstrumentProfile::builtin(k))
        .collect();
    for &p in GenrePreset::all() {
        profiles.push(InstrumentProfile {
            name: p.label().to_string(),
            params: preset_nominal_params(p),
        });
    }
    profiles
}

/// The "neutral" macro settings of a genre preset, expressed in the same terms the
/// timbre designer edits (attack/decay/sustain/release/brightness/cutoff/detune/drive).
/// A custom instrument built on a preset stores absolute values; the synth applies them
/// relative to these nominal values, so the nominal params reproduce the preset exactly.
pub fn preset_nominal_params(preset: GenrePreset) -> SynthParams {
    let (attack, decay, sustain, release, detune, drive) = match preset {
        GenrePreset::Bass808 => (0.002, 2.2, 0.0, 0.35, 0.0, 0.35),
        GenrePreset::TrapBell => (0.001, 1.4, 0.0, 0.9, 0.0, 0.0),
        GenrePreset::DarkPluck => (0.002, 0.45, 0.0, 0.3, 0.012, 0.1),
        GenrePreset::DustyRhodes => (0.002, 2.0, 0.0, 0.45, 0.0, 0.12),
        GenrePreset::BoomBapBass => (0.004, 0.9, 0.35, 0.15, 0.0, 0.25),
        GenrePreset::ReeseBass => (0.01, 0.4, 0.9, 0.2, 0.014, 0.3),
        GenrePreset::SubBass => (0.006, 0.2, 1.0, 0.1, 0.0, 0.05),
        GenrePreset::LiquidPad => (0.7, 1.2, 0.8, 1.6, 0.012, 0.0),
        GenrePreset::GarageOrgan => (0.002, 0.28, 0.22, 0.12, 0.003, 0.1),
        GenrePreset::TwoStepStab => (0.002, 0.35, 0.08, 0.18, 0.01, 0.05),
        GenrePreset::GarageBass => (0.003, 0.3, 0.75, 0.1, 0.004, 0.35),
        GenrePreset::ChamberStrings => (0.28, 0.6, 0.85, 0.7, 0.009, 0.0),
        GenrePreset::Pizzicato => (0.001, 0.38, 0.0, 0.18, 0.003, 0.0),
    };
    SynthParams {
        sample_kind: SampleKind::Piano,
        attack,
        decay,
        sustain,
        release,
        brightness: 0.5,
        detune,
        filter_cutoff: 0.7,
        drive,
        preset: Some(preset),
        library: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip() {
        for &p in GenrePreset::all() {
            assert_eq!(InstrumentId::from_label(p.label()), InstrumentId::Preset(p));
        }
        for &k in DrumKit::all() {
            assert_eq!(InstrumentId::from_label(k.label()), InstrumentId::Kit(k));
        }
        for &s in SampleKind::all() {
            assert_eq!(
                InstrumentId::from_label(s.label()),
                InstrumentId::from_sample_kind(s)
            );
        }
        assert_eq!(InstrumentId::from_label("Drums"), InstrumentId::Drums);
        assert_eq!(
            InstrumentId::from_label("My Thing"),
            InstrumentId::Custom("My Thing".into())
        );
    }

    #[test]
    fn old_synth_params_still_deserialize() {
        let json = r#"{"sample_kind":"Piano","attack":0.1,"decay":0.1,"sustain":0.5,
            "release":0.2,"brightness":0.5,"detune":0.0,"filter_cutoff":1.0}"#;
        let p: SynthParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.drive, 0.0);
        assert!(p.preset.is_none());
    }
}
