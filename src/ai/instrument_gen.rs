//! Natural-language timbre design, fully local.
//!
//! "dark gritty 808 with a long tail" → pick a base sound (genre preset or sample) →
//! apply descriptive traits with intensity ("a bit", "very"), negation ("not too bright",
//! "less boomy") and comparatives ("warmer", "punchier"). The same parser powers
//! iterative refinement ("a bit darker and shorter"), so designing a sound feels like a
//! conversation instead of a form.

use crate::model::instrument::{
    default_params_for_kind, preset_nominal_params, CustomInstrument, GenrePreset, SampleKind,
    SynthParams,
};

/// What a designed instrument is built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
    Sample(SampleKind),
    Preset(GenrePreset),
}

impl Base {
    pub fn label(&self) -> &'static str {
        match self {
            Base::Sample(k) => k.label(),
            Base::Preset(p) => p.label(),
        }
    }

    pub fn params(&self) -> SynthParams {
        match self {
            Base::Sample(k) => default_params_for_kind(*k),
            Base::Preset(p) => preset_nominal_params(*p),
        }
    }

    pub fn of(params: &SynthParams) -> Base {
        match params.preset {
            Some(p) => Base::Preset(p),
            None => Base::Sample(params.sample_kind),
        }
    }

    pub fn all() -> Vec<Base> {
        let mut v: Vec<Base> = GenrePreset::all().iter().map(|p| Base::Preset(*p)).collect();
        v.extend(SampleKind::all().iter().map(|k| Base::Sample(*k)));
        v
    }

    pub fn is_bass(&self) -> bool {
        match self {
            Base::Preset(p) => p.is_bass(),
            Base::Sample(k) => *k == SampleKind::Bass,
        }
    }
}

/// Keywords → base sound, most specific first. Each entry: (phrases, base).
fn base_rules() -> Vec<(&'static [&'static str], Base)> {
    use GenrePreset::*;
    vec![
        (&["808"], Base::Preset(Bass808)),
        (&["reese"], Base::Preset(ReeseBass)),
        (&["garage bass", "2-step bass", "2step bass", "ukg bass", "speed garage bass"], Base::Preset(GarageBass)),
        (&["boom bap bass", "boombap bass", "hip hop bass", "hip-hop bass", "hiphop bass"], Base::Preset(BoomBapBass)),
        (&["sub bass", "sub-bass", "subbass", "sub"], Base::Preset(SubBass)),
        (&["organ stab", "garage organ", "m1 organ", "ukg organ", "house organ"], Base::Preset(GarageOrgan)),
        (&["chord stab", "stab", "2-step", "2step", "two step"], Base::Preset(TwoStepStab)),
        (&["rhodes", "electric piano", "e-piano", "epiano", "wurli", "wurlitzer", "neo soul", "neo-soul", "lofi keys", "lo-fi keys"], Base::Preset(DustyRhodes)),
        (&["bell", "bells", "glock", "glockenspiel", "music box", "celesta", "chime"], Base::Preset(TrapBell)),
        (&["pizzicato", "pizz", "plucked strings"], Base::Preset(Pizzicato)),
        (&["guitar", "acoustic", "nylon"], Base::Sample(SampleKind::Guitar)),
        (&["pluck", "plucky synth", "synth pluck"], Base::Preset(DarkPluck)),
        (&["strings", "string section", "violin", "violins", "cello", "viola", "orchestra", "orchestral", "cinematic"], Base::Preset(ChamberStrings)),
        (&["pad", "atmospheric", "ambient", "atmosphere", "liquid"], Base::Preset(LiquidPad)),
        (&["harp"], Base::Sample(SampleKind::Harp)),
        (&["piano", "grand", "upright", "felt"], Base::Sample(SampleKind::Piano)),
        (&["flute", "breath", "woodwind", "wind"], Base::Sample(SampleKind::Flute)),
        (&["brass", "trumpet", "horn", "horns", "trombone"], Base::Sample(SampleKind::Brass)),
        (&["organ", "church", "hammond"], Base::Sample(SampleKind::Organ)),
    ]
}

