use egui::Ui;

struct Section {
    title: &'static str,
    body: &'static str,
}

const SECTIONS: &[Section] = &[
    Section {
        title: "Getting started",
        body: "Rust Audio Composer is a desktop DAW. The timeline at the top shows all tracks; \
               the piano roll at the bottom edits the selected track.\n\n\
               1. Click a track row to select it.\n\
               2. Click the grid to move the playhead (where recording starts).\n\
               3. Open 🎹 Piano, enable ⏺ Record, and play notes.\n\
               4. Press ▶ Play to hear your work.",
    },
    Section {
        title: "Transport bar",
        body: "▶ Play / ⏸ Pause — start or pause playback.\n\
               ⏹ Stop — return playhead to the start.\n\
               ⏮ / ⏭ — jump to start (Home) or end (End).\n\
               ⏺ Record — capture piano input or drum sequencer hits onto the timeline.\n\n\
               Panel toggles: 🎹 Piano · 🎼 Compose · ✨ AI Instrument · 🥁 Drums · 🎛 DJ · ⚙ Options · 📖 Guide\n\n\
               💾 Save / 📂 Load — project files (.rac.json).\n\
               🎵 Export WAV — offline mix of all tracks.",
    },
    Section {
        title: "Timeline & clips",
        body: "Each row is a track. Colored bubbles are clips — groups of notes from a recording or edit.\n\n\
               · Drag a clip to move it in time.\n\
               · Drag clip edges to trim (hidden notes are kept and recoverable).\n\
               · Drag the loop handle on the right to repeat a clip.\n\
               · Click a clip to select it; edit name and loop in the left track panel.\n\n\
               Song sections (A, B, C…) define structure with bar counts. They appear as colored \
               regions and can be added from the collapsible panel on the timeline.\n\n\
               Zoom with − / + or Ctrl/⌘ + scroll over the timeline.",
    },
    Section {
        title: "Piano roll",
        body: "Detailed editor for the selected melodic or drum track.\n\n\
               · Click empty grid to add notes (melodic tracks).\n\
               · Drag notes to move; drag edges to change length.\n\
               · Delete removes the selected note.\n\
               · Scroll vertically on the piano keys to shift the visible pitch range.\n\
               · Ctrl/⌘ + scroll zooms the beat grid.\n\
               · The ruler shows bar numbers and beats 1–4.",
    },
    Section {
        title: "Virtual piano",
        body: "Open from 🎹 Piano in the transport bar.\n\n\
               · Click keys — lower on the key = louder.\n\
               · Computer keyboard: white keys Z X C V B N M , . / · black keys S D G H J.\n\
               · Hold keys longer for stronger attack and velocity when recording.\n\
               · Shift = accent · Alt = soft.\n\
               · Adjust octaves and base pitch at the top of the window.",
    },
    Section {
        title: "Drum sequencer",
        body: "Open 🥁 Drums (or add a + Drum Track from the sidebar).\n\n\
               · 16-step grid — click to toggle hits, Shift+click for accent, right-click to clear.\n\
               · ▶ Run loop previews the pattern while you edit.\n\
               · Changes stay in memory until you press ⏺ Record in the transport bar.\n\
               · ↺ Reload bar loads the pattern at the current playhead bar.\n\
               · Save patterns to the library and insert them at the playhead.",
    },
    Section {
        title: "Track panel (left sidebar)",
        body: "· Volume and mute for the selected track.\n\
               · Selected clip — rename, enable loop, set loop end beat.\n\
               · Piano roll view — octaves and base octave for the editor.\n\
               · Instrument picker — melodic samples, drums, or custom AI instruments.\n\
               · + Melodic Track / + Drum Track to add tracks.\n\
               · Master volume and shortcut to DJ Tools.",
    },
    Section {
        title: "Composition assistant",
        body: "Open 🎼 Compose for the right-side panel.\n\n\
               · Detects key from notes on all tracks.\n\
               · Suggests chord progressions — click to insert at the playhead.\n\
               · Interactive circle of fifths for key relationships.",
    },
    Section {
        title: "AI instrument designer",
        body: "Open ✨ AI Instrument.\n\n\
               1. Pick a base sample (piano, guitar, etc.).\n\
               2. Describe the sound you want.\n\
               3. Generate, preview, and save to your library.\n\
               4. Assign saved instruments on any track from the sidebar picker.\n\n\
               Uses local keyword mapping today; cloud AI is planned.",
    },
    Section {
        title: "DJ tools",
        body: "Open 🎛 DJ for mix effects on the master bus and the selected track:\n\n\
               · Distance / underwater — far-away muffled tone.\n\
               · Reverb — space and depth.\n\
               · Phaser swirl — sweeping motion.\n\
               · Brightness — low-pass filter.\n\n\
               Per-track effects apply fully on WAV export.",
    },
    Section {
        title: "Options & appearance",
        body: "Open ⚙ Options to switch interface font between code (monospace) and sans serif.\n\n\
               Hover ⓘ icons throughout the UI for quick tips; click to pin a detail bubble.",
    },
    Section {
        title: "Keyboard shortcuts",
        body: "Home — go to start\n\
               End — go to end\n\
               Delete — remove selected note in piano roll\n\n\
               Piano window (when focused):\n\
               Z–M, , . / — white keys · S D G H J — black keys\n\
               Shift — accent · Alt — soft",
    },
];

pub fn show_user_guide(ui: &mut Ui) {
    ui.heading("User Guide");
    ui.label(
        "A quick reference for features in Rust Audio Composer. More detail appears in ⓘ \
         bubbles across the interface.",
    );
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for section in SECTIONS {
                ui.add_space(6.0);
                ui.collapsing(section.title, |ui| {
                    ui.label(section.body);
                });
            }
        });
}
