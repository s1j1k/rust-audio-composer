//! Undo / redo by snapshots of the whole project.
//!
//! Rather than wiring every edit (note drags, clip trims, sliders, recording, ...) into
//! explicit commands, the app calls [`History::commit`] once per frame while the user is
//! *not* in the middle of something (no mouse button held, no text field focused, not
//! recording). If the project changed since the last commit, the previous state becomes
//! one undo step. A whole drag or a whole recorded take therefore undoes in one go.
//!
//! The playhead isn't part of the comparison: moving it or playing isn't an edit.

use crate::model::Project;

/// How many undo steps to keep. Projects are small (notes, not audio), so a clone each
/// is cheap.
const MAX_STEPS: usize = 200;

pub struct History {
    undo: Vec<Project>,
    redo: Vec<Project>,
    /// The project as of the last commit.
    baseline: Project,
}

/// Same musical content (everything except the playhead).
pub fn same_content(a: &Project, b: &Project) -> bool {
    a.tracks == b.tracks
        && a.bpm == b.bpm
        && a.time_sig_numerator == b.time_sig_numerator
        && a.time_sig_denominator == b.time_sig_denominator
        && a.total_beats == b.total_beats
        && a.sections == b.sections
}

impl History {
    pub fn new(project: &Project) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            baseline: project.clone(),
        }
    }

    /// Record an undo step if `current` differs from the last committed state.
    /// Returns true if a step was recorded. A new edit clears the redo stack.
    pub fn commit(&mut self, current: &Project) -> bool {
        if same_content(current, &self.baseline) {
            return false;
        }
        let previous = std::mem::replace(&mut self.baseline, current.clone());
        self.undo.push(previous);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    /// The project to go back to, if any. Any uncommitted edit is committed first, so
    /// undo always reverts the most recent change.
    pub fn undo(&mut self, current: &Project) -> Option<Project> {
        self.commit(current);
        let previous = self.undo.pop()?;
        let now = std::mem::replace(&mut self.baseline, previous.clone());
        self.redo.push(now);
        Some(previous)
    }

    /// The project to go forward to, if any. If something was edited since the last
    /// undo, that edit wins and there is nothing to redo.
    pub fn redo(&mut self, current: &Project) -> Option<Project> {
        if self.commit(current) {
            return None;
        }
        let next = self.redo.pop()?;
        let now = std::mem::replace(&mut self.baseline, next.clone());
        self.undo.push(now);
        Some(next)
    }

    pub fn can_undo(&self, current: &Project) -> bool {
        !self.undo.is_empty() || !same_content(current, &self.baseline)
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Note, NoteClip};

    fn with_clip(mut p: Project, id: u64) -> Project {
        let mut c = NoteClip::new(id, format!("C{id}"), 0.0);
        c.add_note(Note::new(60, 0.0, 1.0));
        p.tracks[0].add_clip(c);
        p
    }

    #[test]
    fn undo_and_redo_walk_the_edits() {
        let p0 = Project::default();
        let mut h = History::new(&p0);
        let p1 = with_clip(p0.clone(), 1);
        assert!(h.commit(&p1));
        let p2 = with_clip(p1.clone(), 2);
        assert!(h.commit(&p2));

        let back = h.undo(&p2).unwrap();
        assert!(same_content(&back, &p1));
        let back = h.undo(&back).unwrap();
        assert!(same_content(&back, &p0));
        assert!(h.undo(&back).is_none());

        let fwd = h.redo(&back).unwrap();
        assert!(same_content(&fwd, &p1));
        let fwd = h.redo(&fwd).unwrap();
        assert!(same_content(&fwd, &p2));
        assert!(h.redo(&fwd).is_none());
    }

    #[test]
    fn playhead_moves_are_not_edits() {
        let p0 = Project::default();
        let mut h = History::new(&p0);
        let mut moved = p0.clone();
        moved.playhead_beat = 12.0;
        assert!(!h.commit(&moved));
        assert!(!h.can_undo(&moved));
    }

    #[test]
    fn uncommitted_edit_is_undone_first_and_a_new_edit_clears_redo() {
        let p0 = Project::default();
        let mut h = History::new(&p0);
        let p1 = with_clip(p0.clone(), 1);
        // Never committed (e.g. Undo clicked right after typing a clip name).
        let back = h.undo(&p1).unwrap();
        assert!(same_content(&back, &p0));
        assert!(h.can_redo());
        let other = with_clip(back.clone(), 9);
        assert!(h.redo(&other).is_none(), "a new edit replaces the redo history");
        assert!(!h.can_redo());
    }
}