/// Genre words decide what a bare "bass"/"keys" (or nothing at all) should be.
fn genre_default(lower: &str, wants_bass: bool) -> Option<(Base, &'static str)> {
    use GenrePreset::*;
    let has = |w: &[&str]| w.iter().any(|x| contains_phrase(lower, x));
    if has(&["trap", "drill", "rage"]) {
        return Some((Base::Preset(if wants_bass { Bass808 } else { TrapBell }), "trap"));
    }
    if has(&["dnb", "d&b", "drum and bass", "drum & bass", "jungle", "neuro", "liquid funk"]) {
        return Some((Base::Preset(if wants_bass { ReeseBass } else { LiquidPad }), "drum & bass"));
    }
    if has(&["garage", "ukg", "speed garage", "bassline"]) {
        return Some((Base::Preset(if wants_bass { GarageBass } else { GarageOrgan }), "UK garage"));
    }
    if has(&["hip hop", "hip-hop", "hiphop", "boom bap", "boombap", "lofi", "lo-fi", "soul"]) {
        return Some((Base::Preset(if wants_bass { BoomBapBass } else { DustyRhodes }), "hip hop"));
    }
    if has(&["classical", "baroque", "romantic", "film score"]) {
        return Some((Base::Preset(ChamberStrings), "classical"));
    }
    None
}

fn contains_phrase(haystack: &str, phrase: &str) -> bool {
    // Whole-word match so "sub" doesn't fire on "subtle".
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(phrase) {
        let i = start + pos;
        let j = i + phrase.len();
        let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        let after_ok = j >= bytes.len() || !bytes[j].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = i + 1;
    }
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trait {
    Warm,
    Bright,
    Punchy,
    SlowAttack,
    Long,
    Short,
    Wide,
    Gritty,
    Clean,
    Vintage,
    Boomy,
    Thin,
}

impl Trait {
    fn describe(&self, positive: bool) -> &'static str {
        match (self, positive) {
            (Trait::Warm, true) => "warmer, darker tone",
            (Trait::Warm, false) => "less muffled",
            (Trait::Bright, true) => "brighter, more presence",
            (Trait::Bright, false) => "tamed top end",
            (Trait::Punchy, true) => "punchier attack",
            (Trait::Punchy, false) => "softer attack",
            (Trait::SlowAttack, true) => "slower swell-in",
            (Trait::SlowAttack, false) => "faster attack",
            (Trait::Long, true) => "longer tail",
            (Trait::Long, false) => "shorter tail",
            (Trait::Short, true) => "shorter, tighter",
            (Trait::Short, false) => "more sustain",
            (Trait::Wide, true) => "wider, more detuned",
            (Trait::Wide, false) => "narrower, more focused",
            (Trait::Gritty, true) => "more grit/saturation",
            (Trait::Gritty, false) => "less distortion",
            (Trait::Clean, true) => "cleaner",
            (Trait::Clean, false) => "a little dirt",
            (Trait::Vintage, true) => "dusty vintage character",
            (Trait::Vintage, false) => "more modern/clean",
            (Trait::Boomy, true) => "more low-end boom",
            (Trait::Boomy, false) => "less boomy",
            (Trait::Thin, true) => "lighter, thinner",
            (Trait::Thin, false) => "fuller body",
        }
    }
}

