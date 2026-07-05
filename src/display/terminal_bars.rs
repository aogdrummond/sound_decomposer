use super::source::DisplaySource;
use crate::audio::source::AudioFrame;
use crate::configs::{NUM_BANDS,BAND_LABELS,BAR_WIDTH,BLOCKS};
use crate::utils::utils::to_db_display;
use std::io::{stdout, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool,Ordering},
    mpsc,
};
use std::time::{Duration, Instant};


pub struct TerminalBars {}

impl TerminalBars {
    pub fn new() -> Result<Self, hound::Error> {

        // Clear terminal once
        print!("\x1B[2J");

        Ok(Self {})
    }
}

impl DisplaySource for TerminalBars {

    fn display_results(
        &mut self,
        rx_bands: mpsc::Receiver<AudioFrame>,
        shutdown: Arc<AtomicBool>
    ) {

        let mut last_update = Instant::now();

        let mut band_acc = vec![0.0f32; NUM_BANDS];
        let mut count = 0usize;

        let mut displayed = vec![0.0f32; NUM_BANDS];
        let mut peaks = vec![0.0f32; NUM_BANDS];

        while !shutdown.load(Ordering::SeqCst) {
        match rx_bands.recv_timeout(Duration::from_millis(100)) {
            Ok(bands) => {
                for i in 0..NUM_BANDS {
                    band_acc[i] += bands.samples[i];
                }

                count += 1;

                if last_update.elapsed() >= Duration::from_millis(100)
                    && count > 0
                {
                    last_update = Instant::now();

                    let mut avg = vec![0.0f32; NUM_BANDS];

                    for i in 0..NUM_BANDS {

                        avg[i] = to_db_display(
                            band_acc[i] / count as f32
                        );

                        // Exponential smoothing
                        displayed[i] =
                            0.8 * displayed[i]
                            + 0.2 * avg[i];

                        // Peak hold with decay
                        peaks[i] *= 0.97;

                        if displayed[i] > peaks[i] {
                            peaks[i] = displayed[i];
                        }
                    }

                    draw_bars(&displayed, &peaks);

                    band_acc.fill(0.0);
                    count = 0;
                }
                }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }
}
fn draw_bars(
    bands: &[f32],
    peaks: &[f32],
) {

    // Move cursor to top-left
    print!("\x1B[H");

    println!("Real-Time Spectrum Analyzer");
    println!("===========================");
    println!();

    for i in 0..NUM_BANDS {

        let value = bands[i];

        let scaled =
            (value / 80.0)
            .clamp(0.0, 1.0)
            * BAR_WIDTH as f32;

        let full_blocks =
            scaled.floor() as usize;

        let partial =
            ((scaled - full_blocks as f32) * 8.0)
            .round() as usize;

        let mut cells =
            vec![" ".to_string(); BAR_WIDTH];

        for j in 0..full_blocks.min(BAR_WIDTH) {
            cells[j] = "█".to_string();
        }

        if partial > 0 && full_blocks < BAR_WIDTH {
            cells[full_blocks] =
                BLOCKS[partial.min(8)].to_string();
        }
        // }

        let peak_pos =
            ((peaks[i] / 80.0)
            * BAR_WIDTH as f32)
            .clamp(0.0, (BAR_WIDTH - 1) as f32)
            as usize;

        if peak_pos < BAR_WIDTH {
            cells[peak_pos] = "|".to_string();
        }

        let bar = cells.join("");

        println!(
            "{:<10} | {} {:5.1}",
            BAND_LABELS[i],
            bar,
            value
        );
    }

    stdout().flush().unwrap();
}