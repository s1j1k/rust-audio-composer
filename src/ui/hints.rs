use crate::app::DAWApp;
use egui::{Color32, PopupCloseBehavior, RichText, Ui};

pub mod text {
    pub const TIMELINE: &str =
        "Clips group notes from each recording or edit. Drag to move, trim edges to crop \
         (hidden notes stay recoverable), and drag the loop handle on the right to repeat.";
    pub const TIMELINE_SECTIONS: &str =
        "Define song structure with labels and bar counts (e.g. A = 8 bars, B = 18 bars). \
         Sections are placed end-to-end and appear as colored regions on the timeline. \
         Click a section chip to jump the playhead.";
    pub const TIMELINE_ZOOM: &str =
        "Zoom the beat grid with − / + or Ctrl/⌘ + scroll over the timeline or piano roll.";
    pub const PIANO_ROLL: &str =
        "Scroll on the piano roll to shift pitch. Ctrl/⌘ + scroll zooms. The ruler shows \
         bar numbers and beats 1–4. Drag notes to move; drag edges to resize.";
    pub const TRACK_PANEL: &str =
        "Click a track row on the timeline to select it. Volume and mute apply during playback.";
    pub const TRACK_CLIP: &str =
        "Click a clip bubble on the timeline to edit its name, loop, and trim here.";
    pub const TRACK_PIANO_ROLL: &str =
        "Octaves and base set the visible pitch range. Scroll on piano keys in the roll \
         to shift pitch up or down.";
    pub const DRUM_SEQUENCER: &str =
        "The pattern loops while you edit. Steps update audio immediately. Press Record \
         in the transport bar to write changes into a timeline clip. Right-click a step to clear.";
    pub const DRUM_PREVIEW: &str =
        "Run loop previews the draft pattern only. Timeline clips update when Record is active.";
    pub const DRUM_LIBRARY: &str =
        "Save the current bar as a reusable pattern, then insert it at the playhead bar.";
    pub const PIANO: &str =
        "Click keys — lower on the key = louder. Hold keyboard keys longer for stronger attack. \
         Scroll shifts octave; Shift = accent, Alt = soft.";
    pub const PIANO_KEYBOARD: &str =
        "Highlighted keys play while this window is focused. White keys: Z X C V B N M , . / \
         · Black keys: S D G H J.";
    pub const TRANSPORT: &str =
        "Space = play/pause · R = record · Home/End = start/end · Delete = remove selected note.";
    pub const TRANSPORT_BPM: &str = "Project tempo in beats per minute (40–240).";
    pub const TRANSPORT_TIME_SIG: &str =
        "Beats per bar and note value (e.g. 4/4). Affects bar lines and section lengths.";
    pub const TRANSPORT_SAVE: &str =
        "Save/load project JSON. Export WAV renders the full mix offline.";
    pub const DJ_PANEL: &str =
        "Mix effects for the whole song and per track — distance, underwater muffling, \
         reverb, and phaser swirl.";
    pub const FX_DISTANCE: &str =
        "Pushes sound farther away / underwater. Adds automatic low-pass and reverb.";
    pub const FX_REVERB: &str = "Room size and depth layered on top of distance.";
    pub const FX_PHASER: &str = "Swirling phase movement — good for fades and motion.";
    pub const FX_PHASER_RATE: &str = "How fast the phaser sweeps (Hz).";
    pub const FX_BRIGHTNESS: &str =
        "Manual low-pass filter. Lower = darker / more muffled.";
    pub const INSTRUMENT_DESIGNER: &str =
        "Pick a base sample, describe the sound in plain language, generate, preview, \
         then save to your library and assign it on a track.";
    pub const COMPOSITION: &str =
        "Analyzes notes on all tracks to guess key and suggest chord progressions.";
    pub const CIRCLE_OF_FIFTHS: &str =
        "Click a key to see its relative major/minor partner and diatonic hints.";
    pub const UI_HINTS: &str =
        "Hover ⓘ icons for quick tips. Click to pin a detail bubble. Enable inline help \
         to show short descriptions under section headers.";
}

/// Small info control: hover shows tooltip, click pins a detail bubble.
pub fn bubble(ui: &mut Ui, id: impl std::hash::Hash, text: &str) {
    let popup_id = ui.id().with(id);
    let resp = ui.small_button("ⓘ").on_hover_text(text);
    if resp.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &resp,
        PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_max_width(340.0);
            ui.label(text);
        },
    );
}

pub fn heading_row(ui: &mut Ui, app: &DAWApp, title: &str, id: impl std::hash::Hash, hint: &str) {
    ui.horizontal(|ui| {
        ui.heading(title);
        bubble(ui, id, hint);
    });
    expanded_hint(ui, app, hint);
}

pub fn expanded_hint(ui: &mut Ui, app: &DAWApp, hint: &str) {
    if app.ui_hints_expanded {
        ui.label(RichText::new(hint).small().color(Color32::GRAY));
    }
}

pub fn label_with_bubble(ui: &mut Ui, label: &str, id: impl std::hash::Hash, hint: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        bubble(ui, id, hint);
    });
}
