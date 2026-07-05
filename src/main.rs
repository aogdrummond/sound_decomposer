mod audio;
mod audio_processing;
mod display;
mod utils;
mod configs;
use audio_processing::Processor;
use std::env;
use audio::source::AudioFrame;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use utils::init_args::parse_args;
use std::thread;
use std::time::{Duration, Instant};
use std::error::Error;
use audio::source::AudioSource;
use display::source::DisplaySource;
use log::{info,trace,error};
use env_logger::Env;

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

fn process_audio(
    rx_chunk: mpsc::Receiver<AudioFrame>,
    tx_bands: mpsc::Sender<AudioFrame>,
    shutdown: Arc<AtomicBool>,
) {
    let mut processor = Processor::new(configs::CHUNK_SIZE);

    info!("Initiating processing thread.");

    while !shutdown.load(Ordering::SeqCst) {
        match rx_chunk.recv_timeout(Duration::from_millis(100)) {
            Ok(frame) => {
                trace!(
                    "Latency Processing: {:.3} ms",
                    frame.timestamp.elapsed().as_secs_f64() * 1000.0
                );

                let start = Instant::now();
                let bands = processor.process(&frame.samples);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                trace!("Elapsed: {:.3} ms", elapsed);

                let frame2 = AudioFrame {
                    timestamp: Instant::now(),
                    samples: bands,
                };

                if tx_bands.send(frame2).is_err() {
                    info!("Processing: display receiver dropped, stopping.");
                    break;
                }
            }

            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No frame arrived during this period.
                // That's fine: loop again and check shutdown flag.
                continue;
            }

            Err(mpsc::RecvTimeoutError::Disconnected) => {
                info!("Processing: input channel disconnected, stopping.");
                break;
            }
        }
    }

    info!("Processing thread exiting.");
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
    rx_bands: mpsc::Receiver<AudioFrame>,
    shutdown: Arc<AtomicBool>,
)
{
    info!("Initiating display thread.");
    source.display_results(rx_bands, shutdown);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_for_handler = Arc::clone(&shutdown);
    let producer_shutdown = Arc::clone(&shutdown);
    let processing_shutdown = Arc::clone(&shutdown);
    let display_shutdown = Arc::clone(&shutdown);

    ctrlc::set_handler(move || {
        info!("Ctrl+C received. Requesting shutdown...");
        shutdown_for_handler.store(true, Ordering::SeqCst);
    })?;

    match env_logger::Builder::from_env(
        Env::default().default_filter_or("info")
    ).try_init() {
        Ok(()) => info!("Logger initialized."),
        Err(e) => error!("Logger initialization failed: {e}. Continuing without logger."),
    }
        
    let parsed_args = match parse_args() {
    Ok(args) => args,
    Err(e) => {
        error!("{e}");
        return Ok(());
    }
    };

    // Verificar se args são validos. Se não, quit
    info!("Creating audio source '{}'", parsed_args.audio_source_name);
    let audio_source = create_audio_source(&parsed_args.audio_source_name)?;
    info!("Audio source '{}' successfully created", parsed_args.audio_source_name);

    info!("Creating display destination '{}'", parsed_args.display_name);
    let display_source = create_display_source(&parsed_args.display_name)?;
    info!("Display destination '{}' successfully created", parsed_args.display_name);

    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    info!("Source channel opened.");
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();
    info!("Display channel opened.");
    
    let producer_thread = thread::Builder::new()
        .name("producer".into())
        .spawn(move || produce_audio(audio_source, tx_chunk, producer_shutdown))?;
    info!("Producer thread spawned successfully.");

    let processing_thread = thread::Builder::new()
        .name("processing".into())
        .spawn(move || process_audio(rx_chunk, tx_bands, processing_shutdown))?;
    info!("Processing thread spawned successfully.");

    let display_thread = thread::Builder::new()
        .name("display".into())
        .spawn(move || display_results(display_source, rx_bands, display_shutdown))?;
    info!("Display thread spawned successfully.");

    producer_thread
        .join()
        .map_err(|_| "Producer thread panicked")?;

    processing_thread
        .join()
        .map_err(|_| "Processing thread panicked")?;

    display_thread
        .join()
        .map_err(|_| "Display thread panicked")?;

    Ok(())
}