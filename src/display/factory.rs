use std::error::Error;
use log::{info,trace,error,warn};

use super::{
    source::DisplaySource,
    oled_bars::OledBars,
    terminal::TerminalDisplay,
    terminal_bars::TerminalBars,
};

pub fn create_display_source(
    name: &str,
) -> Result<Box<dyn DisplaySource>, Box<dyn Error>> {
    match name {

        "oled" => initialize_display_with_fallback(
            "OLED display",
            "Terminal display",
            || display::oled_bars::OledBars::new(),
            || display::terminal::TerminalDisplay::new(),
        ),

        "bars" => initialize_display_with_fallback(
            "Bar display",
            "Terminal display",
            || display::terminal_bars::TerminalBars::new(),
            || display::terminal::TerminalDisplay::new(),
        ),

        "terminal" => initialize_display_source(
            display::terminal::TerminalDisplay::new()?
        ),

        other => Err(format!("Unknown display '{}'", other).into()),
    }
}


fn initialize_display_source<T>(
    mut source: T,
) -> Result<Box<dyn DisplaySource>, Box<dyn Error>>
where
    T: DisplaySource + 'static,
{
    source.self_test()?;
    Ok(Box::new(source))
}

fn initialize_display_with_fallback<P, B>(
    primary_name: &str,
    backup_name: &str,
    primary: impl FnOnce() -> Result<P, Box<dyn Error>>,
    backup: impl FnOnce() -> Result<B, Box<dyn Error>>,
) -> Result<Box<dyn DisplaySource>, Box<dyn Error>>
where
    P: DisplaySource + 'static,
    B: DisplaySource + 'static,
{
    info!("Trying to initialize {primary_name}...");

    match primary() {
        Ok(source) => match initialize_display_source(source) {
            Ok(source) => {
                info!("{primary_name} initialized successfully.");
                Ok(source)
            }

            Err(e) => {
                error!("{primary_name} self-test failed: {e}");
                warn!("Falling back to {backup_name}.");

                initialize_display_source(backup()?)
            }
        },

        Err(e) => {
            error!("Unable to initialize {primary_name}: {e}");
            warn!("Falling back to {backup_name}.");

            initialize_display_source(backup()?)
        }
    }
}