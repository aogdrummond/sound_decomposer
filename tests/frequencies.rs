use sound_processor::audio::{
    factory::produce_audio,
    source::AudioFrame,
    wav::WavSource,
};
use sound_processor::audio_processing::process_audio;
use sound_processor::configs::NUM_BANDS;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc,
};
use std::thread;
use std::time::Duration;

fn assert_frequency(path: &str, expected_band: usize) {
    let source = Box::new(
        WavSource::new(path)
            .expect("Failed to open WAV file"),
    );

    let shutdown = Arc::new(AtomicBool::new(false));

    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();

    let producer_shutdown = Arc::clone(&shutdown);
    let processor_shutdown = Arc::clone(&shutdown);

    let producer = thread::spawn(move || {
        produce_audio(source, tx_chunk, producer_shutdown);
    });

    let processor = thread::spawn(move || {
        process_audio(rx_chunk, tx_bands, processor_shutdown);
    });

    let frame = rx_bands
        .recv_timeout(Duration::from_secs(2))
        .expect("No processed frame received");

    assert_eq!(frame.samples.len(), NUM_BANDS);

    let (max_band, max_value) = frame
        .samples
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, v)| (i, *v))
        .unwrap();

    assert_eq!(
        max_band,
        expected_band,
        "Wrong dominant band for {}",
        path
    );

    for (i, value) in frame.samples.iter().enumerate() {
        if i != expected_band {
            assert!(
                max_value > *value,
                "Band {} has greater energy than expected band {}",
                i,
                expected_band
            );
        }
    }

    shutdown.store(true, Ordering::SeqCst);

    producer.join().unwrap();
    processor.join().unwrap();
}

#[test]
fn detects_125hz() {
    assert_frequency("tests/data/125Hz.wav", 0);
}

#[test]
fn detects_250hz() {
    assert_frequency("tests/data/250Hz.wav", 1);
}

#[test]
fn detects_500hz() {
    assert_frequency("tests/data/500Hz.wav", 2);
}

#[test]
fn detects_1000hz() {
    assert_frequency("tests/data/1000Hz.wav", 3);
}

#[test]
fn detects_2000hz() {
    assert_frequency("tests/data/2000Hz.wav", 4);
}

#[test]
fn detects_4000hz() {
    assert_frequency("tests/data/4000Hz.wav", 5);
}

#[test]
fn detects_8000hz() {
    assert_frequency("tests/data/8000Hz.wav", 6);
}

#[test]
fn detects_16000hz() {
    assert_frequency("tests/data/16000Hz.wav", 7);
}