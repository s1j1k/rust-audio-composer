# Rust Audio Composer

A desktop Digital Audio Workstation (DAW) built in Rust with egui. Play piano and guitar sounds, record multi-track compositions, and get AI-assisted composition help.

## Features

### Virtual Piano
- Clickable piano keyboard window (🎹 button in transport bar)
- Computer keyboard mapping: **Z-M** for white keys, **S/D/G/H/J** for black keys
- Polyphonic playback with ADSR envelopes

### Instruments
Built-in presets:
- **Piano** — triangle wave with natural decay
- **Guitar** — plucked string character
- **Bass** — warm saw wave
- **Strings** — slow-attack pad

Each track can have its own instrument.

### Piano Roll / Timeline
- Vertical piano keys on the left (C3–B5 range)
- Horizontal timeline with beat and bar grid lines
- Click the grid to place notes on the selected track
- Red playhead shows current playback position
- Notes displayed as colored blocks per track

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
Describe a sound in plain language and the app generates a playable synth preset:
- Example: *"warm mellow piano with gentle attack"*
- Preview before saving
- Saved instruments appear in the track instrument picker

Currently uses local keyword analysis. Cloud LLM integration is planned (see TODO).

## Running

```bash
cargo run
```

Requires a working audio output device (macOS CoreAudio, etc.).

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
