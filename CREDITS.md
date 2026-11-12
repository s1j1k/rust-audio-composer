# Third-Party Sample Credits

Rust Audio Composer uses **free and open sample libraries** for built-in instruments.  
These licenses allow use in **commercial applications** (including monetized products).

> **Note:** CC0 places works in the public domain — attribution is not legally required —  
> but we credit creators below for transparency and good practice.

## Primary source: [FreePats](https://freepats.zenvoid.org/)

All built-in instrument samples are sourced from the [FreePats project](https://github.com/freepats)  
GitHub repositories, published under **Creative Commons Zero (CC0 1.0)**.

| Instrument | Repository | Creator / notes |
|------------|------------|-----------------|
| Piano | [upright-piano-KW](https://github.com/freepats/upright-piano-KW) | Gonzalo & Roberto (zenvoid.org); Kawai upright, 2017 |
| Guitar | [spanish-classical-guitar](https://github.com/freepats/spanish-classical-guitar) | Roberto (zenvoid.org); recorded 2008 |
| Bass | [electric-bass-YR](https://github.com/freepats/electric-bass-YR) | Andrea Biasior; Yamaha RBX electric bass |
| Strings | [synth-strings-1](https://github.com/freepats/synth-strings-1) | Roberto (zenvoid.org); ZynAddSubFX / Yoshimi |
| Flute | [new-age](https://github.com/freepats/new-age) | Roberto (zenvoid.org); soft synth (used as flute-like tone) |
| Brass | [synth-brass-1](https://github.com/freepats/synth-brass-1) | Roberto (zenvoid.org); ZynAddSubFX / Yoshimi |
| Organ | [synth-calliope](https://github.com/freepats/synth-calliope) | Roberto (zenvoid.org); ZynAddSubFX / Yoshimi |
| Pad | [sweep-pad](https://github.com/freepats/sweep-pad) | Roberto (zenvoid.org); ZynAddSubFX / Yoshimi |

### Drum kit: [Synthesizer Percussion](https://github.com/freepats/synthesizer-percussion)

All beat sequencer samples (kick, snare, hi-hats, toms, clap, cymbal, shaker) from this kit — **CC0 1.0**, commercial use permitted.

| Drum | Sample file |
|------|-------------|
| Kick | Kick04.flac |
| Snare | Snare09.flac |
| Closed hi-hat | ClosedHiHat01-01.flac |
| Open hi-hat | OpenHiHat02-01.flac |
| Toms | Low/Mid/HighTom02-01.flac |
| Clap | Clap01.flac |
| Cymbal | Cymbal01-01.flac |
| Shaker | Shaker04.flac |

Cached at: `~/rust-audio-composer/drums/`

### CC0 1.0 summary

You may copy, modify, distribute, and use these samples for any purpose, including commercial  
software, without asking permission. See: https://creativecommons.org/publicdomain/zero/1.0/

### Multi-velocity piano (FreePats Upright Piano KW)

The Piano also loads **soft (`vL`) and hard (`vH`) layers** at every C and F♯ from C1 to C8
from the same CC0 repository, crossfaded by velocity. Cached at `~/rust-audio-composer/multisamples/piano/`.

## Versilian Community Sample Library (VCSL)

| Instrument | Files | License |
|------------|-------|---------|
| Concert Harp | `Chordophones/Composite Chordophones/Concert Harp/KSHarp_*_mf1.wav` and `*_f1.wav` (D2, A2, G3, D4, A4, G5, D6, plus C5 as the single-sample fallback) | [CC0 1.0](https://github.com/sgossner/VCSL/blob/master/LICENSE) |

Source: [github.com/sgossner/VCSL](https://github.com/sgossner/VCSL) by Versilian Studios / Sam Gossner.
Cached at `~/rust-audio-composer/multisamples/concert-harp/`.

## Synthesized instruments

The genre presets (808 Bass, Reese Bass, Dusty Rhodes, Garage Organ, Chamber Strings, etc.)
and the genre drum kits (Trap 808, Boom Bap, D&B/Jungle, UK Garage) are synthesized in real
time by this project's own code. They contain no third-party audio.

## Procedural fallback

If samples cannot be downloaded (offline / network error), the app synthesizes placeholder  
waveforms locally. These are original to this project and carry no third-party license.

## Cached files

Downloaded samples are stored at:

`~/rust-audio-composer/samples/`

Each instrument includes a `{name}.attribution.txt` with license and source URL.

## App icon

`assets/icon.png`, `assets/icon-512.png` and `assets/icon.ico` are a placeholder: the letters
"Ch" set in French Script MT (a font bundled with Windows/Office) on staff lines. The font's
licence should be checked before the icon is used to promote the app publicly, or the
letters redrawn by hand (planned, along with custom hand-drawn track icons).

## Future libraries (not yet integrated)

These are documented for planned expansion. Verify license before integration:

| Library | License | Notes |
|---------|---------|-------|
| [VCSL](https://github.com/sgossner/VCSL) | CC0 | More instruments (Steinway B grand, marimba, vibraphone; large files) |
| [Sonatina Symphonic Orchestra](https://sso.mattiaswestlund.net/) | CC BY 4.0 | Requires attribution in commercial apps |

## Contact

FreePats contributions: freepats@zenvoid.org  
Project issues: https://github.com/s1j1k/rust-audio-composer
