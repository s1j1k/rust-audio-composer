use super::{instrument::InstrumentId, note::Note};

#[derive(Clone, Debug)]
pub struct Track {
    pub name: String,
    pub instrument: InstrumentId,
    pub volume: f32,
    pub muted: bool,
    pub notes: Vec<Note>,
    pub color: [u8; 3],
}

impl Track {
    pub fn new(index: usize) -> Self {
        let colors: [[u8; 3]; 6] = [
            [100, 180, 255],
            [255, 140, 100],
            [140, 220, 140],
            [220, 140, 220],
            [255, 220, 100],
            [180, 180, 255],
        ];
        Self {
            name: format!("Track {}", index + 1),
            instrument: InstrumentId::Piano,
            volume: 0.8,
            muted: false,
            notes: Vec::new(),
            color: colors[index % colors.len()],
        }
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.sort_by(|a, b| {
            a.start_beat
                .partial_cmp(&b.start_beat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}