/// Word stems → trait. Stems match the start of a word ("warm" matches "warmer").
const TRAIT_STEMS: &[(&str, Trait)] = &[
    ("warm", Trait::Warm),
    ("mellow", Trait::Warm),
    ("round", Trait::Warm),
    ("smooth", Trait::Warm),
    ("velvet", Trait::Warm),
    ("cream", Trait::Warm),
    ("dark", Trait::Warm),
    ("muffl", Trait::Warm),
    ("mute", Trait::Warm),
    ("soft", Trait::Warm),
    ("intimate", Trait::Warm),
    ("bright", Trait::Bright),
    ("crisp", Trait::Bright),
    ("air", Trait::Bright),
    ("sparkl", Trait::Bright),
    ("shimmer", Trait::Bright),
    ("glass", Trait::Bright),
    ("shin", Trait::Bright),
    ("clear", Trait::Bright),
    ("brilliant", Trait::Bright),
    ("cutting", Trait::Bright),
    ("harsh", Trait::Bright),
    ("biting", Trait::Bright),
    ("punch", Trait::Punchy),
    ("snap", Trait::Punchy),
    ("tight", Trait::Punchy),
    ("percuss", Trait::Punchy),
    ("hard", Trait::Punchy),
    ("slow", Trait::SlowAttack),
    ("swell", Trait::SlowAttack),
    ("bowed", Trait::SlowAttack),
    ("gentle", Trait::SlowAttack),
    ("evolv", Trait::SlowAttack),
    ("long", Trait::Long),
    ("sustain", Trait::Long),
    ("lush", Trait::Long),
    ("spacious", Trait::Long),
    ("ring", Trait::Long),
    ("tail", Trait::Long),
    ("endless", Trait::Long),
    ("short", Trait::Short),
    ("staccato", Trait::Short),
    ("dry", Trait::Short),
    ("clip", Trait::Short),
    ("chop", Trait::Short),
    ("quick", Trait::Short),
    ("wide", Trait::Wide),
    ("chorus", Trait::Wide),
    ("detun", Trait::Wide),
    ("thick", Trait::Wide),
    ("fat", Trait::Wide),
    ("huge", Trait::Wide),
    ("massive", Trait::Wide),
    ("stereo", Trait::Wide),
    ("supersaw", Trait::Wide),
    ("grit", Trait::Gritty),
    ("dirt", Trait::Gritty),
    ("distort", Trait::Gritty),
    ("saturat", Trait::Gritty),
    ("crunch", Trait::Gritty),
    ("fuzz", Trait::Gritty),
    ("overdriv", Trait::Gritty),
    ("growl", Trait::Gritty),
    ("nast", Trait::Gritty),
    ("filth", Trait::Gritty),
    ("aggress", Trait::Gritty),
    ("clean", Trait::Clean),
    ("pure", Trait::Clean),
    ("pristine", Trait::Clean),
    ("vintage", Trait::Vintage),
    ("dust", Trait::Vintage),
    ("lofi", Trait::Vintage),
    ("tape", Trait::Vintage),
    ("retro", Trait::Vintage),
    ("worn", Trait::Vintage),
    ("vinyl", Trait::Vintage),
    ("wobbl", Trait::Vintage),
    ("boom", Trait::Boomy),
    ("rumbl", Trait::Boomy),
    ("deep", Trait::Boomy),
    ("heavy", Trait::Boomy),
    ("thin", Trait::Thin),
    ("small", Trait::Thin),
    ("tiny", Trait::Thin),
    ("light", Trait::Thin),
    ("delicate", Trait::Thin),
];

/// Words that are part of base names and must not also trigger traits.
const NOT_TRAITS: &[&str] = &["sub", "subtle", "bitter", "airy-fairy", "lighter"];

const BOOST: &[&str] = &["very", "super", "really", "extremely", "so", "ultra", "much", "way", "seriously", "insanely", "lots", "loads"];
const DAMPEN: &[&str] = &["bit", "slightly", "little", "somewhat", "kinda", "kind", "touch", "tad", "hint", "bit more", "mildly"];
const NEGATE: &[&str] = &["not", "no", "less", "without", "never", "don't", "dont", "isn't", "nothing"];

#[derive(Clone, Debug)]
pub struct TraitHit {
    pub word: String,
    pub effect: String,
    pub strength: f32,
}

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '-'))
        .filter(|t| !t.is_empty())
        .map(|t| t.trim_matches('-').to_string())
        .collect()
}

