use crate::model::effects::{eq_gain, MasterEffects, TrackEffects, FILTER_DEAD_ZONE};

const COMB_DELAYS: [usize; 4] = [1557, 1617, 1491, 1422];
const AP_DELAY: usize = 556;
/// Longest echo the delay line can hold.
const MAX_ECHO_SECS: f32 = 3.0;
/// Parameter changes glide over about this long, so knob moves and automation steps
/// (which arrive at UI frame rate) never click.
const SMOOTH_SECS: f32 = 0.015;
/// Filter coefficients are recomputed every this many samples (they involve a `tan`).
const COEF_EVERY: u32 = 8;

/// One channel strip: EQ → DJ filter → phaser → tone/distance → echo → reverb.
pub struct EffectProcessor {
    sample_rate: f32,
    bpm: f32,
    // Smoothed parameters (glide toward the target each sample).
    cur: TrackEffects,
    smooth_coef: f32,
    // Reverb.
    comb_bufs: [Vec<f32>; 4],
    comb_idx: [usize; 4],
    ap_buf: Vec<f32>,
    ap_idx: usize,
    // Phaser + tone.
    phaser_phase: f32,
    phaser_y1: f32,
    lp_state: f32,
    // 3-band EQ crossovers (one-pole low-passes at 250 Hz and 2.5 kHz).
    eq_lo_state: f32,
    eq_hi_state: f32,
    eq_lo_coef: f32,
    eq_hi_coef: f32,
    // DJ filter: TPT state-variable filter.
    svf_ic1: f32,
    svf_ic2: f32,
    svf_a1: f32,
    svf_a2: f32,
    svf_a3: f32,
    svf_k: f32,
    coef_countdown: u32,
    // Echo.
    echo_buf: Vec<f32>,
    echo_idx: usize,
}

impl EffectProcessor {
    pub fn new(sample_rate: f32) -> Self {
        let one_pole = |hz: f32| 1.0 - (-std::f32::consts::TAU * hz / sample_rate).exp();
        Self {
            sample_rate,
            bpm: 120.0,
            cur: TrackEffects::default(),
            smooth_coef: 1.0 - (-1.0 / (SMOOTH_SECS * sample_rate)).exp(),
            comb_bufs: [
                vec![0.0; COMB_DELAYS[0]],
                vec![0.0; COMB_DELAYS[1]],
                vec![0.0; COMB_DELAYS[2]],
                vec![0.0; COMB_DELAYS[3]],
            ],
            comb_idx: [0; 4],
            ap_buf: vec![0.0; AP_DELAY],
            ap_idx: 0,
            phaser_phase: 0.0,
            phaser_y1: 0.0,
            lp_state: 0.0,
            eq_lo_state: 0.0,
            eq_hi_state: 0.0,
            eq_lo_coef: one_pole(250.0),
            eq_hi_coef: one_pole(2_500.0),
            svf_ic1: 0.0,
            svf_ic2: 0.0,
            svf_a1: 1.0,
            svf_a2: 0.0,
            svf_a3: 0.0,
            svf_k: 1.0,
            coef_countdown: 0,
            echo_buf: vec![0.0; (MAX_ECHO_SECS * sample_rate) as usize + 1],
            echo_idx: 0,
        }
    }

    /// Start from these settings without gliding (e.g. at the start of an export).
    pub fn jump_to(&mut self, fx: &TrackEffects) {
        self.cur = *fx;
        self.coef_countdown = 0;
    }

