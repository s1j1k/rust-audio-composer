//! Remote keyboard: play the desktop app from an iPhone / iPad over the local network.
//!
//! ```text
//!  iPhone Safari / home-screen web app            this computer (Rust)
//!  ┌─────────────────────────────┐   Wi-Fi LAN   ┌──────────────────────────────────┐
//!  │ touch → velocity + pressure │──WebSocket──▶ │ network thread (tokio)           │
//!  │ (position, force, tap accel)│  JSON frames  │   └─ lock engine → note_on  ──▶ audio
//!  └─────────────────────────────┘               │   └─ event → UI (record, display)│
//!                                                └──────────────────────────────────┘
//! ```
//!
//! Notes go *straight* from the network thread into the audio engine, so latency is
//! Wi-Fi + one audio buffer — the UI frame rate is never in the path. The UI is told
//! afterwards (for recording and key highlighting). No cloud, no accounts: the computer
//! serves the web client itself and everything stays on the LAN.

pub mod net;
pub mod protocol;
mod tls;
mod web;

use crate::audio::engine::VelocityCurve;
use crate::audio::AudioEngine;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use protocol::{chain_layouts, ClientMsg, InstrumentChoice, Layout, ServerMsg};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

pub const DEFAULT_HTTP_PORT: u16 = 7878;
pub const DEFAULT_HTTPS_PORT: u16 = 7879;

/// Something that happened on a remote device, for the UI thread.
#[derive(Debug, Clone)]
pub enum RemoteEvent {
    NoteOn {
        device: u64,
        pitch: u8,
        velocity: f32,
        at: Instant,
    },
    NoteOff {
        device: u64,
        pitch: u8,
    },
    SelectInstrument(String),
    SetCurve(VelocityCurve),
    DevicesChanged,
}

#[derive(Debug, Clone)]
pub struct DeviceSnapshot {
    pub id: u64,
    pub name: String,
    pub addr: SocketAddr,
    pub width: f32,
    pub height: f32,
    pub rtt_ms: Option<f32>,
    pub notes: u64,
    pub layout: Option<Layout>,
    pub connected_for: std::time::Duration,
}

struct Device {
    id: u64,
    name: String,
    addr: SocketAddr,
    width: f32,
    height: f32,
    rtt_ms: Option<f32>,
    notes: u64,
    layout: Option<Layout>,
    since: Instant,
    tx: UnboundedSender<String>,
}

struct Settings {
    chain: bool,
    chain_start: u8,
    instruments: Vec<InstrumentChoice>,
    current: String,
    curve: VelocityCurve,
}

struct Shared {
    engine: Arc<Mutex<AudioEngine>>,
    events: mpsc::Sender<RemoteEvent>,
    devices: Mutex<Vec<Device>>,
    settings: Mutex<Settings>,
    next_id: AtomicU64,
    ca_der: Option<Vec<u8>>,
}

impl Shared {
    fn send_event(&self, e: RemoteEvent) {
        let _ = self.events.send(e);
    }

    /// Recompute chain layouts and push them to every device that changed.
    fn relayout(&self) {
        let (chain, start) = {
            let s = self.settings.lock().unwrap_or_else(|e| e.into_inner());
            (s.chain, s.chain_start)
        };
        let mut devices = self.devices.lock().unwrap_or_else(|e| e.into_inner());
        let ready: Vec<usize> = devices
            .iter()
            .enumerate()
            .filter(|(_, d)| d.width > 0.0)
            .map(|(i, _)| i)
            .collect();
        let layouts = if chain {
            let widths: Vec<f32> = ready.iter().map(|&i| devices[i].width).collect();
            chain_layouts(&widths, start)
        } else {
            Vec::new()
        };
        for (k, &i) in ready.iter().enumerate() {
            let d = &mut devices[i];
            let new = layouts.get(k).copied();
            if new != d.layout {
                d.layout = new;
                let msg = new.unwrap_or(Layout {
                    chained: false,
                    start: 0,
                    whites: 0,
                    key_w: 0.0,
                    index: 0,
                    count: 0,
                });
                let _ = d.tx.send(ServerMsg::Layout(msg).to_json());
            }
        }
    }

