// Composer Keys — wireless touch keyboard for Rust Audio Composer.
// Plain JS, no build step. Talks to the desktop app over a WebSocket on the local network.
'use strict';

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const isBlack = (n) => [1, 3, 6, 8, 10].includes(n % 12);
const noteName = (n) => NAMES[n % 12] + (Math.floor(n / 12) - 1);
const clamp = (x, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));
const $ = (id) => document.getElementById(id);

// ---------- persisted settings ----------
const store = {
  get(k, d) { try { const v = localStorage.getItem('ck.' + k); return v === null ? d : JSON.parse(v); } catch (_) { return d; } },
  set(k, v) { try { localStorage.setItem('ck.' + k, JSON.stringify(v)); } catch (_) { /* private mode */ } },
};

function defaultName() {
  const ua = navigator.userAgent;
  if (/iPad/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1)) return 'iPad';
  if (/iPhone/.test(ua)) return 'iPhone';
  if (/Android/.test(ua)) return 'Android';
  return 'Browser';
}

const settings = {
  base: store.get('base', 48),          // first note when not chained (C3)
  keyW: store.get('keyW', 56),          // preferred white-key width, CSS px
  curve: store.get('curve', 'natural'),
  sensitivity: store.get('sensitivity', 1),
  usePosition: store.get('usePosition', true),
  usePressure: store.get('usePressure', true),
  useGlide: store.get('useGlide', true),
  motion: false,
  name: store.get('name', defaultName()),
};

// ---------- connection ----------
let ws = null;
let connected = false;
let retryMs = 400;
let rtt = null;
let chain = null; // layout from the desktop when devices are chained

function send(obj) {
  if (ws && ws.readyState === 1) ws.send(JSON.stringify(obj));
}

function connect() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  ws = new WebSocket(`${proto}://${location.host}/ws`);
  ws.onopen = () => {
    connected = true;
    retryMs = 400;
    sendHello();
    updateStatus();
  };
  ws.onclose = () => {
    connected = false;
    rtt = null;
    releaseAll(false);
    updateStatus();
    setTimeout(connect, retryMs);
    retryMs = Math.min(retryMs * 1.7, 4000);
  };
  ws.onerror = () => { try { ws.close(); } catch (_) {} };
  ws.onmessage = (e) => {
    let m;
    try { m = JSON.parse(e.data); } catch (_) { return; }
    switch (m.t) {
      case 'welcome':
        setInstruments(m.instruments, m.current);
        setCurve(m.curve, false);
        break;
      case 'instruments':
        setInstruments(m.instruments, m.current);
        break;
      case 'curve':
        setCurve(m.name, false);
        break;
      case 'pong': {
        const sample = performance.now() - m.ts;
        rtt = rtt === null ? sample : rtt * 0.8 + sample * 0.2;
        updateStatus();
        break;
      }
      case 'layout':
        chain = m.chained ? m : null;
        buildKeys();
        break;
      case 'identify':
        flash();
        break;
      case 'rec':
        setRec(m.state, m.beats);
        break;
    }
  };
}

function sendHello() {
  const r = $('keys').getBoundingClientRect();
  send({ t: 'hello', name: settings.name, w: Math.round(r.width), h: Math.round(r.height) });
}

// Frequent tiny pings measure latency *and* keep the Wi-Fi radio awake: phones nap their
// radio between packets to save power, which otherwise adds 50–200 ms spikes.
setInterval(() => send({ t: 'ping', ts: performance.now() }), 250);
setInterval(() => { if (rtt !== null) send({ t: 'rtt', ms: Math.round(rtt * 10) / 10 }); }, 2000);

function updateStatus() {
  const dot = document.querySelector('#status .dot');
  const text = $('status-text');
  dot.className = 'dot';
  if (!connected) {
    text.textContent = 'Offline';
    return;
  }
  if (rtt === null) {
    text.textContent = 'Connected';
    dot.classList.add('ok');
    return;
  }
  const oneWay = rtt / 2;
  text.textContent = `${oneWay < 10 ? oneWay.toFixed(1) : Math.round(oneWay)} ms`;
  dot.classList.add(oneWay < 15 ? 'ok' : oneWay < 40 ? 'warn' : 'bad');
  $('latency-note').textContent =
    `Network: ~${Math.round(oneWay)} ms one-way (round trip ${Math.round(rtt)} ms). ` +
    'Under ~10 ms feels instant. If it jumps around, move closer to the router or use 5 GHz Wi-Fi.';
}

