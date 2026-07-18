use hound::WavReader;
use super::source::AudioSource;
use crate::configs::{WAV_FILE,CHUNK_SIZE};
 use std::error::Error;
// pub const CHUNK_SIZE:usize = 4096;

pub struct WavSource {
    path: String,
    samples: hound::WavIntoSamples<std::io::BufReader<std::fs::File>, i16>,
}

impl WavSource {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let reader = WavReader::open(path)?;

        Ok(Self {
            path: path.to_string(),
            samples: reader.into_samples::<i16>(),
        })
    }
    fn reopen(&mut self) -> Result<(), hound::Error> {
        let reader = WavReader::open(&self.path)?;
        self.samples = reader.into_samples::<i16>();
        Ok(())
    }
    pub fn default() -> Result<Self, Box<dyn Error>> {
        Self::new(WAV_FILE)
    }
}

impl AudioSource for WavSource {

    fn self_test(&mut self) -> Result<(), Box<dyn Error>>{
        Ok(())
    }
    fn next_chunk(&mut self) -> Option<Vec<f32>> {
        let mut chunk = Vec::with_capacity(CHUNK_SIZE);

        while chunk.len() < CHUNK_SIZE {
            match self.samples.next() {
                Some(Ok(sample)) => {
                    chunk.push(sample as f32 / i16::MAX as f32);
                }

                Some(Err(_)) => {
                    return None;
                }

                None => {
                    // EOF reached: reopen and continue reading from the beginning
                    if self.reopen().is_err() {
                        return None;
                    }
                }
            }
        }

        Some(chunk)
    }
}