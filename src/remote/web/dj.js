// Composer Keys — DJ page: the desktop's DJ mixer on a phone or tablet.
// Four channel strips (EQ, filter, echo, reverb, fader, crossfader side), a crossfader,
// and PLAY / WRITE / REC MIX. Uses pointer events so several fingers can work at once.
// Loaded after app.js (uses its `send`, `$`, `clamp`).
'use strict';

const DJ = {
  snap: null,          // last state from the desktop
  built: false,
  held: new Map(),     // control id -> { send: fn, value }
  meters: [0, 0, 0, 0],
  master: 0,
};

const DJ_KNOBS = [
  { p: 'eq_high', label: 'HI', bip: true, size: 'sm', color: '#78beff' },
  { p: 'eq_mid', label: 'MID', bip: true, size: 'sm', color: '#78beff' },
  { p: 'eq_low', label: 'LOW', bip: true, size: 'sm', color: '#78beff' },
  { p: 'filter', label: 'FILTER', bip: true, size: 'lg', color: '#be78ff' },
  { p: 'echo', label: 'ECHO', bip: false, size: 'sm', color: '#5adcc8' },
  { p: 'reverb', label: 'VERB', bip: false, size: 'sm', color: '#5adcc8' },
];
const DJ_NEUTRAL = { volume: 0.8 };

function glyphFor(icon, glyph) {
  // Phosphor has no drum; phones have a good drum emoji.
  return icon === 'drums' ? '\u{1F941}' : glyph || '';
}

function chVal(ch, p) {
  const c = DJ.snap && DJ.snap.channels[ch];
  if (!c) return 0;
  for (const [k, v] of c.vals) if (k === p) return v;
  return 0;
}

// ---------- generic drag control ----------
// `axis`: 'knob' (up/right = more), 'v' (vertical position), 'h' (horizontal position).
function dragControl(el, id, opts) {
  let start = null;
  let lastTap = 0;
  el.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    e.stopPropagation();
    el.setPointerCapture(e.pointerId);
    const now = performance.now();
    if (now - lastTap < 300) {
      // Double tap: reset.
      opts.set(opts.neutral, true);
      lastTap = 0;
      return;
    }
    lastTap = now;
    start = { x: e.clientX, y: e.clientY, v: opts.get(), pid: e.pointerId };
    DJ.held.set(id, { send: () => opts.set(opts.get(), false) });
    if (opts.axis !== 'knob') moveTo(e);
    el.classList.add('active');
  });
  const moveTo = (e) => {
    const r = el.getBoundingClientRect();
    const [lo, hi] = opts.range;
    let t;
    if (opts.axis === 'v') t = 1 - (e.clientY - r.top - 10) / Math.max(1, r.height - 20);
    else t = (e.clientX - r.left - 12) / Math.max(1, r.width - 24);
    opts.set(lo + clamp(t) * (hi - lo), false);
  };
  el.addEventListener('pointermove', (e) => {
    if (!start || e.pointerId !== start.pid) return;
    e.preventDefault();
    if (opts.axis === 'knob') {
      const [lo, hi] = opts.range;
      const px = (e.clientX - start.x) - (e.clientY - start.y);
      opts.set(clamp(start.v + (px / 160) * (hi - lo), lo, hi), false);
    } else {
      moveTo(e);
    }
  });
  const end = (e) => {
    if (!start || e.pointerId !== start.pid) return;
    start = null;
    DJ.held.delete(id);
    el.classList.remove('active');
  };
  el.addEventListener('pointerup', end);
  el.addEventListener('pointercancel', end);
}

// Keep the desktop's "finger is still on it" alive while a held control isn't moving, so a
// held knob keeps writing a flat line into the automation.
setInterval(() => { for (const h of DJ.held.values()) h.send(); }, 150);

// ---------- building ----------
function knobEl(ch, k) {
  const wrap = document.createElement('div');
  wrap.className = `knob ${k.size}`;
  wrap.innerHTML = `<div class="ring"></div><div class="cap"><div class="ptr"></div></div><span>${k.label}</span>`;
  wrap.style.setProperty('--accent', k.color);
  const id = `${ch}:${k.p}`;
  wrap.dataset.id = id;
  dragControl(wrap, id, {
    axis: 'knob',
    range: k.bip ? [-1, 1] : [0, 1],
    neutral: DJ_NEUTRAL[k.p] ?? 0,
    get: () => parseFloat(wrap.dataset.v || '0'),
    set: (v, _reset) => {
      v = Math.round(v * 1000) / 1000;
      setKnob(wrap, v, k.bip);
      send({ t: 'dj', ch, p: k.p, v });
    },
  });
  return wrap;
}

