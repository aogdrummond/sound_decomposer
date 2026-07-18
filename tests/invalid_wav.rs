use sound_processor::audio::wav::WavSource;

#[test]
fn opening_nonexistent_file_returns_error() {
    let source = WavSource::new("tests/data/does_not_exist.wav");

    assert!(source.is_err());
}

#[test]
fn opening_directory_returns_error() {
    let source = WavSource::new("tests/data");

    assert!(source.is_err());
}

#[test]
fn opening_empty_path_returns_error() {
    let source = WavSource::new("");

    assert!(source.is_err());
}

#[test]
fn opening_text_file_returns_error() {
    let source = WavSource::new("tests/data/not_a_wav.txt");

    assert!(source.is_err());
}