/// Find traits with their signed strength in a phrase.
fn parse_traits(text: &str) -> Vec<(Trait, f32, String)> {
    let toks = tokens(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i].as_str();
        // Phrase rules first: "soft/slow/gentle attack" vs "hard/fast/sharp attack".
        if t == "attack" || t == "attacks" {
            if let Some(prev) = i.checked_sub(1).map(|j| toks[j].as_str()) {
                let slow = ["soft", "slow", "gentle", "smooth", "lazy"].contains(&prev);
                let fast = ["hard", "fast", "sharp", "snappy", "quick", "punchy"].contains(&prev);
                if slow || fast {
                    // Undo the trait the previous word already produced.
                    if out.last().map(|(_, _, w): &(Trait, f32, String)| w == prev).unwrap_or(false) {
                        out.pop();
                    }
                    let tr = if slow { Trait::SlowAttack } else { Trait::Punchy };
                    out.push((tr, 1.0, format!("{prev} attack")));
                }
            }
            i += 1;
            continue;
        }
        if t == "fade" && toks.get(i + 1).map(|s| s == "in").unwrap_or(false) {
            out.push((Trait::SlowAttack, 1.0, "fade in".into()));
            i += 2;
            continue;
        }
        if NOT_TRAITS.contains(&t) || (t == "bit" && i > 0 && toks[i - 1] == "a") {
            i += 1;
            continue;
        }
        if let Some(&(_, tr)) = TRAIT_STEMS.iter().find(|(stem, _)| t.starts_with(stem)) {
            let mut strength = 1.0f32;
            let mut negated = false;
            let mut too = false;
            for back in 1..=3 {
                let Some(j) = i.checked_sub(back) else { break };
                let w = toks[j].as_str();
                if w == "too" {
                    too = true;
                } else if BOOST.contains(&w) {
                    strength *= 1.7;
                } else if DAMPEN.contains(&w) {
                    strength *= 0.5;
                } else if NEGATE.contains(&w) {
                    negated = !negated;
                } else if w == "and" || w == "but" || w == "with" || w == "," {
                    break;
                }
            }
            // "too bright" (a complaint) and "not too bright" both mean: a bit less bright.
            if too {
                negated = true;
            }
            // Negated traits are applied gently: "less bright" isn't "dark".
            if negated {
                strength = -strength * 0.7;
            }
            out.push((tr, strength.clamp(-2.0, 2.5), t.to_string()));
        }
        i += 1;
    }
    out
}

fn apply_trait(p: &mut SynthParams, tr: Trait, s: f32, base: Base) {
    let pos = s.max(0.0);
    let neg = (-s).max(0.0);
    match tr {
        Trait::Warm => {
            p.filter_cutoff -= 0.14 * s;
            p.brightness -= 0.14 * s;
        }
        Trait::Bright => {
            p.filter_cutoff += 0.14 * s;
            p.brightness += 0.16 * s;
        }
        Trait::Punchy => {
            p.attack *= 0.35_f32.powf(s);
            p.decay *= 0.8_f32.powf(s);
        }
        Trait::SlowAttack => {
            if s > 0.0 {
                p.attack = p.attack.max(0.08) * (1.0 + 2.5 * pos);
            } else {
                p.attack *= 0.4_f32.powf(neg);
            }
        }
        Trait::Long => {
            p.release *= 1.8_f32.powf(s);
            p.decay *= 1.4_f32.powf(s);
            if p.sustain > 0.05 || s < 0.0 {
                p.sustain += 0.12 * s;
            }
        }
        Trait::Short => {
            p.release *= 0.45_f32.powf(s);
            p.decay *= 0.6_f32.powf(s);
            p.sustain *= 0.6_f32.powf(s);
        }
        Trait::Wide => p.detune += 0.006 * s,
        Trait::Gritty => p.drive += 0.25 * s,
        Trait::Clean => {
            if s > 0.0 {
                p.drive *= 0.3;
                p.detune *= 0.6;
            } else {
                p.drive += 0.1 * neg;
            }
        }
        Trait::Vintage => {
            p.filter_cutoff -= 0.07 * s;
            p.detune += 0.003 * s;
            p.drive += 0.08 * s;
        }
        Trait::Boomy => {
            if base.is_bass() {
                p.decay *= 1.3_f32.powf(s);
                p.drive += 0.08 * s;
            }
            p.filter_cutoff -= 0.05 * s;
        }
        Trait::Thin => {
            p.filter_cutoff += 0.05 * s;
            p.drive *= 0.7_f32.powf(s);
            p.detune *= 0.7_f32.powf(s);
        }
    }
    clamp_params(p);
}

pub fn clamp_params(p: &mut SynthParams) {
    p.attack = p.attack.clamp(0.0005, 3.0);
    p.decay = p.decay.clamp(0.02, 6.0);
    p.sustain = p.sustain.clamp(0.0, 1.0);
    p.release = p.release.clamp(0.02, 6.0);
    p.brightness = p.brightness.clamp(0.0, 1.0);
    p.filter_cutoff = p.filter_cutoff.clamp(0.05, 1.0);
    p.detune = p.detune.clamp(0.0, 0.04);
    p.drive = p.drive.clamp(0.0, 1.0);
}

