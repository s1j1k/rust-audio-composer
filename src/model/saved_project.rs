use crate::model::effects::MasterEffects;
use crate::model::instrument::CustomInstrument;
use crate::model::drum_pattern::SavedDrumPattern;
use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// v6: effect automation lanes, DJ mixer channels / crossfader, track icons, channel EQ,
/// filter and echo. All new fields default, so v5 files load unchanged.
pub const PROJECT_FILE_VERSION: u32 = 6;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FxParam, TrackIcon, XfadeSide};

    #[test]
    fn dj_project_round_trips_through_a_file() {
        let mut project = Project::default();
        project.tracks[0].lane_mut(FxParam::Filter).set_point(0.0, 0.0);
        project.tracks[0].lane_mut(FxParam::Filter).set_point(16.0, -0.8);
        project.tracks[0].icon = Some(TrackIcon::Rocket);
        project.tracks[1].dj.side = XfadeSide::B;
        project.dj.crossfader_lane.set_point(8.0, 0.5);
        let saved = SavedProject::from_app(project, Vec::new(), Vec::new(), 0.8, MasterEffects::default());
        let dir = std::env::temp_dir().join(format!("rac-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dj.rac.json");
        saved.save_to_path(&path).unwrap();
        let loaded = SavedProject::load_from_path(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded.version, PROJECT_FILE_VERSION);
        let t = &loaded.project.tracks[0];
        assert!((t.value_at(FxParam::Filter, 8.0) + 0.4).abs() < 1e-6);
        assert_eq!(t.icon, Some(TrackIcon::Rocket));
        assert_eq!(loaded.project.tracks[1].dj.side, XfadeSide::B);
        assert_eq!(loaded.project.dj.crossfader_at(8.0), 0.5);
    }
}
