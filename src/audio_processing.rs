use rustfft::{FftPlanner, Fft};
use rustfft::num_complex::Complex;
use std::sync::Arc;
use crate::configs::{CENTRAL_FREQS,SAMPLE_RATE};

pub struct Processor {
    fft: Arc<dyn Fft<f32>>,
    buffer: Vec<Complex<f32>>,
}

impl Processor {
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(size);

        Self {
            fft,
            buffer: vec![Complex::new(0.0, 0.0); size],
        }
    }
// Se chunk = 0, pula
// Prepare the handling to the upper layer, so I
// can guarantee that this function is called only with samples    
pub fn process(&mut self, chunk: &[f32]) -> Vec<f32> {
    
    let band_limits = get_freq_lims(&CENTRAL_FREQS);

    let n = chunk.len();

    // Copy samples into FFT buffer
    for i in 0..n {
        self.buffer[i] = Complex::new(chunk[i], 0.0);
    }

    // Run FFT
    self.fft.process(&mut self.buffer);

    // Accumulate power per band
    let mut band_power = vec![0.0f32; band_limits.len()];
    let mut band_counts = vec![0usize; band_limits.len()];

    for bin in 0..n / 2 {
        let re = self.buffer[bin].re;
        let im = self.buffer[bin].im;

        let magnitude = (re * re + im * im).sqrt() / n as f32;

        let freq = bin as f32 * SAMPLE_RATE / n as f32;

        for (band_idx, (f_low, f_high)) in band_limits.iter().enumerate() {
            if freq >= *f_low && freq < *f_high {
                // accumulate power
                band_power[band_idx] += magnitude * magnitude;
                band_counts[band_idx] += 1;
                break;
            }
        }
    }

    // RMS power per band
    let mut band_values = vec![0.0f32; band_limits.len()];

    for i in 0..band_limits.len() {
        if band_counts[i] > 0 {
            band_values[i] = band_power[i].sqrt();
        }
        
    }

    band_values
}
}
pub fn get_freq_lims(central_freqs: &[f32]) -> Vec<(f32, f32)> {
    //(Panics if false)
    assert!(!central_freqs.is_empty(),"ERROR: vector 'central_freqs' cannot be empty.");
    
    let mut frequencies = Vec::new();

    let mut lower_edge = 0.0;

    for i in 0..central_freqs.len() {
        let upper_edge = if i == central_freqs.len() - 1 {
            central_freqs[i] * 2f32.sqrt()
        } else {
            (central_freqs[i] * central_freqs[i + 1]).sqrt()
        };

        if i == 0 {
            lower_edge = central_freqs[i] / 2f32.sqrt();
        }

        frequencies.push((lower_edge, upper_edge));

        lower_edge = upper_edge;
    }

    frequencies
}
