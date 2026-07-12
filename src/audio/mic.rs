use std::sync::mpsc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{debug,info};
use crate::Error;
use crate::Duration;
use super::source::AudioSource;
use crate::configs::CHUNK_SIZE;

pub struct MicrophoneSource {
    rx: mpsc::Receiver<f32>,
    _stream: cpal::Stream,
}

impl MicrophoneSource {

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {

        let host = cpal::default_host();

        let device = host
            .default_input_device()
            .ok_or("No input device found")?;

        debug!("Supported configs:");

        for cfg in device.supported_input_configs()? {
            debug!("{:?}", cfg);
        }

        let config = device.default_input_config()?;
        info!("Using device: {}", device.name()?);
        info!("Sample format: {:?}", config.sample_format());
        info!("Sample Rate: {}", config.sample_rate().0);
        info!("Channels: {}", config.channels());

        let (tx, rx) = mpsc::channel::<f32>();

        let stream = match config.sample_format() {

            cpal::SampleFormat::F32 => {

                let channels = config.channels() as usize;
                device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    for frame in data.chunks_exact(channels) {
                        let sample = frame[0];
                        let _ = tx.send(sample);
                    }
                },
                move |err| {
                    eprintln!("Audio error: {}", err);
                },
                None,
            )?
            }
    // cpal::SampleFormat::I16 => { ... }

    // cpal::SampleFormat::U16 => { ... }
            _ => return Err("Unsupported sample format".into()),
        };

        stream.play()?;

        Ok(Self {
            rx,
            _stream: stream,
        })
    }
}

impl AudioSource for MicrophoneSource {

    fn self_test(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Inside self test");

        let mut rms = 0.0;

        for _ in 0..5000 {
            let s = self.rx.recv_timeout(Duration::from_secs(2))?;
            rms += s*s;
        }

        rms = (rms / 5000.0).sqrt();

        println!("Mic RMS = {}", rms);

        let timeout = Duration::from_secs(2);

        match self.rx.recv_timeout(timeout) {
            Ok(sample) => {
                println!("Received sample {}", sample);
                Ok(())
            }
            Err(_) => Err("No microphone samples received".into()),
        }
    }

    fn next_chunk(&mut self) -> Option<Vec<f32>> {

        let mut chunk = Vec::with_capacity(CHUNK_SIZE);

        for _ in 0..CHUNK_SIZE {

            match self.rx.recv() {

                Ok(sample) => chunk.push(sample),

                Err(_) => return None,
            }
        }

        Some(chunk)
    }
}