use sound_processor::audio::source::AudioFrame;
use sound_processor::audio_processing::process_audio;
use sound_processor::configs::{CHUNK_SIZE, NUM_BANDS};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn processor_processes_single_frame() {
    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();

    let shutdown = Arc::new(AtomicBool::new(false));
    let processing_shutdown = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        process_audio(rx_chunk, tx_bands, processing_shutdown);
    });

    tx_chunk
        .send(AudioFrame {
            timestamp: Instant::now(),
            samples: vec![0.0; CHUNK_SIZE],
        })
        .unwrap();

    let frame = rx_bands
        .recv_timeout(Duration::from_secs(2))
        .expect("No processed frame received");

    assert_eq!(frame.samples.len(), NUM_BANDS);

    for value in frame.samples {
        assert!(value.is_finite());
        assert!(!value.is_nan());
    }

    shutdown.store(true, Ordering::SeqCst);
    drop(tx_chunk);

    handle.join().unwrap();
}

#[test]
fn processor_skips_empty_frame() {
    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();

    let shutdown = Arc::new(AtomicBool::new(false));
    let processing_shutdown = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        process_audio(rx_chunk, tx_bands, processing_shutdown);
    });

    tx_chunk
        .send(AudioFrame {
            timestamp: Instant::now(),
            samples: Vec::new(),
        })
        .unwrap();

    assert!(rx_bands
        .recv_timeout(Duration::from_millis(300))
        .is_err());

    shutdown.store(true, Ordering::SeqCst);
    drop(tx_chunk);

    handle.join().unwrap();
}

#[test]
fn processor_handles_multiple_frames() {
    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();

    let shutdown = Arc::new(AtomicBool::new(false));
    let processing_shutdown = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        process_audio(rx_chunk, tx_bands, processing_shutdown);
    });

    for _ in 0..5 {
        tx_chunk
            .send(AudioFrame {
                timestamp: Instant::now(),
                samples: vec![0.0; CHUNK_SIZE],
            })
            .unwrap();
    }

    for _ in 0..5 {
        let frame = rx_bands
            .recv_timeout(Duration::from_secs(1))
            .expect("Missing processed frame");

        assert_eq!(frame.samples.len(), NUM_BANDS);
    }

    shutdown.store(true, Ordering::SeqCst);
    drop(tx_chunk);

    handle.join().unwrap();
}

#[test]
fn processor_preserves_timestamp_order() {
    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();

    let shutdown = Arc::new(AtomicBool::new(false));
    let processing_shutdown = Arc::clone(&shutdown);

    let handle = thread::spawn(move || {
        process_audio(rx_chunk, tx_bands, processing_shutdown);
    });

    let before = Instant::now();

    tx_chunk
        .send(AudioFrame {
            timestamp: before,
            samples: vec![0.0; CHUNK_SIZE],
        })
        .unwrap();

    let processed = rx_bands.recv_timeout(Duration::from_secs(1)).unwrap();

    assert!(processed.timestamp >= before);

    shutdown.store(true, Ordering::SeqCst);
    drop(tx_chunk);

    handle.join().unwrap();
}