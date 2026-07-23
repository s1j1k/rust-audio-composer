use crate::model::effects::{MasterEffects, TrackEffects};

const COMB_DELAYS: [usize; 4] = [1557, 1617, 1491, 1422];
const AP_DELAY: usize = 556;

pub struct EffectProcessor {
    sample_rate: f32,
    comb_bufs: [Vec<f32>; 4],
    comb_idx: [usize; 4],
    ap_buf: Vec<f32>,
    ap_idx: usize,
    phaser_phase: f32,
    phaser_y1: f32,
    lp_state: f32,
}

impl EffectProcessor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
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
        }
    }

    pub fn process_track(&mut self, input: f32, fx: &TrackEffects) -> f32 {
        self.process(
            input,
            fx.effective_lowpass(),
            fx.effective_reverb(),
            fx.phaser,
            fx.phaser_rate,
        )
    }

    pub fn process_master(&mut self, input: f32, fx: &MasterEffects) -> f32 {
        self.process(
            input,
            fx.effective_lowpass(),
            fx.effective_reverb(),
            fx.phaser,
            fx.phaser_rate,
        )
    }

    fn process(
        &mut self,
        input: f32,
        lowpass: f32,
        reverb_mix: f32,
        phaser_depth: f32,
        phaser_rate: f32,
    ) -> f32 {
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
        let filtered = if lowpass < 0.98 {
            self.lp_state
        } else {
            s
        };
        let dry = s * (1.0 - (1.0 - lowpass) * 0.35) + filtered * (1.0 - lowpass) * 0.35;

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
