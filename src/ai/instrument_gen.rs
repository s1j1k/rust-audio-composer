use crate::model::instrument::{CustomInstrument, SampleKind, SynthParams, default_params_for_kind};

pub fn describe_to_instrument(
    description: &str,
    name: &str,
    base_kind: SampleKind,
) -> CustomInstrument {
    let lower = description.to_lowercase();
    let mut params = default_params_for_kind(base_kind);

    if lower.contains("piano") || lower.contains("keys") {
        params.sample_kind = SampleKind::Piano;
    } else if lower.contains("guitar") || lower.contains("pluck") {
        params.sample_kind = SampleKind::Guitar;
    } else if lower.contains("bass") || lower.contains("deep") || lower.contains("sub") {
        params.sample_kind = SampleKind::Bass;
    } else if lower.contains("string") || lower.contains("violin") || lower.contains("cello") {
        params.sample_kind = SampleKind::Strings;
    } else if lower.contains("flute") || lower.contains("wind") || lower.contains("breath") {
        params.sample_kind = SampleKind::Flute;
    } else if lower.contains("brass") || lower.contains("trumpet") || lower.contains("horn") {
        params.sample_kind = SampleKind::Brass;
    } else if lower.contains("organ") || lower.contains("church") {
        params.sample_kind = SampleKind::Organ;
    } else if lower.contains("pad") || lower.contains("ambient") || lower.contains("atmospheric") {
        params.sample_kind = SampleKind::Pad;
    } else {
        params.sample_kind = base_kind;
    }

    if lower.contains("warm") || lower.contains("soft") || lower.contains("mellow") {
        params.brightness = params.brightness.min(0.35);
        params.filter_cutoff = params.filter_cutoff.min(0.55);
        params.attack = params.attack.max(0.05);
    }
    if lower.contains("harsh") || lower.contains("aggressive") || lower.contains("bright") {
        params.brightness = params.brightness.max(0.75);
        params.filter_cutoff = params.filter_cutoff.max(0.8);
    }
    if lower.contains("fast") || lower.contains("snappy") || lower.contains("percussive") {
        params.attack = 0.001;
        params.decay = 0.1;
        params.sustain = 0.0;
    }
    if lower.contains("slow") || lower.contains("gentle") || lower.contains("smooth") {
        params.attack = params.attack.max(0.2);
        params.release = params.release.max(0.5);
    }
    if lower.contains("short") {
        params.release = 0.05;
        params.sustain = 0.0;
    }
    if lower.contains("long") || lower.contains("sustain") {
        params.release = 1.0;
        params.sustain = 0.8;
    }

    CustomInstrument {
        name: name.to_string(),
        description: description.to_string(),
        base_kind: params.sample_kind,
        params,
    }
}

pub fn ai_summary(_description: &str, instrument: &CustomInstrument) -> String {
    format!(
        "Created \"{}\" based on {} sample.\n\
         Attack: {:.0}ms, Decay: {:.0}ms, Sustain: {:.0}%, Release: {:.0}ms\n\
         Brightness: {:.0}%, Filter: {:.0}%\n\
         Saved instruments are added to your library automatically.",
        instrument.name,
        instrument.base_kind.label(),
        instrument.params.attack * 1000.0,
        instrument.params.decay * 1000.0,
        instrument.params.sustain * 100.0,
        instrument.params.release * 1000.0,
        instrument.params.brightness * 100.0,
        instrument.params.filter_cutoff * 100.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guitar_description() {
        let inst = describe_to_instrument("warm acoustic guitar pluck", "My Guitar", SampleKind::Guitar);
        assert_eq!(inst.base_kind, SampleKind::Guitar);
        assert!(inst.params.decay > 0.3);
    }
}
