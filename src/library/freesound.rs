//! Minimal Freesound APIv2 client: search single-note instrument samples, list a pack,
//! and download the high-quality preview of a sound.
//!
//! Uses the user's own API key (https://freesound.org/apiv2/apply). Freesound's terms
//! allow free use for non-commercial purposes and forbid sharing keys, so the key is
//! never bundled with the app: each user pastes theirs, and it's stored only on their
//! computer. Original-quality downloads need OAuth2 sign-in; the HQ previews (OGG,
//! ~192 kbps) need only the key and are plenty for one-note samples.

use serde_json::Value;

const API: &str = "https://freesound.org/apiv2";
const FIELDS: &str =
    "id,name,username,license,pack,duration,previews,note_midi,note_name,note_confidence,url";

#[derive(Clone, Debug, PartialEq)]
pub struct FsSound {
    pub id: u64,
    pub name: String,
    pub username: String,
    /// "Creative Commons 0", "Attribution", "Attribution NonCommercial", …
    pub license: String,
    pub pack_id: Option<u64>,
    pub duration: f32,
    pub preview_url: String,
    /// Freesound's estimated note, when its analysis found one.
    pub note_midi: Option<u8>,
    pub note_confidence: f32,
    /// Freesound page for the sound (for credits).
    pub url: String,
}

/// Only licences that allow using a sound in songs you release or sell are ever
/// searched for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LicenseFilter {
    /// Creative Commons 0 (public domain) only.
    Cc0,
    /// CC0 or Attribution: fine for music you sell (with credit).
    CommercialOk,
}

#[derive(Clone, Debug)]
pub struct SearchOptions {
    pub query: String,
    pub license: LicenseFilter,
    /// Only sounds where Freesound detected a note.
    pub detected_note_only: bool,
    pub page: u32,
}

pub struct SearchPage {
    pub sounds: Vec<FsSound>,
    pub total: u64,
}

/// The `filter` parameter: Freesound's "Instrument samples" category, single events,
/// no loops, short files.
pub fn build_filter(opts: &SearchOptions) -> String {
    let mut f = vec![
        "category:\"Instrument samples\"".to_string(),
        "loopable:false".to_string(),
        "duration:[0.05 TO 12]".to_string(),
    ];
    if opts.detected_note_only {
        f.push("note_confidence:[0.5 TO *]".to_string());
    }
    match opts.license {
        LicenseFilter::Cc0 => f.push("license:\"Creative Commons 0\"".to_string()),
        LicenseFilter::CommercialOk => {
            f.push("license:(\"Creative Commons 0\" OR \"Attribution\")".to_string())
        }
    }
    f.join(" ")
}

pub fn search(api_key: &str, opts: &SearchOptions) -> Result<SearchPage, String> {
    let url = format!(
        "{API}/search/?query={}&filter={}&fields={FIELDS}&page_size=40&page={}",
        encode(opts.query.trim()),
        encode(&build_filter(opts)),
        opts.page.max(1)
    );
    let json = get_json(&url, api_key)?;
    Ok(SearchPage {
        sounds: song_safe(parse_results(&json)),
        total: json.get("count").and_then(Value::as_u64).unwrap_or(0),
    })
}

/// Every sound in a pack (up to 150), e.g. all the notes of one recorded instrument.
pub fn pack_sounds(api_key: &str, pack_id: u64) -> Result<Vec<FsSound>, String> {
    let url = format!("{API}/packs/{pack_id}/sounds/?fields={FIELDS}&page_size=150");
    Ok(song_safe(parse_results(&get_json(&url, api_key)?)))
}

/// Drop anything whose licence doesn't allow use in songs (packs mix licences, and the
/// search filter is belt; this is braces).
fn song_safe(sounds: Vec<FsSound>) -> Vec<FsSound> {
    sounds
        .into_iter()
        .filter(|s| s.license == "Creative Commons 0" || s.license == "Attribution")
        .collect()
}

/// The HQ OGG preview. Tried without the key first (previews are usually public), then
/// with it.
pub fn download_preview(api_key: &str, url: &str) -> Result<Vec<u8>, String> {
    match fetch(url, None) {
        Ok(b) => Ok(b),
        Err(_) => fetch(url, Some(api_key)),
    }
}

