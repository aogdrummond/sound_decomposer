// Frequency settings
pub const NUM_BANDS: usize = 8;

pub const CENTRAL_FREQS: [f32; NUM_BANDS] = [
    125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const BAND_LABELS: [&str; NUM_BANDS] = [
    "125", "250", "500", "1K", "2K", "4K", "8K", "16K",
];
//Wav Settings
pub const CHUNK_SIZE:usize = 4096;
pub const WAV_FILE: &str = "./data/file_example_WAV_2MG.wav";

// Audio Settings
pub const SAMPLE_RATE: f32 = 48_000.0;
// Display Settings
pub const UPDATE_INTERVAL_MS: u64 = 100;
pub const SCREEN_WIDTH : usize = 128;
pub const GRAPH_HEIGHT: usize = 56;
pub const I2C_PERIPHERAL_PATH: &str = "/dev/i2c-1";

// Terminal bars settings

pub const BAR_WIDTH: usize = 50;
pub const BLOCKS: [&str; 9] = [
    " ",
    "▏",
    "▎",
    "▍",
    "▌",
    "▋",
    "▊",
    "▉",
    "█",
];