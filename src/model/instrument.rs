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
        ]
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
    Custom(String),
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
            Self::Custom(name) => name.clone(),
        }
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
            Self::Custom(_) => None,
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
    match kind {
        SampleKind::Piano => SynthParams {
            sample_kind: kind,
            attack: 0.002,
            decay: 0.35,
            sustain: 0.15,
            release: 0.45,
            brightness: 0.65,
            detune: 0.0,
            filter_cutoff: 0.85,
        },
        SampleKind::Guitar => SynthParams {
            sample_kind: kind,
            attack: 0.001,
            decay: 0.55,
            sustain: 0.02,
            release: 0.35,
            brightness: 0.75,
            detune: 0.003,
            filter_cutoff: 0.7,
        },
        SampleKind::Bass => SynthParams {
            sample_kind: kind,
            attack: 0.008,
            decay: 0.25,
            sustain: 0.55,
            release: 0.18,
            brightness: 0.25,
            detune: 0.0,
            filter_cutoff: 0.45,
        },
        SampleKind::Strings => SynthParams {
            sample_kind: kind,
            attack: 0.12,
            decay: 0.15,
            sustain: 0.85,
            release: 0.55,
            brightness: 0.5,
            detune: 0.004,
            filter_cutoff: 0.6,
        },
        SampleKind::Flute => SynthParams {
            sample_kind: kind,
            attack: 0.04,
            decay: 0.1,
            sustain: 0.75,
            release: 0.25,
            brightness: 0.35,
            detune: 0.001,
            filter_cutoff: 0.55,
        },
        SampleKind::Brass => SynthParams {
            sample_kind: kind,
            attack: 0.025,
            decay: 0.08,
            sustain: 0.8,
            release: 0.2,
            brightness: 0.7,
            detune: 0.002,
            filter_cutoff: 0.65,
        },
        SampleKind::Organ => SynthParams {
            sample_kind: kind,
            attack: 0.01,
            decay: 0.05,
            sustain: 0.9,
            release: 0.08,
            brightness: 0.55,
            detune: 0.001,
            filter_cutoff: 0.75,
        },
        SampleKind::Pad => SynthParams {
            sample_kind: kind,
            attack: 0.25,
            decay: 0.3,
            sustain: 0.85,
            release: 0.9,
            brightness: 0.4,
            detune: 0.005,
            filter_cutoff: 0.5,
        },
    }
}

pub fn default_profiles() -> Vec<InstrumentProfile> {
    SampleKind::all()
        .iter()
        .map(|&k| InstrumentProfile::builtin(k))
        .collect()
}
