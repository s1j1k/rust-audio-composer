# TODO — Rust Audio Composer

## High Priority

- [ ] **Project save/load** — serialize project (tracks, notes, BPM, instruments) to JSON
- [ ] **Undo/redo** — note editing history
- [ ] **Note editing in piano roll** — drag to move/resize notes, delete with key press
- [ ] **Proper note-off recording** — record actual note duration instead of fixed 1-beat length

## Mobile / iPad Interface

- [x] **Remote keyboard (MVP)** — iPhone/iPad web app served by the desktop over local Wi-Fi, WebSocket straight into the audio engine
- [x] **Touch-optimized piano** — multi-touch, glissando, sustain pedal, strike-position velocity, slide-for-pressure, optional accelerometer tap strength (HTTPS)
- [x] **Chain several devices into one long keyboard** (v1: uniform key size, manual ordering)
- [ ] Physical-size calibration across iPhone/iPad models; bezel compensation
- [ ] WebRTC DataChannel transport (LAN-only P2P) and A/B latency measurement
- [ ] Native iOS app (cloud-built) with Bluetooth LE MIDI for Wi-Fi-free playing
- [ ] Show the playhead / tempo on the phone; upload recordings from iPad

## AI / Composition

- [ ] **Cloud LLM integration** — connect OpenAI/Anthropic API for richer instrument descriptions
- [ ] **AI melody suggestions** — generate melodic ideas based on detected key
- [ ] **Smarter key detection** — weight by note duration and beat position
- [ ] **Chord name display** — show full chord names (e.g. "Am7") on piano roll

## Audio Engine

- [x] **Sample-based instruments** — multi-velocity piano and harp (CC0), genre synth presets and drum kits
- [ ] **Lower latency** — audio buffer-size setting, exclusive mode/ASIO, lock-free note queue into the audio thread
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
