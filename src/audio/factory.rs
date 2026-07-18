use std::error::Error;
use std::time::Instant;
use log::{error, info, warn};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

use super::{
    mic::MicrophoneSource,
    source::AudioSource,
    source::AudioFrame,
    wav::WavSource,
};


pub fn create_audio_source(
    name: &str,
) -> Result<Box<dyn AudioSource>, Box<dyn Error>> {

    match name {

        "mic" => initialize_with_fallback(
            "Microphone",
            "WAV source",
            || MicrophoneSource::new(),
            || WavSource::default(),
        ),

        "wav" => initialize_audio_source(
            WavSource::default()?
        ),

        other => Err(format!("Unknown audio source '{}'", other).into()),
    }
}


fn initialize_audio_source<T>(
    mut source: T,
) -> Result<Box<dyn AudioSource>, Box<dyn Error>>
where
    T: AudioSource + 'static,
{
    source.self_test()?;
    Ok(Box::new(source))
}

fn initialize_with_fallback<P, B>(
    primary_name: &str,
    backup_name: &str,
    primary: impl FnOnce() -> Result<P, Box<dyn Error>>,
    backup: impl FnOnce() -> Result<B, Box<dyn Error>>,
) -> Result<Box<dyn AudioSource>, Box<dyn Error>>
where
    P: AudioSource + 'static,
    B: AudioSource + 'static,
{
    info!("Trying to initialize {}...", primary_name);

    match primary() {
        Ok(source) => match initialize_audio_source(source) {
            Ok(source) => {
                info!("{} initialized successfully.", primary_name);
                Ok(source)
            }

            Err(e) => {
                error!("{} self-test failed: {}", primary_name, e);
                warn!("Falling back to {}.", backup_name);

                initialize_audio_source(backup()?)
            }
        },

        Err(e) => {
            error!("Unable to initialize {}: {}", primary_name, e);
            warn!("Falling back to {}.", backup_name);

            initialize_audio_source(backup()?)
        }
    }
}

pub fn produce_audio(
    mut source: Box<dyn AudioSource>,
    tx_chunk: mpsc::Sender<AudioFrame>,
    shutdown: Arc<AtomicBool>,
)    
{
    info!("Initiating producer thread.");
        while !shutdown.load(Ordering::SeqCst) {
        match source.next_chunk() {
            Some(chunk) => {
                let frame = AudioFrame {
                    timestamp: Instant::now(),
                    samples: chunk,
                };

                if tx_chunk.send(frame).is_err() {
                    info!("Producer: receiver dropped, stopping.");
                    break;
                }
            }
            None => {
                info!("Producer: source ended.");
                break;
            }
        }
    }
}
///////////////////////////////////////////////////////////
// TESTING SECTION
///////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSource {
        self_test_ok: bool,
    }

    impl FakeSource {
        fn ok() -> Self {
            Self {
                self_test_ok: true,
            }
        }

        fn failing() -> Self {
            Self {
                self_test_ok: false,
            }
        }
    }

    impl AudioSource for FakeSource {
        fn self_test(&mut self) -> Result<(), Box<dyn Error>> {
            if self.self_test_ok {
                Ok(())
            } else {
                Err("self test failed".into())
            }
        }

        fn next_chunk(&mut self) -> Option<Vec<f32>> {
            None
        }
    }

    #[test]
    fn initialize_audio_source_succeeds_when_self_test_passes() {
        let result = initialize_audio_source(FakeSource::ok());

        assert!(result.is_ok());
    }

    #[test]
    fn initialize_audio_source_fails_when_self_test_fails() {
        let result = initialize_audio_source(FakeSource::failing());

        assert!(result.is_err());
    }

    #[test]
    fn primary_source_is_used_when_available() {
        let result = initialize_with_fallback(
            "primary",
            "backup",
            || Ok(FakeSource::ok()),
            || Ok(FakeSource::ok()),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn backup_is_used_when_primary_creation_fails() {
        let result = initialize_with_fallback(
            "primary",
            "backup",
            || -> Result<FakeSource, Box<dyn Error>> {
                Err("primary failed".into())
            },
            || Ok(FakeSource::ok()),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn backup_is_used_when_primary_self_test_fails() {
        let result = initialize_with_fallback(
            "primary",
            "backup",
            || Ok(FakeSource::failing()),
            || Ok(FakeSource::ok()),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn returns_error_when_both_sources_fail_to_create() {
        let result = initialize_with_fallback(
            "primary",
            "backup",
            || -> Result<FakeSource, Box<dyn Error>> {
                Err("primary failed".into())
            },
            || -> Result<FakeSource, Box<dyn Error>> {
                Err("backup failed".into())
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn returns_error_when_backup_self_test_fails() {
        let result = initialize_with_fallback(
            "primary",
            "backup",
            || -> Result<FakeSource, Box<dyn Error>> {
                Err("primary failed".into())
            },
            || Ok(FakeSource::failing()),
        );

        assert!(result.is_err());
    }
}

// next_chunk()

// #[test]
// fn producer_sends_every_chunk() {

//     let source = ...

//     let (tx, rx) = mpsc::channel();

//     produce_audio(...);

//     assert_eq!(rx.iter().count(),2);
// }

// #[test]
// fn producer_stops_when_source_ends()

// #[test]
// fn producer_respects_shutdown_flag()