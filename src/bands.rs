pub const NUM_BANDS: usize = 8;

pub const CENTRAL_FREQS: [f32; NUM_BANDS] = [
    125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const BAND_LABELS: [&str; NUM_BANDS] = [
    "125", "250", "500", "1K", "2K", "4K", "8K", "16K",
];