// ---------- instruments & curve ----------
function setInstruments(list, current) {
  const sel = $('instrument');
  sel.innerHTML = '';
  const groups = new Map();
  for (const it of list || []) {
    if (!groups.has(it.group)) groups.set(it.group, []);
    groups.get(it.group).push(it.name);
  }
  for (const [group, names] of groups) {
    const og = document.createElement('optgroup');
    og.label = group;
    for (const n of names) {
      const o = document.createElement('option');
      o.value = n;
      o.textContent = n;
      og.appendChild(o);
    }
    sel.appendChild(og);
  }
  if (current) sel.value = current;
}

$('instrument').addEventListener('change', (e) => send({ t: 'inst', name: e.target.value }));

function setCurve(name, notify) {
  if (!name) return;
  settings.curve = name;
  store.set('curve', name);
  $('curve').value = name;
  if (notify) send({ t: 'curve', name });
}

// ---------- record button (mirrors the desktop transport) ----------
let recState = 'off';
function setRec(state, beats) {
  recState = state || 'off';
  const b = $('rec');
  b.classList.toggle('count', recState === 'count');
  b.classList.toggle('on', recState === 'rec');
  $('rec-text').textContent =
    recState === 'count' ? `${beats || ''}…` : recState === 'rec' ? 'STOP' : 'REC';
}
$('rec').addEventListener('click', () => {
  // Tap to start (1-bar count-in on the desktop), tap again to stop / cancel.
  send({ t: 'rec', on: recState === 'off' });
});

// ---------- keyboard geometry ----------
let keys = [];      // { note, black, x, w, h, el }
let whiteW = 56;
let blackH = 0;
let keysRect = null;

function buildKeys() {
  const host = $('keys');
  host.innerHTML = '';
  keysRect = host.getBoundingClientRect();
  const width = keysRect.width;
  let start;
  let whites;
  if (chain) {
    start = chain.start;
    whites = chain.whites;
  } else {
    start = isBlack(settings.base) ? settings.base - 1 : settings.base;
    whites = Math.max(5, Math.round(width / settings.keyW));
  }
  whiteW = width / whites;
  blackH = keysRect.height * 0.62;
  const blackW = whiteW * 0.6;
  keys = [];
  let note = start;
  let i = 0;
  while (i < whites && note <= 127) {
    if (!isBlack(note)) {
      const x = i * whiteW;
      keys.push({ note, black: false, x, w: whiteW, h: keysRect.height });
      const sharp = note + 1;
      if (isBlack(sharp) && sharp <= 127 && (i < whites - 1)) {
        keys.push({ note: sharp, black: true, x: x + whiteW - blackW / 2, w: blackW, h: blackH });
      }
      i++;
    }
    note++;
  }
  for (const k of keys) {
    const el = document.createElement('div');
    el.className = 'key ' + (k.black ? 'black' : 'white');
    el.style.left = k.x + 'px';
    el.style.width = k.w + 'px';
    if (!k.black && k.note % 12 === 0) {
      const label = document.createElement('div');
      label.className = 'name';
      label.textContent = noteName(k.note);
      el.appendChild(label);
    }
    host.appendChild(el);
    k.el = el;
  }
  const last = keys.length ? keys[keys.length - 1].note : start;
  $('oct-label').textContent = noteName(start);
  const chainLabel = $('chain-label');
  if (chain) {
    chainLabel.textContent = `Chain ${chain.index + 1}/${chain.count} · ${noteName(start)}–${noteName(last)}`;
    chainLabel.classList.remove('hidden');
    $('octave-group').classList.add('hidden');
    $('zoom-group').classList.add('hidden');
  } else {
    chainLabel.classList.add('hidden');
    $('octave-group').classList.remove('hidden');
    $('zoom-group').classList.remove('hidden');
  }
  // Re-highlight anything still held after a rebuild.
  for (const t of touches.values()) if (t.note !== undefined) paint(t.note, t.vel);
}

function keyAt(x, y) {
  // x, y relative to #keys. Black keys sit on top, so test them first.
  if (y < blackH) {
    for (const k of keys) if (k.black && x >= k.x && x < k.x + k.w) return k;
  }
  for (const k of keys) if (!k.black && x >= k.x && x < k.x + k.w) return k;
  return null;
}

const byNote = (n) => keys.find((k) => k.note === n);

