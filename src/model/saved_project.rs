use crate::model::effects::MasterEffects;
use crate::model::instrument::CustomInstrument;
use crate::model::drum_pattern::SavedDrumPattern;
use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROJECT_FILE_VERSION: u32 = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedProject {
    pub version: u32,
    pub project: Project,
    pub custom_instruments: Vec<CustomInstrument>,
    #[serde(default)]
    pub drum_patterns: Vec<SavedDrumPattern>,
    pub master_volume: f32,
    #[serde(default)]
    pub master_effects: MasterEffects,
}

impl SavedProject {
    pub fn from_app(
        project: Project,
        custom_instruments: Vec<CustomInstrument>,
        drum_patterns: Vec<SavedDrumPattern>,
        master_volume: f32,
        master_effects: MasterEffects,
    ) -> Self {
        Self {
            version: PROJECT_FILE_VERSION,
            project,
            custom_instruments,
            drum_patterns,
            master_volume,
            master_effects,
        }
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| format!("Invalid project file: {}", e))
    }
}
