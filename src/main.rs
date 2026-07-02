mod audio;
mod audio_processing;
mod display;
mod utils;
use audio_processing::Processor;
use std::env;
use audio::source::AudioFrame;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use std::error::Error;
use audio::source::AudioSource;
use display::source::DisplaySource;
use log::{debug, error, info, warn};

fn create_audio_source(
    name: &str,
) -> Result<Box<dyn AudioSource>, Box<dyn Error>> {
    match name {
        "mic" => Ok(Box::new(audio::mic::MicrophoneSource::new()?)),
        "wav" => Ok(Box::new(audio::wav::WavSource::new()?)),
        other => Err(format!("Unknown audio source '{}'", other).into()),
    }
}

fn produce_audio(
    mut source: Box<dyn audio::source::AudioSource>,
    tx_chunk: mpsc::Sender<AudioFrame>)
{
    while let Some(chunk) = source.next_chunk() {
        // Include here the timestamp
        let frame = AudioFrame{timestamp: Instant::now(),
                               samples: chunk};
        if tx_chunk.send(frame).is_err() {
            break;
        }
    }
}

fn process_audio(rx_chunk: mpsc::Receiver<AudioFrame>,
                 tx_bands: mpsc::Sender<AudioFrame>){
    let mut processor = Processor::new(audio::wav::CHUNK_SIZE);

    while let Ok(frame) = rx_chunk.recv() {
        println!("Latency Processing: {:.3} ms",
            frame.timestamp.elapsed().as_secs_f64() * 1000.0
        );
        let start = Instant::now();

        let bands = processor.process(&frame.samples);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        println!("Elapsed: {:.3} ms", elapsed);
        let frame2 = AudioFrame{timestamp: Instant::now(),
                                    samples: bands};
        if tx_bands.send(frame2).is_err() {
            break;
        }
    }

    println!("Processing finished");
}

fn create_display_source(
    name: &str,
) -> Result<Box<dyn DisplaySource>, Box<dyn Error>> {
    match name {
        "terminal" => Ok(Box::new(display::terminal::TerminalDisplay::new()?)),
        "bars" => Ok(Box::new(display::terminal_bars::TerminalBars::new()?)),
        "oled" => Ok(Box::new(display::oled_bars::OledBars::new()?)),
        other => Err(format!("Unknown display '{}'", other).into()),
    }
}

fn display_results(
    mut source: Box<dyn display::source::DisplaySource>,
    rx_bands: mpsc::Receiver<AudioFrame>)
{
    source.display_results(rx_bands);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(
        Env::default().default_filter_or("info")
    ).init();

    let args: Vec<String> = env::args().collect();

    let display_name = args.get(1).map(String::as_str).unwrap_or("terminal");
    let audio_source_name = args.get(2).map(String::as_str).unwrap_or("mic");

    info!("Creating audio source '{}'", audio_source_name);
    let audio_source = create_audio_source(audio_source_name)?;
    info!("Audio source '{}' successfully created", audio_source_name);

    info!("Creating display destination '{}'", display_name);
    let display_source = create_display_source(display_name)?;
    info!("Display destination '{}' successfully created", display_name);

    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();

    let producer_thread = thread::spawn(move || produce_audio(audio_source, tx_chunk));
    let processing_thread = thread::spawn(move || process_audio(rx_chunk, tx_bands));
    let display_thread = thread::spawn(move || display_results(display_source, rx_bands));

    producer_thread.join().unwrap();
    processing_thread.join().unwrap();
    display_thread.join().unwrap();

    Ok(())
}