use std::f32::consts::PI;

pub struct Oscillator {
    frequency: f32,
    phase: f32,
    sample_rate: f32,
    amplitude: f32,
    is_playing: bool,
}

impl Oscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            frequency: 440.0,  // A4 note
            phase: 0.0,
            sample_rate,
            amplitude: 0.5,
            is_playing: false,
        }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency.clamp(20.0, 20000.0);
        self.is_playing = true;
        println!("Playing note at {} Hz", self.frequency); // FIXME remove 
    }

    pub fn next_sample(&mut self) -> f32 {
        if !self.is_playing {
            return 0.0;
        }
        let sample = (self.phase * 2.0 * PI).sin() * self.amplitude;
        self.phase += self.frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.phase = 0.0;
    }

    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude.clamp(0.0, 1.0);
    }

    // Getters
    pub fn frequency(&self) -> f32 { self.frequency }
    pub fn amplitude(&self) -> f32 { self.amplitude }
    pub fn is_playing(&self) -> bool { self.is_playing }
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new(44100.0)  // CD quality sample rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_oscillator() {
        let osc = Oscillator::new(44100.0);
        assert_eq!(osc.frequency, 440.0);
        assert_eq!(osc.amplitude, 0.5);
        assert!(!osc.is_playing);
    }

    #[test]
    fn test_frequency_clamping() {
        let mut osc = Oscillator::default();
        osc.set_frequency(25000.0);
        assert_eq!(osc.frequency, 20000.0);
    }

    #[test]
    fn test_amplitude_clamping() {
        let mut osc = Oscillator::default();
        osc.set_amplitude(1.5);
        assert_eq!(osc.amplitude, 1.0);
    }
}