function setKnob(wrap, v, bip) {
  wrap.dataset.v = String(v);
  const t = bip ? (v + 1) / 2 : v;
  const ang = -135 + 270 * t;
  wrap.querySelector('.cap').style.transform = `rotate(${ang}deg)`;
  const from = bip ? 0 : -135;
  // Angles measured from the start of the sweep (7 o'clock).
  const a0 = Math.min(from, ang) + 135, a1 = Math.max(from, ang) + 135;
  wrap.querySelector('.ring').style.background =
    `conic-gradient(from -135deg, #121216 0deg ${a0}deg, var(--accent) ${a0}deg ${a1}deg, #121216 ${a1}deg 270deg, transparent 270deg)`;
}

function faderEl(ch) {
  const el = document.createElement('div');
  el.className = 'fader';
  el.innerHTML = '<div class="slot"></div><div class="fcap"></div>';
  const id = `${ch}:volume`;
  el.dataset.id = id;
  dragControl(el, id, {
    axis: 'v',
    range: [0, 1],
    neutral: 0.8,
    get: () => parseFloat(el.dataset.v || '0'),
    set: (v) => {
      v = Math.round(v * 1000) / 1000;
      setFader(el, v);
      send({ t: 'dj', ch, p: 'volume', v });
    },
  });
  return el;
}

function setFader(el, v) {
  el.dataset.v = String(v);
  el.querySelector('.fcap').style.bottom = `${4 + clamp(v) * (el.clientHeight - 28)}px`;
}

function buildDj() {
  const host = $('dj-channels');
  host.innerHTML = '';
  for (let ch = 0; ch < 4; ch++) {
    const col = document.createElement('div');
    col.className = 'ch';
    col.id = `ch${ch}`;
    col.innerHTML = `
      <div class="ch-head"><span class="ph ch-icon"></span><select class="ch-track" aria-label="Track on channel ${ch + 1}"></select></div>
      <div class="ch-body">
        <div class="knobs eq"></div>
        <div class="knobs fx"></div>
        <div class="fader-row"><div class="vu"><div class="vu-fill"></div></div></div>
        <div class="sides">
          <button data-side="a">A</button><button data-side="thru">THRU</button><button data-side="b">B</button>
        </div>
        <button class="mute">MUTE</button>
      </div>
      <div class="ch-empty">Empty — pick a track</div>`;
    const eq = col.querySelector('.eq');
    const fx = col.querySelector('.fx');
    for (const k of DJ_KNOBS) (k.p.startsWith('eq') ? eq : fx).appendChild(knobEl(ch, k));
    col.querySelector('.fader-row').prepend(faderEl(ch));
    col.querySelector('.ch-track').addEventListener('change', (e) => {
      const v = e.target.value;
      send({ t: 'dj_load', ch, track: v === '' ? null : parseInt(v, 10) });
    });
    for (const b of col.querySelectorAll('.sides button')) {
      b.addEventListener('click', () => send({ t: 'dj_side', ch, side: b.dataset.side }));
    }
    col.querySelector('.mute').addEventListener('click', () => {
      const c = DJ.snap && DJ.snap.channels[ch];
      send({ t: 'dj_mute', ch, on: !(c && c.muted) });
    });
    host.appendChild(col);
  }

  const xf = $('xf');
  xf.innerHTML = '<span class="xf-a">A</span><div class="slot"></div><div class="xcap"></div><span class="xf-b">B</span>';
  dragControl(xf, 'xf', {
    axis: 'h',
    range: [-1, 1],
    neutral: 0,
    get: () => parseFloat(xf.dataset.v || '0'),
    set: (v) => {
      v = Math.round(v * 1000) / 1000;
      setXf(v);
      send({ t: 'xf', v });
    },
  });
  $('dj-play').addEventListener('click', () => send({ t: 'play', on: !(DJ.snap && DJ.snap.playing) }));
  $('dj-write').addEventListener('click', () => send({ t: 'dj_write', on: !(DJ.snap && DJ.snap.write) }));
  $('dj-mixrec').addEventListener('click', () => send({ t: 'mix_rec', on: !(DJ.snap && DJ.snap.mixrec) }));
  DJ.built = true;
}

