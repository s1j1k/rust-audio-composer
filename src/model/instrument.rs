use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentId {
    Piano,
    Guitar,
    Bass,
    Strings,
    Custom(String),
}

impl InstrumentId {
    pub fn label(&self) -> String {
        match self {
            Self::Piano => "Piano".to_string(),
            Self::Guitar => "Guitar".to_string(),
            Self::Bass => "Bass".to_string(),
            Self::Strings => "Strings".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthParams {
    pub waveform: Waveform,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub brightness: f32,
    pub detune: f32,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            waveform: Waveform::Sine,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
            brightness: 0.5,
            detune: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Saw,
}

#[derive(Clone, Debug)]
pub enum InstrumentPreset {
    BuiltIn { id: InstrumentId, params: SynthParams },
    Custom(CustomInstrument),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomInstrument {
    pub name: String,
    pub description: String,
    pub params: SynthParams,
}

impl InstrumentPreset {
    pub fn piano() -> Self {
        Self::BuiltIn {
            id: InstrumentId::Piano,
            params: SynthParams {
                waveform: Waveform::Triangle,
                attack: 0.005,
                decay: 0.3,
                sustain: 0.2,
                release: 0.4,
                brightness: 0.6,
                detune: 0.0,
            },
        }
    }

    pub fn guitar() -> Self {
        Self::BuiltIn {
            id: InstrumentId::Guitar,
            params: SynthParams {
                waveform: Waveform::Triangle,
                attack: 0.001,
                decay: 0.5,
                sustain: 0.05,
                release: 0.3,
                brightness: 0.8,
                detune: 0.002,
            },
        }
    }

    pub fn bass() -> Self {
        Self::BuiltIn {
            id: InstrumentId::Bass,
            params: SynthParams {
                waveform: Waveform::Saw,
                attack: 0.01,
                decay: 0.2,
                sustain: 0.6,
                release: 0.15,
                brightness: 0.3,
                detune: 0.0,
            },
        }
    }

    pub fn strings() -> Self {
        Self::BuiltIn {
            id: InstrumentId::Strings,
            params: SynthParams {
                waveform: Waveform::Saw,
                attack: 0.15,
                decay: 0.2,
                sustain: 0.8,
                release: 0.5,
                brightness: 0.5,
                detune: 0.003,
            },
        }
    }

    pub fn id(&self) -> InstrumentId {
        match self {
            Self::BuiltIn { id, .. } => id.clone(),
            Self::Custom(c) => InstrumentId::Custom(c.name.clone()),
        }
    }

    pub fn params(&self) -> &SynthParams {
        match self {
            Self::BuiltIn { params, .. } => params,
            Self::Custom(c) => &c.params,
        }
    }

    pub fn label(&self) -> String {
        self.id().label().to_string()
    }
}

pub fn default_presets() -> Vec<InstrumentPreset> {
    vec![
        InstrumentPreset::piano(),
        InstrumentPreset::guitar(),
        InstrumentPreset::bass(),
        InstrumentPreset::strings(),
    ]
}