    /// Tempo for the beat-synced echo.
    pub fn set_tempo(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 400.0);
    }

    pub fn process_track(&mut self, input: f32, fx: &TrackEffects) -> f32 {
        self.glide(fx);
        let c = self.cur;
        let mut s = input;
        s = self.eq(s, c.eq_low, c.eq_mid, c.eq_high);
        s = self.dj_filter(s, c.filter);
        s = self.space(s, c.effective_lowpass(), c.phaser, c.phaser_rate);
        s = self.echo(s, c.echo, c.echo_beats);
        self.reverb_mix(s, c.effective_reverb())
    }

    pub fn process_master(&mut self, input: f32, fx: &MasterEffects) -> f32 {
        self.process_track(input, &fx.as_track())
    }

    fn glide(&mut self, target: &TrackEffects) {
        let k = self.smooth_coef;
        let c = &mut self.cur;
        let step = |cur: &mut f32, t: f32| *cur += (t - *cur) * k;
        step(&mut c.distance, target.distance);
        step(&mut c.reverb, target.reverb);
        step(&mut c.phaser, target.phaser);
        step(&mut c.lowpass, target.lowpass);
        step(&mut c.eq_low, target.eq_low);
        step(&mut c.eq_mid, target.eq_mid);
        step(&mut c.eq_high, target.eq_high);
        step(&mut c.filter, target.filter);
        step(&mut c.echo, target.echo);
        // Rate and echo time jump: gliding them would warble the pitch.
        c.phaser_rate = target.phaser_rate;
        c.echo_beats = target.echo_beats;
    }

    fn eq(&mut self, s: f32, low: f32, mid: f32, high: f32) -> f32 {
        // Always run the crossovers so turning a knob from flat never clicks.
        self.eq_lo_state += self.eq_lo_coef * (s - self.eq_lo_state);
        self.eq_hi_state += self.eq_hi_coef * (s - self.eq_hi_state);
        if low.abs() < 1e-4 && mid.abs() < 1e-4 && high.abs() < 1e-4 {
            return s;
        }
        let lo = self.eq_lo_state;
        let hi = s - self.eq_hi_state;
        let mi = s - lo - hi;
        lo * eq_gain(low) + mi * eq_gain(mid) + hi * eq_gain(high)
    }

    /// One knob, two filters: left sweeps a low-pass down, right sweeps a high-pass up.
    fn dj_filter(&mut self, s: f32, knob: f32) -> f32 {
        let amount = knob.abs();
        if self.coef_countdown == 0 {
            self.coef_countdown = COEF_EVERY;
            let nyq = self.sample_rate * 0.45;
            let fc = if knob < 0.0 {
                // 20 kHz → 60 Hz, exponential so the sweep sounds even.
                20_000.0 * (60.0_f32 / 20_000.0).powf(amount)
            } else {
                // 20 Hz → 8 kHz.
                20.0 * (8_000.0_f32 / 20.0).powf(amount)
            }
            .min(nyq);
            let q = 0.75 + 0.9 * amount; // a little resonance as it closes, like hardware
            let g = (std::f32::consts::PI * fc / self.sample_rate).tan();
            self.svf_k = 1.0 / q;
            self.svf_a1 = 1.0 / (1.0 + g * (g + self.svf_k));
            self.svf_a2 = g * self.svf_a1;
            self.svf_a3 = g * self.svf_a2;
        }
        self.coef_countdown -= 1;

        let v3 = s - self.svf_ic2;
        let v1 = self.svf_a1 * self.svf_ic1 + self.svf_a2 * v3;
        let v2 = self.svf_ic2 + self.svf_a2 * self.svf_ic1 + self.svf_a3 * v3;
        self.svf_ic1 = 2.0 * v1 - self.svf_ic1;
        self.svf_ic2 = 2.0 * v2 - self.svf_ic2;
        let filtered = if knob < 0.0 { v2 } else { s - self.svf_k * v1 - v2 };

        // Fade in over the detent so crossing the centre never clicks.
        let mix = (amount / (FILTER_DEAD_ZONE * 2.0)).clamp(0.0, 1.0);
        s + (filtered - s) * mix
    }

    fn space(&mut self, input: f32, lowpass: f32, phaser_depth: f32, phaser_rate: f32) -> f32 {
        let dt = 1.0 / self.sample_rate;
        let mut s = input;

        if phaser_depth > 0.001 {
            self.phaser_phase += phaser_rate * dt * std::f32::consts::TAU;
            if self.phaser_phase > std::f32::consts::TAU {
                self.phaser_phase -= std::f32::consts::TAU;
            }
            let modulated = 0.2 + (self.phaser_phase.sin() * 0.5 + 0.5) * 0.75;
            let alpha = modulated * phaser_depth * 0.9;
            let y0 = s;
            self.phaser_y1 += alpha * (y0 - self.phaser_y1);
            s = y0 * (1.0 - phaser_depth * 0.5) + self.phaser_y1 * phaser_depth * 0.5;
        }

        let alpha = lowpass.clamp(0.02, 1.0) * 0.45;
        self.lp_state += alpha * (s - self.lp_state);
        let filtered = if lowpass < 0.98 { self.lp_state } else { s };
        s * (1.0 - (1.0 - lowpass) * 0.35) + filtered * (1.0 - lowpass) * 0.35
    }

    /// Tempo-synced feedback echo. Turning it down lets the repeats ring out naturally
    /// (the "echo out" DJ trick) instead of cutting them.
    fn echo(&mut self, s: f32, amount: f32, beats: f32) -> f32 {
        let len = self.echo_buf.len();
        let delay = ((beats.max(0.0625) * 60.0 / self.bpm) * self.sample_rate) as usize;
        let delay = delay.clamp(1, len - 1);
        let read = (self.echo_idx + len - delay) % len;
        let delayed = self.echo_buf[read];
        let feedback = 0.3 + 0.35 * amount;
        self.echo_buf[self.echo_idx] = s * amount + delayed * feedback;
        self.echo_idx = (self.echo_idx + 1) % len;
        s + delayed * 0.8
    }

    fn reverb_mix(&mut self, dry: f32, reverb_mix: f32) -> f32 {
        let wet = self.reverb_tick(dry);
        dry * (1.0 - reverb_mix) + wet * reverb_mix
    }

    fn reverb_tick(&mut self, input: f32) -> f32 {
        let mut out = 0.0;
        for (i, delay) in COMB_DELAYS.iter().enumerate() {
            let idx = self.comb_idx[i];
            let delayed = self.comb_bufs[i][idx];
            let next = input + delayed * 0.78;
            self.comb_bufs[i][idx] = next;
            self.comb_idx[i] = (idx + 1) % delay;
            out += delayed;
        }
        out *= 0.22;

        let ap_idx = self.ap_idx;
        let ap_delayed = self.ap_buf[ap_idx];
        let ap_out = -out * 0.55 + ap_delayed;
        self.ap_buf[ap_idx] = out + ap_delayed * 0.55;
        self.ap_idx = (ap_idx + 1) % AP_DELAY;

        ap_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// RMS of a 0.5-amplitude sine at `hz` after settling through the processor.
    fn rms_through(fx: &TrackEffects, hz: f32) -> f32 {
        let mut p = EffectProcessor::new(SR);
        p.jump_to(fx);
        let n = (SR * 0.3) as usize;
        let mut acc = 0.0;
        let mut count = 0;
        for i in 0..n {
            let x = (std::f32::consts::TAU * hz * i as f32 / SR).sin() * 0.5;
            let y = p.process_track(x, fx);
            if i > n / 2 {
                acc += y * y;
                count += 1;
            }
        }
        (acc / count as f32).sqrt()
    }

    #[test]
    fn neutral_settings_pass_signal_through() {
        let rms = rms_through(&TrackEffects::default(), 440.0);
        assert!((rms - 0.3535).abs() < 0.02, "rms {rms}");
    }

    #[test]
    fn low_pass_filter_removes_highs() {
        let fx = TrackEffects { filter: -0.8, ..Default::default() };
        assert!(rms_through(&fx, 5_000.0) < 0.05);
        assert!(rms_through(&fx, 80.0) > 0.25);
    }

    #[test]
    fn high_pass_filter_removes_lows() {
        let fx = TrackEffects { filter: 0.8, ..Default::default() };
        assert!(rms_through(&fx, 60.0) < 0.05);
        assert!(rms_through(&fx, 8_000.0) > 0.25);
    }

    #[test]
    fn eq_kill_removes_its_band() {
        let fx = TrackEffects { eq_low: -1.0, ..Default::default() };
        assert!(rms_through(&fx, 40.0) < 0.12);
        assert!(rms_through(&fx, 6_000.0) > 0.3);
    }

    #[test]
    fn echo_repeats_after_the_beat_time() {
        let fx = TrackEffects { echo: 1.0, echo_beats: 1.0, ..Default::default() };
        let mut p = EffectProcessor::new(SR);
        p.set_tempo(120.0); // 1 beat = 0.5 s
        p.jump_to(&fx);
        let mut out = Vec::new();
        for i in 0..(SR as usize) {
            let x = if i == 0 { 1.0 } else { 0.0 };
            out.push(p.process_track(x, &fx));
        }
        let at = (SR * 0.5) as usize;
        assert!(out[at].abs() > 0.3, "no echo at 0.5 s: {}", out[at]);
        assert!(out[at / 2].abs() < 1e-3);
    }
}
