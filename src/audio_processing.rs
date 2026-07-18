
use rustfft::{FftPlanner, Fft};
use rustfft::num_complex::Complex;
use std::sync::{
    Arc,
    mpsc,
    atomic::{AtomicBool, Ordering},
};
use log::{info,trace,error,warn};
use std::time::{Duration, Instant};

use crate::configs::{CENTRAL_FREQS,SAMPLE_RATE,CHUNK_SIZE};
use crate::audio::source::AudioFrame;

pub struct Processor {
    fft: Arc<dyn Fft<f32>>,
    buffer: Vec<Complex<f32>>,
}

impl Processor {
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(size);

        Self {fft,buffer: vec![Complex::new(0.0, 0.0); size],}
    }

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

pub fn process_audio(
    rx_chunk: mpsc::Receiver<AudioFrame>,
    tx_bands: mpsc::Sender<AudioFrame>,
    shutdown: Arc<AtomicBool>,
) {
    let mut processor = Processor::new(CHUNK_SIZE);

    info!("Initiating processing thread.");

    while !shutdown.load(Ordering::SeqCst) {
        match rx_chunk.recv_timeout(Duration::from_millis(100)) {
            Ok(frame) => {
                trace!(
                    "Latency Processing: {:.3} ms",
                    frame.timestamp.elapsed().as_secs_f64() * 1000.0
                );
                // Here guarantee that frame.samples ain't empty. If it is, skip.
                if frame.samples.is_empty() {
                    info!("Processing: received empty frame, skipping.");
                    continue;
                } 
                let start = Instant::now();
                let bands = processor.process(&frame.samples);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                trace!("Elapsed: {:.3} ms", elapsed);

                let frame2 = AudioFrame {
                    timestamp: Instant::now(),
                    samples: bands,
                };

                if tx_bands.send(frame2).is_err() {
                    error!("Processing: Sending data for display failed, dropping it.");
                    // It informs it failed and then returns processing, keep trying to send forever
                    //
                }
            }

            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No frame arrived during this period.
                // That's fine: loop again and check shutdown flag.
                continue;
            }

            Err(mpsc::RecvTimeoutError::Disconnected) => {
                info!("Processing: input channel disconnected, stopping.");
                break;
            }
        }
    }

    info!("Processing thread exiting.");
}

///////////////////////////////////////////////////////////
// TESTING SECTION
///////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::AtomicBool,
        Arc,
        mpsc,
    };
    use std::thread;
    use std::time::Instant;

    #[test]
    fn processor_new_creates_correct_buffer_size() {
        let processor = Processor::new(CHUNK_SIZE);

        assert_eq!(processor.buffer.len(), CHUNK_SIZE);
    }

    #[test]
    fn process_returns_one_value_per_band() {
        let mut processor = Processor::new(CHUNK_SIZE);

        let input = vec![0.0; CHUNK_SIZE];

        let output = processor.process(&input);

        assert_eq!(output.len(), CENTRAL_FREQS.len());
    }

    #[test]
    fn process_of_silence_returns_zero_energy() {
        let mut processor = Processor::new(CHUNK_SIZE);

        let input = vec![0.0; CHUNK_SIZE];

        let output = processor.process(&input);

        for value in output {
            assert!(value.abs() < 1e-6);
        }
    }

    #[test]
    fn process_does_not_return_nan() {
        let mut processor = Processor::new(CHUNK_SIZE);

        let input = vec![1.0; CHUNK_SIZE];

        let output = processor.process(&input);

        for value in output {
            assert!(!value.is_nan());
            assert!(value.is_finite());
        }
    }

    #[test]
    fn get_freq_lims_returns_same_number_of_bands() {
        let bands = get_freq_lims(&CENTRAL_FREQS);

        assert_eq!(bands.len(), CENTRAL_FREQS.len());
    }

    #[test]
    fn get_freq_lims_are_contiguous() {
        let bands = get_freq_lims(&CENTRAL_FREQS);

        for i in 1..bands.len() {
            assert!((bands[i - 1].1 - bands[i].0).abs() < 1e-6);
        }
    }

    #[test]
    fn get_freq_lims_are_increasing() {
        let bands = get_freq_lims(&CENTRAL_FREQS);

        for (low, high) in bands {
            assert!(high > low);
        }
    }

    #[test]
    #[should_panic]
    fn get_freq_lims_panics_for_empty_vector() {
        get_freq_lims(&[]);
    }

    #[test]
    fn process_audio_processes_one_frame() {

        let (tx_chunk, rx_chunk) = mpsc::channel();
        let (tx_bands, rx_bands) = mpsc::channel();

        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_thread = Arc::clone(&shutdown);

        let handle = thread::spawn(move || {
            process_audio(rx_chunk, tx_bands, shutdown_thread);
        });

        tx_chunk.send(AudioFrame {
            timestamp: Instant::now(),
            samples: vec![0.0; CHUNK_SIZE],
        }).unwrap();

        let result = rx_bands.recv_timeout(Duration::from_secs(1));

        assert!(result.is_ok());

        shutdown.store(true, Ordering::SeqCst);

        drop(tx_chunk);

        handle.join().unwrap();
    }

    #[test]
    fn process_audio_skips_empty_frames() {

        let (tx_chunk, rx_chunk) = mpsc::channel();
        let (tx_bands, rx_bands) = mpsc::channel();

        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_thread = Arc::clone(&shutdown);

        let handle = thread::spawn(move || {
            process_audio(rx_chunk, tx_bands, shutdown_thread);
        });

        tx_chunk.send(AudioFrame {
            timestamp: Instant::now(),
            samples: vec![],
        }).unwrap();

        assert!(rx_bands.recv_timeout(Duration::from_millis(300)).is_err());

        shutdown.store(true, Ordering::SeqCst);

        drop(tx_chunk);

        handle.join().unwrap();
    }

    #[test]
    fn process_detects_500hz_tone() {

        let mut processor = Processor::new(CHUNK_SIZE);

        let frequency = 500.0;

        let signal: Vec<f32> = (0..CHUNK_SIZE)
            .map(|n| {
                (2.0 * std::f32::consts::PI
                    * frequency
                    * n as f32
                    / SAMPLE_RATE)
                    .sin()
            })
            .collect();

        let bands = processor.process(&signal);

        let max_band = bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;


        let band_limits = get_freq_lims(&CENTRAL_FREQS);

        let expected_band = band_limits
            .iter()
            .position(|(low, high)| frequency >= *low && frequency < *high)
            .unwrap();

        assert_eq!(max_band, expected_band);
    }
}