function setXf(v) {
  const xf = $('xf');
  xf.dataset.v = String(v);
  const w = xf.clientWidth - 24 - 28;
  xf.querySelector('.xcap').style.left = `${12 + ((v + 1) / 2) * w}px`;
}

// ---------- applying desktop state ----------
function applyDj(snap) {
  DJ.snap = snap;
  if (!DJ.built) buildDj();
  const tracks = snap.tracks || [];
  snap.channels.forEach((c, ch) => {
    const col = $(`ch${ch}`);
    if (!col) return;
    col.style.setProperty('--track', c.color);
    col.classList.toggle('empty', c.track === null);
    const icon = col.querySelector('.ch-icon');
    icon.textContent = glyphFor(c.icon, c.glyph);
    icon.style.color = c.color;
    const sel = col.querySelector('.ch-track');
    const opts = [['', '— empty —']].concat(tracks.map((t) => [String(t.i), t.name]));
    const key = JSON.stringify(opts);
    if (sel.dataset.key !== key) {
      sel.innerHTML = '';
      for (const [v, label] of opts) {
        const o = document.createElement('option');
        o.value = v;
        o.textContent = label;
        sel.appendChild(o);
      }
      sel.dataset.key = key;
    }
    sel.value = c.track === null ? '' : String(c.track);
    if (c.track === null) return;
    for (const k of DJ_KNOBS) {
      const el = col.querySelector(`.knob[data-id="${ch}:${k.p}"]`);
      el.classList.toggle('auto', c.auto.includes(k.p));
      if (!DJ.held.has(`${ch}:${k.p}`)) setKnob(el, chVal(ch, k.p), k.bip);
    }
    const f = col.querySelector('.fader');
    f.classList.toggle('auto', c.auto.includes('volume'));
    if (!DJ.held.has(`${ch}:volume`)) setFader(f, chVal(ch, 'volume'));
    for (const b of col.querySelectorAll('.sides button')) b.classList.toggle('on', b.dataset.side === c.side);
    col.querySelector('.mute').classList.toggle('on', c.muted);
  });
  $('xf').classList.toggle('auto', snap.xf_auto);
  if (!DJ.held.has('xf')) setXf(snap.xf);
  $('dj-play').classList.toggle('on', snap.playing);
  $('dj-play').innerHTML = `<span class="ph">${snap.playing ? '' : ''}</span> ${snap.playing ? 'PAUSE' : 'PLAY'}`;
  $('dj-write').classList.toggle('on', snap.write);
  const rec = $('dj-mixrec');
  rec.classList.toggle('on', snap.mixrec);
  const s = snap.mixrec_secs || 0;
  rec.textContent = snap.mixrec ? `STOP ${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}` : 'REC MIX';
}

function applyMeters(m) {
  DJ.meters = m.l || [];
  DJ.master = m.m || 0;
  DJ.meters.forEach((lvl, ch) => {
    const col = $(`ch${ch}`);
    if (!col) return;
    const db = 20 * Math.log10(Math.max(lvl, 1e-6));
    col.querySelector('.vu-fill').style.height = `${clamp((db + 48) / 51) * 100}%`;
  });
  const db = 20 * Math.log10(Math.max(DJ.master, 1e-6));
  $('master-vu-fill').style.width = `${clamp((db + 48) / 51) * 100}%`;
}

// ---------- page switching ----------
function setMode(dj) {
  document.body.classList.toggle('dj-mode', dj);
  $('mode-btn').textContent = dj ? 'KEYS' : 'DJ';
  store.set('mode', dj ? 'dj' : 'keys');
  if (typeof releaseAll === 'function') releaseAll();
  if (dj) {
    if (!DJ.built) buildDj();
    if (DJ.snap) applyDj(DJ.snap);
  } else {
    buildKeys();
  }
}
$('mode-btn').addEventListener('click', () => setMode(!document.body.classList.contains('dj-mode')));
$('dj').addEventListener('touchstart', (e) => e.stopPropagation(), { passive: true });
window.addEventListener('resize', () => { if (DJ.snap) applyDj(DJ.snap); });
setMode(store.get('mode', 'keys') === 'dj');
