use crate::model::instrument::{CustomInstrument, SynthParams, Waveform};

pub fn describe_to_instrument(description: &str, name: &str) -> CustomInstrument {
    let lower = description.to_lowercase();
    let mut params = SynthParams::default();

    if lower.contains("piano") || lower.contains("keys") {
        params.waveform = Waveform::Triangle;
        params.attack = 0.005;
        params.decay = 0.3;
        params.sustain = 0.2;
        params.release = 0.4;
    } else if lower.contains("guitar") || lower.contains("pluck") || lower.contains("string") {
        params.waveform = Waveform::Triangle;
        params.attack = 0.001;
        params.decay = 0.5;
        params.sustain = 0.05;
        params.release = 0.3;
        params.detune = 0.002;
    } else if lower.contains("bass") || lower.contains("deep") || lower.contains("sub") {
        params.waveform = Waveform::Saw;
        params.attack = 0.01;
        params.decay = 0.2;
        params.sustain = 0.6;
        params.release = 0.15;
        params.brightness = 0.2;
    } else if lower.contains("pad") || lower.contains("ambient") || lower.contains("atmospheric") {
        params.waveform = Waveform::Saw;
        params.attack = 0.3;
        params.decay = 0.4;
        params.sustain = 0.8;
        params.release = 1.0;
    } else if lower.contains("lead") || lower.contains("bright") || lower.contains("sharp") {
        params.waveform = Waveform::Square;
        params.attack = 0.01;
        params.decay = 0.1;
        params.sustain = 0.7;
        params.release = 0.2;
        params.brightness = 0.9;
    }

    if lower.contains("warm") || lower.contains("soft") || lower.contains("mellow") {
        params.brightness = params.brightness.min(0.3);
        params.attack = params.attack.max(0.05);
        if !lower.contains("guitar") && !lower.contains("lead") && !lower.contains("harsh") {
            params.waveform = Waveform::Sine;
        }
    }
    if lower.contains("harsh") || lower.contains("aggressive") || lower.contains("distort") {
        params.waveform = Waveform::Square;
        params.brightness = 1.0;
        params.sustain = 0.9;
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
        params,
    }
}

pub fn ai_summary(_description: &str, instrument: &CustomInstrument) -> String {
    format!(
        "Created \"{}\" from your description.\n\
         Waveform: {:?}, Attack: {:.0}ms, Decay: {:.0}ms, Sustain: {:.0}%, Release: {:.0}ms\n\
         Tip: Try playing it with the piano keyboard. Adjust by describing different qualities \
         (e.g. \"warmer\", \"more plucky\", \"longer sustain\").",
        instrument.name,
        instrument.params.waveform,
        instrument.params.attack * 1000.0,
        instrument.params.decay * 1000.0,
        instrument.params.sustain * 100.0,
        instrument.params.release * 1000.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guitar_description() {
        let inst = describe_to_instrument("warm acoustic guitar pluck", "My Guitar");
        assert_eq!(inst.params.waveform, Waveform::Triangle);
        assert!(inst.params.decay > 0.3);
    }
}
