use crate::audio::sample_sources::SampleFileFormat;
use crate::model::drum::DrumKind;

#[derive(Clone, Copy, Debug)]
pub struct DrumSource {
    pub kind: DrumKind,
    pub url: &'static str,
    pub format: SampleFileFormat,
    pub license: &'static str,
    pub attribution: &'static str,
    pub project_url: &'static str,
}

pub fn source_for_kind(kind: DrumKind) -> DrumSource {
    *ALL_SOURCES
        .iter()
        .find(|s| s.kind == kind)
        .expect("every DrumKind has a source")
}

pub fn all_drum_sources() -> &'static [DrumSource] {
    ALL_SOURCES
}

const ALL_SOURCES: &[DrumSource] = &[
    DrumSource {
        kind: DrumKind::Kick,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/Kick04.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::Snare,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/Snare09.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::ClosedHat,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/ClosedHiHat01-01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::OpenHat,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/OpenHiHat02-01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::LowTom,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/LowTom02-01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::MidTom,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/MidTom02-01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::HighTom,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/HighTom02-01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::Clap,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/Clap01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::Cymbal,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/Cymbal01-01.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
    DrumSource {
        kind: DrumKind::Shaker,
        url: "https://raw.githubusercontent.com/freepats/synthesizer-percussion/main/samples/Shaker04.flac",
        format: SampleFileFormat::Flac,
        license: "CC0-1.0",
        attribution: "Synthesizer Percussion — FreePats (roberto@zenvoid.org), Yoshimi/Geonkick",
        project_url: "https://github.com/freepats/synthesizer-percussion",
    },
];