/// The result of reading a description.
#[derive(Clone, Debug)]
pub struct Interpretation {
    pub base: Base,
    pub base_reason: String,
    pub traits: Vec<TraitHit>,
    pub params: SynthParams,
    pub suggested_name: String,
}

/// Work out which base sound a description is asking for.
pub fn detect_base(description: &str) -> (Base, String) {
    let lower = description.to_lowercase();
    for (phrases, base) in base_rules() {
        if let Some(p) = phrases.iter().find(|p| contains_phrase(&lower, p)) {
            return (base, format!("heard “{p}”"));
        }
    }
    let wants_bass = contains_phrase(&lower, "bass") || contains_phrase(&lower, "bassline");
    if let Some((base, genre)) = genre_default(&lower, wants_bass) {
        return (base, format!("{genre} {}", if wants_bass { "bass" } else { "sound" }));
    }
    if wants_bass {
        return (Base::Sample(SampleKind::Bass), "heard “bass”".into());
    }
    if contains_phrase(&lower, "keys") {
        return (Base::Sample(SampleKind::Piano), "heard “keys”".into());
    }
    (Base::Sample(SampleKind::Piano), "default".into())
}

pub fn interpret(description: &str, forced: Option<Base>) -> Interpretation {
    let (base, base_reason) = match forced {
        Some(b) => (b, "chosen by you".to_string()),
        None => detect_base(description),
    };
    let mut params = base.params();
    let mut traits = Vec::new();
    for (tr, s, word) in parse_traits(description) {
        apply_trait(&mut params, tr, s, base);
        traits.push(TraitHit {
            word,
            effect: tr.describe(s > 0.0).to_string(),
            strength: s,
        });
    }
    let suggested_name = suggest_name(&traits, base);
    Interpretation {
        base,
        base_reason,
        traits,
        params,
        suggested_name,
    }
}

/// Apply a follow-up instruction ("a bit darker and less boomy") to existing params.
/// Returns the new params and a human summary of what changed.
pub fn refine(params: &SynthParams, instruction: &str) -> (SynthParams, Vec<String>) {
    let base = Base::of(params);
    let mut p = params.clone();
    let mut notes = Vec::new();
    for (tr, s, _) in parse_traits(instruction) {
        apply_trait(&mut p, tr, s, base);
        notes.push(tr.describe(s > 0.0).to_string());
    }
    // Allow switching the base mid-conversation ("make it a harp instead").
    let lower = instruction.to_lowercase();
    let switch = ["make it", "turn it into", "instead", "switch to", "use a", "as a"]
        .iter()
        .any(|w| lower.contains(w));
    if switch {
        let (new_base, _) = detect_base(instruction);
        if new_base != base && base_rules().iter().any(|(_, b)| *b == new_base) {
            let mut fresh = new_base.params();
            // Carry over the user's shaping relative to the old base's defaults.
            let old = base.params();
            fresh.filter_cutoff += p.filter_cutoff - old.filter_cutoff;
            fresh.brightness += p.brightness - old.brightness;
            fresh.drive += p.drive - old.drive;
            fresh.detune += p.detune - old.detune;
            clamp_params(&mut fresh);
            p = fresh;
            notes.push(format!("now built on {}", new_base.label()));
        }
    }
    if notes.is_empty() {
        notes.push("didn't catch a sound word — try warmer, brighter, punchier, longer, wider, grittier…".into());
    }
    (p, notes)
}

