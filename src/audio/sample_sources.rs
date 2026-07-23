use crate::model::instrument::SampleKind;

#[derive(Clone, Copy, Debug)]
pub enum SampleFileFormat {
    Wav,
    Flac,
}

#[derive(Clone, Copy, Debug)]
pub struct SampleSource {
    pub kind: SampleKind,
    pub url: &'static str,
    pub format: SampleFileFormat,
    pub root_midi: u8,
    /// SPDX or license name
    pub license: &'static str,
    pub attribution: &'static str,
    pub project_url: &'static str,
}

pub fn sources_for_kind(kind: SampleKind) -> SampleSource {
    *ALL_SOURCES
        .iter()
        .find(|s| s.kind == kind)
        .expect("every SampleKind has a source")
}

pub fn all_sources() -> &'static [SampleSource] {
    ALL_SOURCES
}

const ALL_SOURCES: &[SampleSource] = &[
    SampleSource {
        kind: SampleKind::Piano,
        url: "https://raw.githubusercontent.com/freepats/upright-piano-KW/main/samples/C4vL.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "Upright Piano KW — FreePats (Gonzalo & Roberto @ zenvoid.org), Kawai upright recorded 2017",
        project_url: "https://github.com/freepats/upright-piano-KW",
    },
    SampleSource {
        kind: SampleKind::Guitar,
        url: "https://raw.githubusercontent.com/freepats/spanish-classical-guitar/main/samples/C4.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "Spanish Classical Guitar — FreePats (roberto@zenvoid.org), recorded 2008",
        project_url: "https://github.com/freepats/spanish-classical-guitar",
    },
    SampleSource {
        kind: SampleKind::Bass,
        url: "https://raw.githubusercontent.com/freepats/electric-bass-YR/main/samples/finger/C.flac",
        format: SampleFileFormat::Flac,
        root_midi: 36,
        license: "CC0-1.0",
        attribution: "Electric Bass YR — FreePats (Andrea Biasior / Yamaha RBX), edited by zenvoid.org",
        project_url: "https://github.com/freepats/electric-bass-YR",
    },
    SampleSource {
        kind: SampleKind::Strings,
        url: "https://raw.githubusercontent.com/freepats/synth-strings-1/main/samples/C4.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "Synth Strings 1 — FreePats (roberto@zenvoid.org), ZynAddSubFX/Yoshimi",
        project_url: "https://github.com/freepats/synth-strings-1",
    },
    SampleSource {
        kind: SampleKind::Flute,
        url: "https://raw.githubusercontent.com/freepats/new-age/main/samples/C4.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "New Age (flute-like pad) — FreePats (roberto@zenvoid.org), ZynAddSubFX/Yoshimi",
        project_url: "https://github.com/freepats/new-age",
    },
    SampleSource {
        kind: SampleKind::Brass,
        url: "https://raw.githubusercontent.com/freepats/synth-brass-1/main/samples/C4.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "Synth Brass 1 — FreePats (roberto@zenvoid.org), ZynAddSubFX/Yoshimi",
        project_url: "https://github.com/freepats/synth-brass-1",
    },
    SampleSource {
        kind: SampleKind::Organ,
        url: "https://raw.githubusercontent.com/freepats/synth-calliope/main/samples/C4.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "Synth Calliope — FreePats (roberto@zenvoid.org), ZynAddSubFX/Yoshimi",
        project_url: "https://github.com/freepats/synth-calliope",
    },
    SampleSource {
        kind: SampleKind::Pad,
        url: "https://raw.githubusercontent.com/freepats/sweep-pad/main/samples/C4.flac",
        format: SampleFileFormat::Flac,
        root_midi: 60,
        license: "CC0-1.0",
        attribution: "Sweep Pad — FreePats (roberto@zenvoid.org), ZynAddSubFX/Yoshimi",
        project_url: "https://github.com/freepats/sweep-pad",
    },
];
