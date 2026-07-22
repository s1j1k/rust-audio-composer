use crate::model::instrument::CustomInstrument;
use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROJECT_FILE_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedProject {
    pub version: u32,
    pub project: Project,
    pub custom_instruments: Vec<CustomInstrument>,
    pub master_volume: f32,
}

impl SavedProject {
    pub fn from_app(project: Project, custom_instruments: Vec<CustomInstrument>, master_volume: f32) -> Self {
        Self {
            version: PROJECT_FILE_VERSION,
            project,
            custom_instruments,
            master_volume,
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
