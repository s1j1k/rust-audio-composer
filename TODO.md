# TODO — Rust Audio Composer

## High Priority

- [ ] **Project save/load** — serialize project (tracks, notes, BPM, instruments) to JSON
- [ ] **Undo/redo** — note editing history
- [ ] **Note editing in piano roll** — drag to move/resize notes, delete with key press
- [ ] **Proper note-off recording** — record actual note duration instead of fixed 1-beat length

## Mobile / iPad Interface

- [ ] **Mobile companion app** — iPad app to play virtual instruments remotely
- [ ] **Network sync** — upload recordings from iPad to desktop DAW over Wi-Fi
- [ ] **Touch-optimized piano** — multi-touch polyphonic keyboard for iPad
- [ ] **Shared project format** — common JSON/MIDI format between desktop and mobile

The desktop app is the primary target for now. Mobile is documented here as a future milestone.

## AI / Composition

- [ ] **Cloud LLM integration** — connect OpenAI/Anthropic API for richer instrument descriptions
- [ ] **AI melody suggestions** — generate melodic ideas based on detected key
- [ ] **Smarter key detection** — weight by note duration and beat position
- [ ] **Chord name display** — show full chord names (e.g. "Am7") on piano roll

## Audio Engine

- [ ] **Sample-based instruments** — load WAV/FLAC samples for realistic piano/guitar
- [ ] **Effects chain** — reverb, delay, EQ per track
- [ ] **Audio export** — render project to WAV/MP3
- [ ] **MIDI input** — support external MIDI keyboards via midir crate
- [ ] **MIDI file import/export**

## UI / UX

- [ ] **Zoomable piano roll** — horizontal and vertical zoom
- [ ] **Snap to grid** — quantize note placement to 1/4, 1/8, 1/16 beats
- [ ] **Track colors and renaming** — inline track name editing
- [ ] **Waveform preview** — show rendered waveform per track
- [ ] **Dark/light theme toggle**

## Infrastructure

- [ ] **Cross-platform CI** — GitHub Actions for macOS, Linux, Windows
- [ ] **Unit and integration tests** — audio engine, music theory, project serialization
- [ ] **Plugin architecture** — VST/AU instrument plugins (long-term)
