use hound::WavReader;
use super::source::AudioSource;
use crate::configs::{WAV_FILE,CHUNK_SIZE};
use crate::Error;
// pub const CHUNK_SIZE:usize = 4096;

pub struct WavSource {
    samples: hound::WavIntoSamples<std::io::BufReader<std::fs::File>,i16>
}
impl WavSource {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let reader = WavReader::open(WAV_FILE)?;

        Ok(Self {
            samples: reader.into_samples::<i16>(),
        })
    }
    fn reopen(&mut self) -> Result<(), hound::Error> {
        let reader = WavReader::open(WAV_FILE)?;
        self.samples = reader.into_samples::<i16>();
        Ok(())
    }
}
impl WavSource {
    pub fn new() -> Result<Self, hound::Error> {
        let reader = WavReader::open(WAV_FILE)?;

        Ok(Self {samples: reader.into_samples::<i16>()})
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