fn get_json(url: &str, api_key: &str) -> Result<Value, String> {
    if api_key.trim().is_empty() {
        return Err("Add your Freesound API key first.".to_string());
    }
    let bytes = fetch(url, Some(api_key))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Unexpected reply from Freesound ({e})"))
}

fn fetch(url: &str, api_key: Option<&str>) -> Result<Vec<u8>, String> {
    let mut req = ureq::get(url).set("User-Agent", "ChorDAW/0.1");
    if let Some(k) = api_key {
        req = req.set("Authorization", &format!("Token {}", k.trim()));
    }
    match req.call() {
        Ok(resp) => {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut resp.into_reader(), &mut buf)
                .map_err(|e| e.to_string())?;
            Ok(buf)
        }
        Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => {
            Err("Freesound didn't accept the API key.".to_string())
        }
        Err(ureq::Error::Status(429, _)) => {
            Err("Freesound's rate limit was reached; try again in a minute.".to_string())
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("Freesound returned an error ({code}).")),
        Err(e) => Err(format!("Couldn't reach Freesound ({e}).")),
    }
}

pub fn parse_results(json: &Value) -> Vec<FsSound> {
    json.get("results")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(parse_sound).collect())
        .unwrap_or_default()
}

fn parse_sound(v: &Value) -> Option<FsSound> {
    let previews = v.get("previews")?;
    let preview_url = previews
        .get("preview-hq-ogg")
        .or_else(|| previews.get("preview-hq-mp3"))
        .and_then(Value::as_str)?
        .to_string();
    let note_midi = v
        .get("note_midi")
        .and_then(Value::as_i64)
        .filter(|m| (0..=127).contains(m))
        .map(|m| m as u8);
    Some(FsSound {
        id: v.get("id")?.as_u64()?,
        name: v.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
        username: v.get("username").and_then(Value::as_str).unwrap_or("").to_string(),
        license: license_name(v.get("license").and_then(Value::as_str).unwrap_or("")),
        pack_id: v.get("pack").and_then(Value::as_str).and_then(pack_id_from_uri),
        duration: v.get("duration").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        preview_url,
        note_midi,
        note_confidence: v.get("note_confidence").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        url: v.get("url").and_then(Value::as_str).unwrap_or("").to_string(),
    })
}

/// Search results give the licence as a name or as a creativecommons.org URL.
fn license_name(raw: &str) -> String {
    let l = raw.to_ascii_lowercase();
    if l.contains("publicdomain/zero") || l.contains("creative commons 0") {
        "Creative Commons 0".to_string()
    } else if l.contains("by-nc") || l.contains("noncommercial") {
        "Attribution NonCommercial".to_string()
    } else if l.contains("/by/") || l == "attribution" {
        "Attribution".to_string()
    } else if l.contains("sampling+") {
        "Sampling+".to_string()
    } else {
        raw.to_string()
    }
}

/// ".../apiv2/packs/9678/" → 9678
pub fn pack_id_from_uri(uri: &str) -> Option<u64> {
    uri.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

/// Percent-encode a query-string value.
pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_results() {
        let json: Value = serde_json::from_str(
            r#"{"count": 2, "results": [
                {"id": 1, "name": "Cello A3.wav", "username": "u", "license": "http://creativecommons.org/publicdomain/zero/1.0/",
                 "pack": "https://freesound.org/apiv2/packs/9678/", "duration": 3.2,
                 "previews": {"preview-hq-ogg": "https://cdn.freesound.org/x.ogg"},
                 "note_midi": 57, "note_confidence": 0.93, "url": "https://freesound.org/s/1/"},
                {"id": 2, "name": "no previews"}
            ]}"#,
        )
        .unwrap();
        let s = parse_results(&json);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].pack_id, Some(9678));
        assert_eq!(s[0].note_midi, Some(57));
        assert_eq!(s[0].license, "Creative Commons 0");
    }

    #[test]
    fn filter_and_encoding() {
        let opts = SearchOptions {
            query: "felt piano".into(),
            license: LicenseFilter::Cc0,
            detected_note_only: true,
            page: 1,
        };
        let f = build_filter(&opts);
        assert!(f.contains("category:\"Instrument samples\""));
        assert!(f.contains("loopable:false"));
        assert!(f.contains("license:\"Creative Commons 0\""));
        assert_eq!(encode("a b\"c"), "a%20b%22c");
    }
}
