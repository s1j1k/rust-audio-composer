//! Wire protocol between the phone/tablet web client and the desktop app.
//!
//! Small JSON text frames over a WebSocket on the local network. JSON keeps the client
//! trivial and debuggable; at ~30 bytes per note event the encoding cost is microseconds,
//! far below Wi-Fi jitter, so a binary format would not be audibly faster.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ClientMsg {
    /// First message (and on resize): who the device is and how big its screen is (CSS px).
    Hello {
        name: String,
        w: f32,
        h: f32,
    },
    /// Note on. `v` is the raw touch strength 0..1 (the desktop applies the velocity curve).
    On { n: u8, v: f32 },
    /// Note off.
    Off { n: u8 },
    /// Continuous pressure / aftertouch 0..1 on a held note.
    P { n: u8, p: f32 },
    /// Sustain pedal.
    Sus { on: bool },
    /// Latency probe; echoed straight back as `Pong`.
    Ping { ts: f64 },
    /// Client-measured round-trip time, reported for the desktop's device list.
    Rtt { ms: f32 },
    /// Switch the selected track's instrument.
    Inst { name: String },
    /// Change the live velocity curve.
    Curve { name: String },
    /// REC button on the phone: start (with count-in) or stop a take on the desktop.
    Rec { on: bool },
    /// DJ: a channel control moved. `p` is an `FxParam` key ("filter", "eq_low",
    /// "volume", ...), `v` its value in that parameter's range.
    Dj { ch: u8, p: String, v: f32 },
    /// DJ: crossfader moved (-1 = A, +1 = B).
    Xf { v: f32 },
    /// DJ: put a channel on crossfader side "a", "thru" or "b".
    DjSide { ch: u8, side: String },
    /// DJ: load a track (index) onto a channel, or empty it with `null`.
    DjLoad { ch: u8, track: Option<usize> },
    /// DJ: mute / unmute a channel.
    DjMute { ch: u8, on: bool },
    /// DJ: arm / disarm recording knob moves as automation.
    DjWrite { on: bool },
    /// Transport play / pause.
    Play { on: bool },
    /// Start / stop recording a live mixdown on the desktop.
    MixRec { on: bool },
}

/// One DJ mixer channel as the phone sees it.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct DjChannel {
    pub ch: u8,
    /// Track index on this channel (`None` = empty).
    pub track: Option<usize>,
    pub name: String,
    /// Track icon key and its Phosphor glyph (the phone loads the same icon font).
    pub icon: String,
    pub glyph: String,
    /// "#rrggbb".
    pub color: String,
    /// "a" / "thru" / "b".
    pub side: String,
    pub muted: bool,
    /// Current value of each control, keyed by `FxParam` key.
    pub vals: Vec<(String, f32)>,
    /// Keys of controls that follow automation.
    pub auto: Vec<String>,
}

/// A track the phone can load onto a channel.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct DjTrack {
    pub i: usize,
    pub name: String,
    pub icon: String,
    pub glyph: String,
    pub color: String,
}

