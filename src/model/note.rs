#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub pitch: u8,
    pub start_beat: f64,
    pub duration_beats: f64,
    pub velocity: f32,
}

impl Note {
    pub fn new(pitch: u8, start_beat: f64, duration_beats: f64) -> Self {
        Self {
            pitch,
            start_beat,
            duration_beats,
            velocity: 0.8,
        }
    }

    pub fn end_beat(&self) -> f64 {
        self.start_beat + self.duration_beats
    }
}