    fn broadcast(&self, msg: &ServerMsg) {
        let json = msg.to_json();
        for d in self.devices.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            let _ = d.tx.send(json.clone());
        }
    }
}

pub struct RemoteServer {
    shared: Arc<Shared>,
    events_rx: mpsc::Receiver<RemoteEvent>,
    runtime: Option<tokio::runtime::Runtime>,
    handles: Vec<axum_server::Handle>,
    pub http_port: u16,
    pub https_port: Option<u16>,
    pub https_error: Option<String>,
    pub addresses: Vec<IpAddr>,
    pub hostname: Option<String>,
}

#[derive(Clone)]
struct NoDelay;

impl<S: Send + 'static> axum_server::accept::Accept<tokio::net::TcpStream, S> for NoDelay {
    type Stream = tokio::net::TcpStream;
    type Service = S;
    type Future = std::future::Ready<std::io::Result<(Self::Stream, Self::Service)>>;

    fn accept(&self, stream: tokio::net::TcpStream, service: S) -> Self::Future {
        // Nagle's algorithm would batch small note frames for up to ~40 ms. Disable it.
        let _ = stream.set_nodelay(true);
        std::future::ready(Ok((stream, service)))
    }
}

fn bind(port: u16) -> Result<std::net::TcpListener, String> {
    let listener = std::net::TcpListener::bind(("0.0.0.0", port))
        .map_err(|e| format!("port {port}: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok(listener)
}

impl RemoteServer {
    pub fn start(engine: Arc<Mutex<AudioEngine>>, http_port: u16, https_port: u16) -> Result<Self, String> {
        let addresses = net::lan_addresses();
        let hostname = net::hostname();
        let (events_tx, events_rx) = mpsc::channel();

        let tls = tls::load_or_create(&addresses, hostname.as_deref());
        let (tls_config, ca_der, https_error) = match tls {
            Ok(t) => (Some(t.server_config), Some(t.ca_der), None),
            Err(e) => (None, None, Some(e)),
        };

        let curve = engine
            .lock()
            .map(|e| e.velocity_curve)
            .unwrap_or_default();
        let shared = Arc::new(Shared {
            engine,
            events: events_tx,
            devices: Mutex::new(Vec::new()),
            settings: Mutex::new(Settings {
                chain: false,
                chain_start: 36,
                instruments: Vec::new(),
                current: String::new(),
                curve,
            }),
            next_id: AtomicU64::new(1),
            ca_der,
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("remote-keys")
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;

        let app = Router::new()
            .route("/ws", get(ws_handler))
            .route("/ca.crt", get(ca_handler))
            .merge(web::routes())
            .with_state(shared.clone());

        let http_listener = bind(http_port)?;
        let mut handles = Vec::new();

        let handle = axum_server::Handle::new();
        handles.push(handle.clone());
        {
            let app = app.clone();
            runtime.spawn(async move {
                let _ = axum_server::from_tcp(http_listener)
                    .acceptor(NoDelay)
                    .handle(handle)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await;
            });
        }

        let mut https_error = https_error;
        let mut https_bound = None;
        if let Some(config) = tls_config {
            match bind(https_port) {
                Ok(listener) => {
                    let handle = axum_server::Handle::new();
                    handles.push(handle.clone());
                    https_bound = Some(https_port);
                    let app = app.clone();
                    runtime.spawn(async move {
                        let rustls = axum_server::tls_rustls::RustlsConfig::from_config(config);
                        let acceptor =
                            axum_server::tls_rustls::RustlsAcceptor::new(rustls).acceptor(NoDelay);
                        let _ = axum_server::from_tcp(listener)
                            .acceptor(acceptor)
                            .handle(handle)
                            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                            .await;
                    });
                }
                Err(e) => https_error = Some(e),
            }
        }

        Ok(Self {
            shared,
            events_rx,
            runtime: Some(runtime),
            handles,
            http_port,
            https_port: https_bound,
            https_error,
            addresses,
            hostname,
        })
    }

    pub fn http_urls(&self) -> Vec<String> {
        self.addresses
            .iter()
            .map(|ip| format!("http://{}:{}/", ip, self.http_port))
            .collect()
    }

    pub fn https_urls(&self) -> Vec<String> {
        match self.https_port {
            Some(port) => self
                .addresses
                .iter()
                .map(|ip| format!("https://{}:{}/", ip, port))
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn poll_events(&self) -> Vec<RemoteEvent> {
        self.events_rx.try_iter().collect()
    }

    pub fn devices(&self) -> Vec<DeviceSnapshot> {
        self.shared
            .devices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|d| DeviceSnapshot {
                id: d.id,
                name: d.name.clone(),
                addr: d.addr,
                width: d.width,
                height: d.height,
                rtt_ms: d.rtt_ms,
                notes: d.notes,
                layout: d.layout,
                connected_for: d.since.elapsed(),
            })
            .collect()
    }

    /// Keep the phone's instrument menu in sync with the desktop.
    pub fn set_instruments(&self, instruments: Vec<InstrumentChoice>, current: &str) {
        let changed = {
            let mut s = self.shared.settings.lock().unwrap_or_else(|e| e.into_inner());
            let names_changed = s.instruments.len() != instruments.len()
                || s
                    .instruments
                    .iter()
                    .zip(&instruments)
                    .any(|(a, b)| a.name != b.name);
            let cur_changed = s.current != current;
            if names_changed || cur_changed {
                s.instruments = instruments;
                s.current = current.to_string();
                Some(ServerMsg::Instruments {
                    instruments: s.instruments.clone(),
                    current: s.current.clone(),
                })
            } else {
                None
            }
        };
        if let Some(msg) = changed {
            self.shared.broadcast(&msg);
        }
    }

    pub fn set_curve(&self, curve: VelocityCurve) {
        let changed = {
            let mut s = self.shared.settings.lock().unwrap_or_else(|e| e.into_inner());
            let c = s.curve != curve;
            s.curve = curve;
            c
        };
        if changed {
            self.shared.broadcast(&ServerMsg::Curve {
                name: curve.key().to_string(),
            });
        }
    }

    pub fn chain(&self) -> (bool, u8) {
        let s = self.shared.settings.lock().unwrap_or_else(|e| e.into_inner());
        (s.chain, s.chain_start)
    }

    pub fn set_chain(&self, on: bool, start: u8) {
        {
            let mut s = self.shared.settings.lock().unwrap_or_else(|e| e.into_inner());
            s.chain = on;
            s.chain_start = start;
        }
        self.shared.relayout();
    }

    /// Move a device left (-1) or right (+1) in the chained keyboard.
    pub fn move_device(&self, id: u64, delta: i32) {
        {
            let mut devices = self.shared.devices.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(i) = devices.iter().position(|d| d.id == id) {
                let j = (i as i32 + delta).clamp(0, devices.len() as i32 - 1) as usize;
                devices.swap(i, j);
            }
        }
        self.shared.relayout();
    }

    pub fn identify(&self, id: u64) {
        let devices = self.shared.devices.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(d) = devices.iter().find(|d| d.id == id) {
            let _ = d.tx.send(ServerMsg::Identify.to_json());
        }
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        for h in &self.handles {
            h.shutdown();
        }
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn ca_handler(State(shared): State<Arc<Shared>>) -> impl IntoResponse {
    match &shared.ca_der {
        Some(der) => (
            [
                (axum::http::header::CONTENT_TYPE, "application/x-x509-ca-cert"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=\"RustAudioComposer-LocalCA.crt\"",
                ),
            ],
            der.clone(),
        )
            .into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "HTTPS is not enabled").into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(shared): State<Arc<Shared>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, shared, addr))
}

async fn handle_socket(socket: WebSocket, shared: Arc<Shared>, addr: SocketAddr) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let id = shared.next_id.fetch_add(1, Ordering::Relaxed);

    let writer = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    {
        let s = shared.settings.lock().unwrap_or_else(|e| e.into_inner());
        let sustain = shared.engine.lock().map(|e| e.synth.sustain()).unwrap_or(false);
        let _ = tx.send(
            ServerMsg::Welcome {
                id,
                instruments: s.instruments.clone(),
                current: s.current.clone(),
                curve: s.curve.key().to_string(),
                sustain,
            }
            .to_json(),
        );
    }
    shared
        .devices
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Device {
            id,
            name: format!("Device {id}"),
            addr,
            width: 0.0,
            height: 0.0,
            rtt_ms: None,
            notes: 0,
            layout: None,
            since: Instant::now(),
            tx: tx.clone(),
        });
    shared.send_event(RemoteEvent::DevicesChanged);

    let mut held: HashSet<u8> = HashSet::new();
    let mut sustain = false;

    while let Some(Ok(msg)) = stream.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(msg) = serde_json::from_str::<ClientMsg>(text.as_str()) else {
            continue;
        };
        match msg {
            ClientMsg::On { n, v } => {
                if n > 127 {
                    continue;
                }
                let vel = match shared.engine.lock() {
                    Ok(mut eng) => {
                        if held.contains(&n) {
                            eng.stop_note(n);
                        }
                        eng.live_note_on(n, v)
                    }
                    Err(_) => continue,
                };
                held.insert(n);
                shared.send_event(RemoteEvent::NoteOn {
                    device: id,
                    pitch: n,
                    velocity: vel,
                    at: Instant::now(),
                });
                if let Some(d) = shared
                    .devices
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .iter_mut()
                    .find(|d| d.id == id)
                {
                    d.notes += 1;
                }
            }
            ClientMsg::Off { n } => {
                if held.remove(&n) {
                    if let Ok(mut eng) = shared.engine.lock() {
                        eng.stop_note(n);
                    }
                    shared.send_event(RemoteEvent::NoteOff { device: id, pitch: n });
                }
            }
            ClientMsg::P { n, p } => {
                if held.contains(&n) {
                    if let Ok(mut eng) = shared.engine.lock() {
                        eng.set_note_pressure(n, p);
                    }
                }
            }
            ClientMsg::Sus { on } => {
                sustain = on;
                if let Ok(mut eng) = shared.engine.lock() {
                    eng.set_sustain(on);
                }
            }
            ClientMsg::Ping { ts } => {
                let _ = tx.send(ServerMsg::Pong { ts }.to_json());
            }
            ClientMsg::Rtt { ms } => {
                if let Some(d) = shared
                    .devices
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .iter_mut()
                    .find(|d| d.id == id)
                {
                    d.rtt_ms = Some(ms);
                }
            }
            ClientMsg::Hello { name, w, h } => {
                {
                    let mut devices = shared.devices.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(d) = devices.iter_mut().find(|d| d.id == id) {
                        d.name = name.chars().take(40).collect();
                        d.width = w;
                        d.height = h;
                        d.layout = None; // force a fresh layout message
                    }
                }
                shared.relayout();
                shared.send_event(RemoteEvent::DevicesChanged);
            }
            ClientMsg::Inst { name } => {
                shared.send_event(RemoteEvent::SelectInstrument(name));
            }
            ClientMsg::Curve { name } => {
                if let Some(c) = VelocityCurve::from_key(&name) {
                    if let Ok(mut eng) = shared.engine.lock() {
                        eng.velocity_curve = c;
                    }
                    shared.send_event(RemoteEvent::SetCurve(c));
                }
            }
        }
    }

    // Disconnected (closed tab, phone locked, Wi-Fi dropped): never leave stuck notes.
    if let Ok(mut eng) = shared.engine.lock() {
        for &n in &held {
            eng.stop_note(n);
        }
        if sustain {
            eng.set_sustain(false);
        }
    }
    for n in held {
        shared.send_event(RemoteEvent::NoteOff { device: id, pitch: n });
    }
    shared
        .devices
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|d| d.id != id);
    shared.relayout();
    shared.send_event(RemoteEvent::DevicesChanged);
    writer.abort();
}