function paint(note, vel) {
  const k = byNote(note);
  if (!k) return;
  if (vel === null) {
    k.el.style.background = '';
    return;
  }
  const v = clamp(vel);
  k.el.style.background = k.black
    ? `hsl(215, 70%, ${22 + v * 30}%)`
    : `hsl(215, 85%, ${88 - v * 30}%)`;
}

// ---------- touch strength ----------
// Velocity on glass is inferred from whatever the hardware gives us, blended:
//   • strike position along the key (always available; like playing nearer the key's front)
//   • Touch.force (3D Touch iPhones, Apple Pencil on iPad)
//   • contact size (radiusX — Android; ignored where it never varies)
//   • tap strength from the accelerometer (needs HTTPS + permission on iOS)
let forceSeen = false;
let radiusMin = Infinity;
let radiusMax = 0;

const motion = { samples: [], peakRef: 2.5, ok: false };

function onMotion(e) {
  const a = e.acceleration;
  let mag;
  if (a && a.x !== null) {
    mag = Math.hypot(a.x, a.y, a.z);
  } else if (e.accelerationIncludingGravity) {
    const g = e.accelerationIncludingGravity;
    mag = Math.abs(Math.hypot(g.x, g.y, g.z) - 9.81);
  } else {
    return;
  }
  const now = performance.now();
  motion.samples.push([now, mag]);
  while (motion.samples.length && now - motion.samples[0][0] > 400) motion.samples.shift();
  motion.ok = true;
  flushPending(false);
}

function motionStrength(touchTime) {
  const s = motion.samples;
  if (!s.length) return null;
  const before = s.filter(([t]) => t < touchTime - 40).map(([, m]) => m).sort((a, b) => a - b);
  const baseline = before.length ? before[Math.floor(before.length / 2)] : 0;
  let peak = 0;
  for (const [t, m] of s) if (t >= touchTime - 40) peak = Math.max(peak, m - baseline);
  // Adapt to how hard this person plays (and whether the phone is held or on a table).
  motion.peakRef = Math.max(0.6, motion.peakRef * 0.995, peak);
  return clamp(Math.log1p(peak / 0.04) / Math.log1p(motion.peakRef / 0.04));
}

function strikeVelocity(touch, key, yRel, touchTime) {
  let sum = 0;
  let wsum = 0;
  const add = (v, w) => { if (v !== null && !Number.isNaN(v)) { sum += v * w; wsum += w; } };

  if (settings.usePosition) add(0.25 + 0.75 * Math.pow(clamp(yRel), 0.9), 1);

  const f = touch.force;
  const stylus = touch.touchType === 'stylus';
  if (f > 0.01 && f < 0.99) forceSeen = true;
  if ((forceSeen || stylus) && f > 0) add(clamp(f * 1.25), 2);

  const r = touch.radiusX || 0;
  if (r > 0) {
    radiusMin = Math.min(radiusMin, r);
    radiusMax = Math.max(radiusMax, r);
    if (radiusMax - radiusMin > 2) add(clamp((r - radiusMin) / (radiusMax - radiusMin)), 1.5);
  }

  if (settings.motion && motion.ok) add(motionStrength(touchTime), 2.5);

  const raw = wsum ? sum / wsum : 0.75;
  return clamp(raw * settings.sensitivity, 0.02, 1);
}

// ---------- touches → notes ----------
const touches = new Map(); // id -> { note, vel, startY, key, pending, pedal }
const pending = [];        // notes waiting a few ms for the accelerometer spike

function noteOn(t, touch, key, yRel, now) {
  t.note = key.note;
  t.key = key;
  t.startYRel = yRel;
  t.pressure = 0;
  if (settings.motion && motion.ok) {
    // Wait for the next accelerometer sample (≤ ~16 ms) so the tap's spike is included.
    t.pending = { touch: { force: touch.force, touchType: touch.touchType, radiusX: touch.radiusX }, yRel, time: now };
    pending.push(t);
    setTimeout(() => flushPending(true), 24);
    paint(key.note, 0.6);
    return;
  }
  t.vel = strikeVelocity(touch, key, yRel, now);
  send({ t: 'on', n: key.note, v: Math.round(t.vel * 1000) / 1000 });
  paint(key.note, t.vel);
}

function flushPending(force) {
  const now = performance.now();
  for (let i = pending.length - 1; i >= 0; i--) {
    const t = pending[i];
    if (!t.pending) { pending.splice(i, 1); continue; }
    if (force || now - t.pending.time > 6) {
      t.vel = strikeVelocity(t.pending.touch, t.key, t.pending.yRel, t.pending.time);
      send({ t: 'on', n: t.note, v: Math.round(t.vel * 1000) / 1000 });
      paint(t.note, t.vel);
      t.pending = null;
      pending.splice(i, 1);
    }
  }
}