/// Everything the phone's DJ page shows (sent when it changes, at most ~20 times a second).
#[derive(Debug, Serialize, Clone, PartialEq, Default)]
pub struct DjSnapshot {
    pub channels: Vec<DjChannel>,
    pub tracks: Vec<DjTrack>,
    pub xf: f32,
    pub xf_auto: bool,
    pub playing: bool,
    pub write: bool,
    pub mixrec: bool,
    /// Seconds recorded so far when `mixrec` is on.
    pub mixrec_secs: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct InstrumentChoice {
    pub group: String,
    pub name: String,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
pub struct Layout {
    pub chained: bool,
    pub start: u8,
    pub whites: u8,
    pub key_w: f32,
    pub index: usize,
    pub count: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ServerMsg {
    Welcome {
        id: u64,
        instruments: Vec<InstrumentChoice>,
        current: String,
        curve: String,
        sustain: bool,
    },
    Pong { ts: f64 },
    Layout(Layout),
    Instruments {
        instruments: Vec<InstrumentChoice>,
        current: String,
    },
    Curve { name: String },
    Identify,
    /// Desktop transport state for the phone's REC button: "off", "count" (count-in, with
    /// beats left) or "rec".
    Rec { state: String, beats: u32 },
    /// DJ mixer state.
    Dj(DjSnapshot),
    /// VU meters: post-fader peak per DJ channel (4) and the master, linear 0..1+.
    Meters { l: Vec<f32>, m: f32 },
}

impl ServerMsg {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

pub fn is_black_key(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

/// The note `n` white keys after `start` (start itself counts as the first white key).
pub fn advance_whites(start: u8, n: u32) -> u8 {
    let mut note = start;
    let mut seen = 0;
    while note < 127 {
        if !is_black_key(note) {
            if seen == n {
                return note;
            }
            seen += 1;
        }
        note += 1;
    }
    127
}

/// Comfortable white-key width on glass, in CSS px.
const TARGET_KEY_W: f32 = 54.0;

/// Chain several screens into one long keyboard. Keys are sized for the *smallest* screen
/// and every device gets as many white keys as fit at that size, consecutively.
/// `widths` are in device order (left → right).
pub fn chain_layouts(widths: &[f32], start: u8) -> Vec<Layout> {
    if widths.is_empty() {
        return Vec::new();
    }
    let min_w = widths.iter().cloned().fold(f32::INFINITY, f32::min).max(200.0);
    let whites_on_smallest = (min_w / TARGET_KEY_W).round().clamp(7.0, 24.0);
    let key_w = min_w / whites_on_smallest;
    let mut note = if is_black_key(start) { start + 1 } else { start };
    let count = widths.len();
    widths
        .iter()
        .enumerate()
        .map(|(index, &w)| {
            let whites = ((w.max(200.0) / key_w).floor() as u32).clamp(7, 52);
            let layout = Layout {
                chained: true,
                start: note,
                whites: whites as u8,
                key_w,
                index,
                count,
            };
            note = advance_whites(note, whites);
            layout
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_messages() {
        let m: ClientMsg = serde_json::from_str(r#"{"t":"on","n":60,"v":0.5}"#).unwrap();
        assert_eq!(m, ClientMsg::On { n: 60, v: 0.5 });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"sus","on":true}"#).unwrap();
        assert_eq!(m, ClientMsg::Sus { on: true });
        let m: ClientMsg =
            serde_json::from_str(r#"{"t":"hello","name":"iPhone","w":844,"h":390}"#).unwrap();
        assert!(matches!(m, ClientMsg::Hello { .. }));
        // Phone REC button, and the state the desktop sends back (what app.js reads).
        let m: ClientMsg = serde_json::from_str(r#"{"t":"rec","on":true}"#).unwrap();
        assert_eq!(m, ClientMsg::Rec { on: true });
        let json = ServerMsg::Rec { state: "count".into(), beats: 3 }.to_json();
        assert_eq!(json, r#"{"t":"rec","state":"count","beats":3}"#);
    }

    #[test]
    fn parses_dj_messages() {
        let m: ClientMsg = serde_json::from_str(r#"{"t":"dj","ch":2,"p":"filter","v":-0.5}"#).unwrap();
        assert_eq!(m, ClientMsg::Dj { ch: 2, p: "filter".into(), v: -0.5 });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"xf","v":1.0}"#).unwrap();
        assert_eq!(m, ClientMsg::Xf { v: 1.0 });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"dj_side","ch":0,"side":"b"}"#).unwrap();
        assert_eq!(m, ClientMsg::DjSide { ch: 0, side: "b".into() });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"dj_load","ch":1,"track":null}"#).unwrap();
        assert_eq!(m, ClientMsg::DjLoad { ch: 1, track: None });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"dj_load","ch":1,"track":3}"#).unwrap();
        assert_eq!(m, ClientMsg::DjLoad { ch: 1, track: Some(3) });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"dj_mute","ch":3,"on":true}"#).unwrap();
        assert_eq!(m, ClientMsg::DjMute { ch: 3, on: true });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"dj_write","on":true}"#).unwrap();
        assert_eq!(m, ClientMsg::DjWrite { on: true });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"play","on":false}"#).unwrap();
        assert_eq!(m, ClientMsg::Play { on: false });
        let m: ClientMsg = serde_json::from_str(r#"{"t":"mix_rec","on":true}"#).unwrap();
        assert_eq!(m, ClientMsg::MixRec { on: true });
        // Every parameter key the phone sends is one the desktop understands.
        for key in ["volume", "eq_high", "eq_mid", "eq_low", "filter", "echo", "reverb"] {
            assert!(crate::model::FxParam::from_key(key).is_some(), "{key}");
        }
    }

    #[test]
    fn dj_server_messages_are_flat() {
        let snap = DjSnapshot {
            channels: vec![DjChannel {
                ch: 0,
                track: Some(1),
                name: "Bass".into(),
                icon: "sine".into(),
                glyph: "x".into(),
                color: "#ff8000".into(),
                side: "a".into(),
                muted: false,
                vals: vec![("filter".into(), -0.25)],
                auto: vec!["filter".into()],
            }],
            xf: -0.5,
            playing: true,
            ..Default::default()
        };
        let json = ServerMsg::Dj(snap).to_json();
        assert!(json.starts_with(r#"{"t":"dj","channels":[{"ch":0,"track":1,"#), "{json}");
        assert!(json.contains(r#""vals":[["filter",-0.25]]"#), "{json}");
        assert!(json.contains(r#""xf":-0.5"#));
        let json = ServerMsg::Meters { l: vec![0.5, 0.0], m: 0.25 }.to_json();
        assert_eq!(json, r#"{"t":"meters","l":[0.5,0.0],"m":0.25}"#);
    }

    #[test]
    fn layout_message_is_flat() {
        let json = ServerMsg::Layout(Layout {
            chained: true,
            start: 48,
            whites: 14,
            key_w: 50.0,
            index: 0,
            count: 1,
        })
        .to_json();
        assert!(json.contains(r#""t":"layout""#));
        assert!(json.contains(r#""start":48"#));
    }

    #[test]
    fn advance_whites_counts_white_keys() {
        assert_eq!(advance_whites(60, 0), 60); // C4
        assert_eq!(advance_whites(60, 1), 62); // D4
        assert_eq!(advance_whites(60, 7), 72); // C5
    }

    #[test]
    fn chain_is_contiguous_and_uniform() {
        // iPhone (844) + iPad (1180) in landscape.
        let l = chain_layouts(&[844.0, 1180.0], 36);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].key_w, l[1].key_w);
        assert!(l[1].whites > l[0].whites);
        assert_eq!(l[1].start, advance_whites(l[0].start, l[0].whites as u32));
        assert!(!is_black_key(l[1].start));
    }
}
