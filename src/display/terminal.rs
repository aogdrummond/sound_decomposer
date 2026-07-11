use super::source::DisplaySource;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use crate::audio::source::AudioFrame;
use crate::utils::utils::to_db_display;
use std::time::{Duration, Instant};
use crate::Error;
pub struct TerminalDisplay {}

impl TerminalDisplay{
    pub fn new() -> Result<Self, hound::Error> {     
        Ok(Self{})
    }
}

impl DisplaySource for TerminalDisplay {
    fn self_test(&mut self) -> Result<(), Box<dyn Error>>{
        Ok(())
    }
    }
    fn display_results(&mut self, rx_bands: mpsc::Receiver<AudioFrame>,shutdown: Arc<AtomicBool>){

        let mut last_update = Instant::now();
        let mut band_acc = vec![0.0f32; 8];
        let mut count = 0;

    while !shutdown.load(Ordering::SeqCst) {
        match rx_bands.recv_timeout(Duration::from_millis(100)) {
            Ok(bands) => {
                for i in 0..bands.samples.len() {
                    band_acc[i] += bands.samples[i];
                }

                count += 1;
                if last_update.elapsed() >= Duration::from_secs(1)
                    && count > 0
                {
                    last_update = Instant::now();

                    let mut avg = vec![0.0; 8];

                    for i in 0..8 {
                        avg[i] = to_db_display(
                                band_acc[i] / count as f32
                            );
                    }

                    print!("Bands: ");

                    for (i, v) in avg.iter().enumerate() {
                        print!("B{}: {:.2}dB ; ", i + 1, v);
                    }

                    println!();
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