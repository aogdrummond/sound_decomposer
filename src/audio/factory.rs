use std::error::Error;

use log::{error, info, warn};

use crate::audio::{
    mic::MicrophoneSource,
    source::AudioSource,
    wav::WavSource,
};

pub fn create_audio_source(
    name: &str,
) -> Result<Box<dyn AudioSource>, Box<dyn Error>> {

    match name {

        "mic" => initialize_with_fallback(
            "Microphone",
            "WAV source",
            || mic::MicrophoneSource::new(),
            || wav::WavSource::new(),
        ),

        "wav" => initialize_audio_source(
            wav::WavSource::new()?
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