function noteOff(t) {
  if (t.note === undefined) return;
  if (t.pending) flushPending(true);
  send({ t: 'off', n: t.note });
  paint(t.note, null);
  t.note = undefined;
}

function setPedal(down) {
  $('pedal').classList.toggle('down', down);
  send({ t: 'sus', on: down });
}

function local(touch) {
  return { x: touch.clientX - keysRect.left, y: touch.clientY - keysRect.top };
}

function startTouch(id, touch, target) {
  if ($('pedal').contains(target)) {
    touches.set(id, { pedal: true });
    setPedal(true);
    return;
  }
  if (!keysRect) return;
  const { x, y } = local(touch);
  const key = keyAt(x, y);
  if (!key) return;
  const t = {};
  touches.set(id, t);
  noteOn(t, touch, key, y / key.h, performance.now());
}

function moveTouch(id, touch) {
  const t = touches.get(id);
  if (!t || t.pedal || t.note === undefined) return;
  const { x, y } = local(touch);
  const key = keyAt(x, y);
  if (key && key.note !== t.note && settings.useGlide) {
    const vel = t.vel || 0.6;
    noteOff(t);
    t.vel = vel;
    t.note = key.note;
    t.key = key;
    t.startYRel = y / key.h;
    t.pressure = 0;
    send({ t: 'on', n: key.note, v: Math.round(vel * 0.92 * 1000) / 1000 });
    paint(key.note, vel);
    return;
  }
  if (settings.usePressure && !t.pending) {
    // Sliding toward the front of the key (down) = more pressure, like leaning into it.
    let p = clamp(((y / t.key.h) - t.startYRel) * 2.5);
    if (forceSeen && touch.force > 0) p = Math.max(p, clamp(touch.force * 1.25 - (t.vel || 0) * 0.5));
    if (Math.abs(p - t.pressure) > 0.02) {
      t.pressure = p;
      send({ t: 'p', n: t.note, p: Math.round(p * 100) / 100 });
    }
  }
}

function endTouch(id) {
  const t = touches.get(id);
  if (!t) return;
  if (t.pedal) setPedal(false);
  else noteOff(t);
  touches.delete(id);
}

function releaseAll(notify = true) {
  for (const [id, t] of touches) {
    if (t.pedal) { if (notify) setPedal(false); else $('pedal').classList.remove('down'); }
    else if (notify) noteOff(t);
    else if (t.note !== undefined) paint(t.note, null);
    touches.delete(id);
  }
  pending.length = 0;
}

const stage = $('stage');
stage.addEventListener('touchstart', (e) => {
  e.preventDefault();
  keysRect = $('keys').getBoundingClientRect();
  requestWakeLock();
  for (const touch of e.changedTouches) startTouch(touch.identifier, touch, touch.target);
}, { passive: false });
stage.addEventListener('touchmove', (e) => {
  e.preventDefault();
  for (const touch of e.changedTouches) moveTouch(touch.identifier, touch);
}, { passive: false });
const touchEnd = (e) => {
  e.preventDefault();
  for (const touch of e.changedTouches) endTouch(touch.identifier);
};
stage.addEventListener('touchend', touchEnd, { passive: false });
stage.addEventListener('touchcancel', touchEnd, { passive: false });

// Mouse, for trying it out in a desktop browser.
let mouseDown = false;
stage.addEventListener('mousedown', (e) => {
  mouseDown = true;
  keysRect = $('keys').getBoundingClientRect();
  startTouch('mouse', { clientX: e.clientX, clientY: e.clientY, force: 0 }, e.target);
});
window.addEventListener('mousemove', (e) => {
  if (mouseDown) moveTouch('mouse', { clientX: e.clientX, clientY: e.clientY, force: 0 });
});
window.addEventListener('mouseup', () => { mouseDown = false; endTouch('mouse'); });

// ---------- controls ----------
$('oct-down').addEventListener('click', () => { settings.base = Math.max(0, settings.base - 12); store.set('base', settings.base); buildKeys(); });
$('oct-up').addEventListener('click', () => { settings.base = Math.min(108, settings.base + 12); store.set('base', settings.base); buildKeys(); });
$('zoom-in').addEventListener('click', () => { settings.keyW = Math.min(120, settings.keyW + 8); store.set('keyW', settings.keyW); buildKeys(); });
$('zoom-out').addEventListener('click', () => { settings.keyW = Math.max(30, settings.keyW - 8); store.set('keyW', settings.keyW); buildKeys(); });

