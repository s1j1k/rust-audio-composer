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
