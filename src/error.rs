use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("MIDI parsing failed: {0}")]
    MidiParse(String),

    #[error("No drum data found in MIDI file")]
    NoDrumData,

    #[error("Audio engine error: {0}")]
    Audio(String),

    #[error("Kit sample not found: {path}")]
    SampleNotFound { path: PathBuf },

    #[error("Track bundle missing required file: {file} in {bundle}")]
    BundleIncomplete { bundle: String, file: String },

    #[error("Config error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Terminal error: {0}")]
    Terminal(#[from] std::io::Error),

    #[error("Terminal too small: need {need_cols}x{need_rows}, got {have_cols}x{have_rows}")]
    TerminalTooSmall {
        need_cols: u16,
        need_rows: u16,
        have_cols: u16,
        have_rows: u16,
    },
}
