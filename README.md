# Rust Audio Composer

A desktop Digital Audio Workstation (DAW) built in Rust with egui. Play piano and guitar sounds, record multi-track compositions, and get AI-assisted composition help.

## Features

### Virtual Piano
- Clickable piano keyboard window (🎹 button in transport bar)
- Computer keyboard mapping: **Z-M** for white keys, **S/D/G/H/J** for black keys
- Polyphonic playback with ADSR envelopes

### Instruments
Built-in instruments use **FreePats** sample libraries ([CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) — safe for commercial use). Samples download on first launch and cache in `~/.rust-audio-composer/samples/`. See [CREDITS.md](CREDITS.md) for full attribution.

| Instrument | Source |
|------------|--------|
| Piano | FreePats Upright Piano KW |
| Guitar | FreePats Spanish Classical Guitar |
| Bass | FreePats Electric Bass YR |
| Strings / Flute / Brass / Organ / Pad | FreePats synth collection |

If offline, the app falls back to procedural waveforms until samples can be downloaded.

Each instrument has an emoji icon in the track picker (🎹 🎸 🎻 🥁 etc.).

### Beat Sequencer (🥁 Drums)
- **16-step drum grid** with emoji icons per drum (kick, snare, hi-hats, toms, clap, cymbal, shaker)
- Click to toggle a hit; **Shift+click** for accent (louder); **right-click** to remove
- Drum samples from [FreePats Synthesizer Percussion](https://github.com/freepats/synthesizer-percussion) (**CC0**) — cached in `~/.rust-audio-composer/drums/`
- Add drum tracks via **+ Drum Track** in the sidebar or the sequencer panel
- Default project includes a drum track on track 2

### Velocity
- Notes store **velocity** (0–1) which controls playback and export volume
- **Virtual piano:** click lower on a key = louder; **Shift** = accent, **Alt** = soft on keyboard
- **Drum sequencer:** normal hits vs shift-accent hits
- Recording captures velocity automatically

### DJ Tools (🎛 DJ)
GarageBand-style mix effects for the **master bus** and **each track**:
- **Distance / underwater** — muffled, far-away sound (auto low-pass + reverb)
- **Reverb** — space and depth
- **Phaser swirl** — phase movement (fade in/out character)
- **Brightness** — manual low-pass filter

Effects apply live on the master output; per-track effects apply fully on **WAV export** and partially during live playback.

### Timeline & Piano Roll

The main view is split into two sections:

**Timeline (top)** — one row per track:
- Click a row to select that track
- Click anywhere on a row's grid to set the playhead (where recording starts)
- Notes appear as small blocks whose height reflects pitch — a high-level overview of each track's shape
- The red vertical line is the playhead, shared across all tracks
- Selected track is highlighted with a colored border

**Piano Roll (bottom)** — detailed editor for the selected track:
- Vertical piano keys on the left (C3–B5)
- Full note grid with pitch-by-pitch detail
- Click the grid to place notes, or use the virtual piano while recording

### Multi-Track
- Add unlimited tracks via the sidebar
- Per-track volume, mute, and instrument selection
- Master volume control

### Transport
- **Play / Pause** — playback with playhead
- **Stop** — reset playhead to start
- **Record** — notes played on the piano are recorded to the current track at the playhead
- **BPM** slider (40–240)
- **Time signature** — adjustable numerator and denominator (2/4, 4/4, 8/8, etc.)

### Composition Assistant (🎼 Compose panel)
- **Key detection** — analyzes notes across all tracks
- **Chord progression suggestions** — pop, jazz, and minor-key progressions
- Click a chord button to insert it at the playhead
- **Interactive Circle of Fifths** — click keys to learn relationships
- Educational collapsible section explaining the circle of fifths

### AI Instrument Designer (✨ AI Instrument)
1. Select a **base instrument** from the library (uses its cached waveform sample)
2. Describe how you want it to sound
3. **Generate** → preview → **Save to Library**
4. Saved custom instruments appear in the track instrument picker

Currently uses local keyword analysis to map descriptions onto base samples. Cloud LLM integration is planned (see TODO).

### Save, Load & Export
- **💾 Save Project** — saves all tracks, notes, BPM, time signature, and custom instruments to a `.rac.json` file
- **📂 Load Project** — restores a saved project
- **🎵 Export WAV** — renders the full composition to a WAV audio file

## Running

From the project directory:

```bash
cd rust-audio-composer
cargo run
```

The first build can take a minute or two; later runs are much faster.

**Requirements:** a working audio output device (speakers or headphones). On macOS the app uses CoreAudio automatically.

### Quick tour

Once the app opens:

| Button | What it does |
|--------|--------------|
| **🎹 Piano** | Open the virtual keyboard to play notes |
| **⏺ Record** | Record what you play onto the selected track |
| **▶ Play** | Play back your composition |
| **🎼 Compose** | Key detection, chord suggestions, circle of fifths |
| **✨ AI Instrument** | Create custom instruments from text descriptions |

Select a track in the timeline rows, click to set the playhead, enable **⏺ Record**, then play on the piano to capture a section.

If you hit a compile error or no sound, check the terminal output for errors and ensure your audio device is connected and not muted.

## Project Structure

```
src/
  main.rs              # Entry point
  app.rs               # Main DAW state and playback logic
  model/               # Project, tracks, notes, instruments
  audio/               # Polyphonic synth engine (cpal)
  music/               # Music theory: keys, chords, circle of fifths
  ai/                  # Instrument description → synth params
  ui/
    transport.rs       # Play/stop/record, BPM, time sig
    track_panel.rs     # Multi-track sidebar
    piano.rs           # Virtual piano window
    piano_roll.rs      # Timeline with vertical piano keys
    composition.rs     # Key detection, chord suggestions, circle of fifths
    instrument_designer.rs  # AI instrument creator
```

## Keyboard Shortcuts (Piano Window)

| Key | Note |
|-----|------|
| Z | C3 |
| S | C#3 |
| X | D3 |
| D | D#3 |
| C | E3 |
| V | F3 |
| G | G#3 |
| B | A3 |
| H | A#3 |
| N | B3 |
| M | C4 |
| J | C#4 |

## Roadmap / TODO

See [TODO.md](TODO.md) for planned features including:
- iPad/mobile interface for playing and uploading recordings
- Cloud AI integration for richer instrument design
- MIDI file import/export
- Sample-based instruments
- Undo/redo
- Project save/load

## Tech Stack

- **Rust** 2021 edition
- **eframe / egui** — cross-platform GUI
- **cpal** — cross-platform audio I/O
- **serde** — serialization (for future project save)