fn title(word: &str) -> String {
    let mut c = word.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn suggest_name(traits: &[TraitHit], base: Base) -> String {
    let mut words: Vec<String> = Vec::new();
    for t in traits.iter().filter(|t| t.strength > 0.0) {
        let w = title(&t.word);
        if !words.contains(&w) && words.len() < 2 && !t.word.contains(' ') {
            words.push(w);
        }
    }
    words.push(base.label().to_string());
    words.join(" ")
}

/// Plain-English summary of a designed instrument.
pub fn describe_params(p: &SynthParams) -> String {
    let base = Base::of(p);
    let nominal = base.params();
    let rel = |v: f32, n: f32| {
        if v > n * 1.15 + 0.01 {
            "↑"
        } else if v < n * 0.85 - 0.01 {
            "↓"
        } else {
            "·"
        }
    };
    format!(
        "Tone {:.0}% {}   Attack {:.0} ms {}   Tail {:.2} s {}   Width {:.0} ct {}   Grit {:.0}% {}",
        p.filter_cutoff * 100.0,
        rel(p.filter_cutoff, nominal.filter_cutoff),
        p.attack * 1000.0,
        rel(p.attack, nominal.attack),
        p.release,
        rel(p.release, nominal.release),
        p.detune * 1000.0,
        rel(p.detune, nominal.detune),
        p.drive * 100.0,
        rel(p.drive, nominal.drive),
    )
}

/// Back-compatible entry point: description + explicit sample base → custom instrument.
pub fn describe_to_instrument(description: &str, name: &str, base_kind: SampleKind) -> CustomInstrument {
    let (detected, _) = detect_base(description);
    // Respect an explicit instrument word; otherwise use the caller's base sample.
    let forced = if detected == Base::Sample(SampleKind::Piano) && !description.to_lowercase().contains("piano") {
        Some(Base::Sample(base_kind))
    } else {
        None
    };
    let it = interpret(description, forced);
    CustomInstrument {
        name: name.to_string(),
        description: description.to_string(),
        base_kind: it.params.sample_kind,
        params: it.params,
        sample: None,
    }
}

pub fn ai_summary(description: &str, instrument: &CustomInstrument) -> String {
    let it = interpret(description, Some(Base::of(&instrument.params)));
    let effects: Vec<String> = it.traits.iter().map(|t| t.effect.clone()).collect();
    format!(
        "Created \"{}\" on {}{}.\n{}",
        instrument.name,
        instrument.base_label(),
        if effects.is_empty() {
            String::new()
        } else {
            format!(" — {}", effects.join(", "))
        },
        describe_params(&instrument.params)
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

    #[test]
    fn picks_genre_presets() {
        assert_eq!(detect_base("dark gritty 808 with a long tail").0, Base::Preset(GenrePreset::Bass808));
        assert_eq!(detect_base("huge reese for jungle").0, Base::Preset(GenrePreset::ReeseBass));
        assert_eq!(detect_base("dusty lofi rhodes").0, Base::Preset(GenrePreset::DustyRhodes));
        assert_eq!(detect_base("bright UK garage organ stab").0, Base::Preset(GenrePreset::GarageOrgan));
        assert_eq!(detect_base("trap bass").0, Base::Preset(GenrePreset::Bass808));
        assert_eq!(detect_base("dnb bass, rolling").0, Base::Preset(GenrePreset::ReeseBass));
        assert_eq!(detect_base("soft felt piano").0, Base::Sample(SampleKind::Piano));
        assert_eq!(detect_base("lush cinematic strings").0, Base::Preset(GenrePreset::ChamberStrings));
        // "subtle" must not trigger sub bass.
        assert_ne!(detect_base("subtle bright bell").0, Base::Preset(GenrePreset::SubBass));
    }

    #[test]
    fn intensity_and_negation() {
        let base = Base::Preset(GenrePreset::DustyRhodes).params();
        let warm = interpret("warm rhodes", None).params;
        let very = interpret("very warm rhodes", None).params;
        let bit = interpret("a bit warm rhodes", None).params;
        assert!(very.filter_cutoff < warm.filter_cutoff);
        assert!(bit.filter_cutoff > warm.filter_cutoff);
        assert!(bit.filter_cutoff < base.filter_cutoff);
        let not_bright = interpret("rhodes, not too bright", None).params;
        assert!(not_bright.filter_cutoff < base.filter_cutoff);
    }

    #[test]
    fn soft_attack_is_slow_not_dark() {
        let it = interpret("piano with a soft attack", None);
        assert!(it.traits.iter().any(|t| t.effect.contains("swell")));
        assert!(!it.traits.iter().any(|t| t.effect.contains("darker")));
    }

    #[test]
    fn refine_is_relative() {
        let it = interpret("808", None);
        let (darker, notes) = refine(&it.params, "a bit darker and shorter");
        assert!(darker.filter_cutoff < it.params.filter_cutoff);
        assert!(darker.decay < it.params.decay);
        assert_eq!(notes.len(), 2);
        let (harp, _) = refine(&it.params, "make it a harp instead");
        assert_eq!(Base::of(&harp), Base::Sample(SampleKind::Harp));
    }
}