$('settings-btn').addEventListener('click', () => { $('settings').classList.remove('hidden'); releaseAll(); });
$('status').addEventListener('click', () => { $('settings').classList.remove('hidden'); releaseAll(); });
$('close-settings').addEventListener('click', () => $('settings').classList.add('hidden'));
$('settings').addEventListener('touchstart', (e) => e.stopPropagation(), { passive: true });

$('curve').value = settings.curve;
$('curve').addEventListener('change', (e) => setCurve(e.target.value, true));
$('sensitivity').value = settings.sensitivity;
$('sensitivity').addEventListener('input', (e) => { settings.sensitivity = parseFloat(e.target.value); store.set('sensitivity', settings.sensitivity); });
for (const [id, key] of [['use-position', 'usePosition'], ['use-pressure', 'usePressure'], ['use-glide', 'useGlide']]) {
  $(id).checked = settings[key];
  $(id).addEventListener('change', (e) => { settings[key] = e.target.checked; store.set(key, settings[key]); });
}
$('device-name').value = settings.name;
$('device-name').addEventListener('change', (e) => {
  settings.name = e.target.value.trim() || defaultName();
  store.set('name', settings.name);
  sendHello();
});

function describeMotion() {
  const note = $('motion-note');
  if (!window.isSecureContext) {
    note.textContent = 'Tap strength needs the HTTPS address shown in the desktop app (iOS only shares the accelerometer with secure pages).';
    $('motion-btn').disabled = true;
  } else if (typeof DeviceMotionEvent === 'undefined') {
    note.textContent = 'This browser has no accelerometer access.';
    $('motion-btn').disabled = true;
  } else {
    note.textContent = settings.motion
      ? 'Tap strength on: harder taps play louder. Adds up to ~16 ms while it reads the tap.'
      : 'Uses the accelerometer to feel how hard you tap (works best holding the phone or on a soft surface).';
  }
  $('force-note').textContent = forceSeen ? 'Pressure-sensitive touch detected (3D Touch / Apple Pencil) — used for velocity.' : '';
}

$('motion-btn').addEventListener('click', async () => {
  try {
    if (typeof DeviceMotionEvent !== 'undefined' && typeof DeviceMotionEvent.requestPermission === 'function') {
      const r = await DeviceMotionEvent.requestPermission();
      if (r !== 'granted') throw new Error('denied');
    }
    window.addEventListener('devicemotion', onMotion);
    settings.motion = !settings.motion;
    if (!settings.motion) window.removeEventListener('devicemotion', onMotion);
    $('motion-btn').textContent = settings.motion ? 'Disable tap strength' : 'Enable tap strength (accelerometer)';
  } catch (_) {
    $('motion-note').textContent = 'Motion access was not granted. You can allow it in Settings → Safari → Motion & Orientation Access.';
  }
  describeMotion();
});

// ---------- lifecycle ----------
let wakeLock = null;
async function requestWakeLock() {
  if (wakeLock || !('wakeLock' in navigator)) return;
  try {
    wakeLock = await navigator.wakeLock.request('screen');
    wakeLock.addEventListener('release', () => { wakeLock = null; });
  } catch (_) { /* not allowed right now */ }
}

document.addEventListener('visibilitychange', () => {
  if (document.hidden) releaseAll();
  else requestWakeLock();
});

function flash() {
  const f = $('flash');
  f.classList.remove('hidden');
  requestAnimationFrame(() => f.classList.add('on'));
  setTimeout(() => f.classList.remove('on'), 700);
  setTimeout(() => f.classList.add('hidden'), 1000);
}

let resizeTimer = null;
window.addEventListener('resize', () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    releaseAll();
    buildKeys();
    sendHello();
    checkOrientation();
  }, 120);
});

function checkOrientation() {
  const portraitPhone = window.innerHeight > window.innerWidth && window.innerWidth < 600;
  const r = $('rotate');
  if (portraitPhone && !checkOrientation.shown) {
    checkOrientation.shown = true;
    r.classList.remove('hidden');
    setTimeout(() => r.classList.add('hidden'), 2500);
  }
}

buildKeys();
describeMotion();
checkOrientation();
updateStatus();
connect();
