use std::sync::{
    Arc,
    atomic::AtomicBool,
    mpsc,
};
use super::super::audio::source::AudioFrame;

pub trait DisplaySource: Send {
    fn display_results(
        &mut self,
        rx_bands: mpsc::Receiver<AudioFrame>,
        shutdown: Arc<AtomicBool>
    );
}