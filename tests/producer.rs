use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use sound_processor::audio::factory::produce_audio;
use sound_processor::audio::source::AudioFrame;
use sound_processor::audio::wav::WavSource;
use sound_processor::configs::CHUNK_SIZE;

#[test]
fn producer_generates_audio_frames() {
    let source = Box::new(
        WavSource::new("tests/data/500Hz.wav")
            .expect("Unable to open test WAV"),
    );

    let (tx, rx) = mpsc::channel::<AudioFrame>();

    let shutdown = Arc::new(AtomicBool::new(false));
    let producer_shutdown = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        produce_audio(source, tx, producer_shutdown);
    });

    // Receive one frame
    let frame = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Producer did not send a frame");

    assert_eq!(frame.samples.len(), CHUNK_SIZE);

    // Timestamp should be recent
    assert!(frame.timestamp.elapsed() < Duration::from_secs(2));

    // Stop producer
    shutdown.store(true, Ordering::SeqCst);

    handle.join().unwrap();
}

#[test]
fn producer_generates_multiple_frames() {
    let source = Box::new(
        WavSource::new("tests/data/500Hz.wav")
            .expect("Unable to open test WAV"),
    );

    let (tx, rx) = mpsc::channel::<AudioFrame>();

    let shutdown = Arc::new(AtomicBool::new(false));
    let producer_shutdown = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        produce_audio(source, tx, producer_shutdown);
    });

    for _ in 0..5 {
        let frame = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("Missing frame");

        assert_eq!(frame.samples.len(), CHUNK_SIZE);
    }

    shutdown.store(true, Ordering::SeqCst);

    handle.join().unwrap();
}