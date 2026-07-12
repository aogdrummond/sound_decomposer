mod audio;
mod audio_processing;
mod display;
mod utils;
mod configs;
use audio_processing::Processor;
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
use log::{info,trace,error,warn};
use env_logger::Env;

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

fn create_audio_source(
    name: &str,
) -> Result<Box<dyn AudioSource>, Box<dyn Error>> {

    match name {

        "mic" => initialize_with_fallback(
            "Microphone",
            "WAV source",
            || audio::mic::MicrophoneSource::new(),
            || audio::wav::WavSource::new(),
        ),

        "wav" => initialize_audio_source(
            audio::wav::WavSource::new()?
        ),

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
                // Here guarantee that frame.samples ain't empty. If it is, skip.
                if frame.samples.is_empty() {
                    info!("Processing: received empty frame, skipping.");
                    continue;
                } 
                let start = Instant::now();
                let bands = processor.process(&frame.samples);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                trace!("Elapsed: {:.3} ms", elapsed);

                let frame2 = AudioFrame {
                    timestamp: Instant::now(),
                    samples: bands,
                };

                if tx_bands.send(frame2).is_err() {
                    error!("Processing: Sending data for display failed, dropping it.");
                    // It informs it failed and then returns processing, keep trying to send forever
                    //
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
        "oled" => {
            info!("Trying to initialize OLED display...");

            match display::oled_bars::OledBars::new() {
                Ok(mut oled) => {
                    if let Err(e) = oled.self_test() {
                        error!("OLED self-test failed: {e}");
                        warn!("Falling back to terminal display.");

                        let mut terminal = display::terminal::TerminalDisplay::new()?;
                        terminal.self_test()?;
                        return Ok(Box::new(terminal));
                    }

                    info!("OLED initialized successfully.");
                    Ok(Box::new(oled))
                }

                Err(e) => {
                    error!("Unable to initialize OLED: {e}");
                    warn!("Falling back to terminal display.");

                    let mut terminal = display::terminal::TerminalDisplay::new()?;
                    terminal.self_test()?;
                    Ok(Box::new(terminal))
                }
            }
        }

        "bars" => {
            info!("Trying to initialize bar display...");

            match display::terminal_bars::TerminalBars::new() {
                Ok(mut bars) => {
                    bars.self_test()?;
                    Ok(Box::new(bars))
                }

                Err(e) => {
                    error!("Unable to initialize bar display: {e}");
                    warn!("Falling back to terminal display.");

                    let mut terminal = display::terminal::TerminalDisplay::new()?;
                    terminal.self_test()?;
                    Ok(Box::new(terminal))
                }
            }
        }

        "terminal" => {
            let mut terminal = display::terminal::TerminalDisplay::new()?;
            terminal.self_test()?;
            Ok(Box::new(terminal))
        }

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

    info!("Creating audio source '{}'", parsed_args.audio_source_name);
    let mut audio_source = create_audio_source(&parsed_args.audio_source_name)?;
    // audio_source.self_test()?;
    info!("Audio source '{}' successfully created", parsed_args.audio_source_name);

    info!("Creating display destination '{}'", parsed_args.display_name);
    let mut display_source = create_display_source(&parsed_args.display_name)?;
    info!("Display destination '{}' successfully created", parsed_args.display_name);
    // display_source.self_test()?;

    let (tx_chunk, rx_chunk) = mpsc::channel::<AudioFrame>();
    info!("Source channel opened.");
    let (tx_bands, rx_bands) = mpsc::channel::<AudioFrame>();
    info!("Display channel opened.");
    
    let producer_thread = thread::Builder::new().name("producer".into())
        .spawn(move || produce_audio(audio_source, tx_chunk, producer_shutdown))?;
    info!("Producer thread spawned successfully.");

    let processing_thread = thread::Builder::new().name("processing".into())
        .spawn(move || process_audio(rx_chunk, tx_bands, processing_shutdown))?;
    info!("Processing thread spawned successfully.");

    let display_thread = thread::Builder::new().name("display".into())
        .spawn(move || display_results(display_source, rx_bands, display_shutdown))?;
    info!("Display thread spawned successfully.");

    producer_thread.join().map_err(|_| "Producer thread panicked")?;

    processing_thread.join().map_err(|_| "Processing thread panicked")?;

    display_thread.join().map_err(|_| "Display thread panicked")?;

    Ok(())
}