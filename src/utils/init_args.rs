use std::env;
use std::error::Error;

const DEFAULT_DISPLAY: &str = "terminal";
const DEFAULT_AUDIO_SOURCE: &str = "mic";

const VALID_DISPLAYS: &[&str] = &["terminal", "bars", "oled"];
const VALID_AUDIO_SOURCES: &[&str] = &["mic", "wav"];

pub struct AppArgs {
    pub display_name: String,
    pub audio_source_name: String,
}

pub fn parse_args() -> Result<AppArgs, Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect(); // skip program name

    match args.len() {
        0 => {
            // No arguments provided -> use defaults
            Ok(AppArgs {
                display_name: DEFAULT_DISPLAY.to_string(),
                audio_source_name: DEFAULT_AUDIO_SOURCE.to_string(),
            })
        }

        1 => {
            Err(format!(
                "Only one argument was provided ('{}').\n\
                 Expected either:\n\
                 - no arguments (defaults: {} {})\n\
                 - two arguments: <display> <audio_source>\n\
                 Valid displays: {}\n\
                 Valid audio sources: {}",
                args[0],
                DEFAULT_DISPLAY,
                DEFAULT_AUDIO_SOURCE,
                VALID_DISPLAYS.join(", "),
                VALID_AUDIO_SOURCES.join(", ")
            ).into())
        }

        2 => {
            let display = args[0].as_str();
            let audio = args[1].as_str();

            if !VALID_DISPLAYS.contains(&display) {
                return Err(format!(
                    "Invalid display '{}'. Valid options are: {}",
                    display,
                    VALID_DISPLAYS.join(", ")
                ).into());
            }

            if !VALID_AUDIO_SOURCES.contains(&audio) {
                return Err(format!(
                    "Invalid audio source '{}'. Valid options are: {}",
                    audio,
                    VALID_AUDIO_SOURCES.join(", ")
                ).into());
            }

            Ok(AppArgs {
                display_name: display.to_string(),
                audio_source_name: audio.to_string(),
            })
        }

        _ => Err(format!(
            "Too many arguments.\n\
             Expected either:\n\
             - no arguments (defaults: {} {})\n\
             - two arguments: <display> <audio_source>\n\
             Valid displays: {}\n\
             Valid audio sources: {}",
            DEFAULT_DISPLAY,
            DEFAULT_AUDIO_SOURCE,
            VALID_DISPLAYS.join(", "),
            VALID_AUDIO_SOURCES.join(", ")
        ).into()),
    }
}