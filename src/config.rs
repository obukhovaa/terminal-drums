use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Top-level application configuration, persisted as config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub display: DisplayConfig,
    pub audio: AudioConfig,
    pub playback: PlaybackConfig,
    pub keys: KeysConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub theme: String,
    pub fps: u32,
    pub look_ahead_ms: u32,
    pub show_velocity: bool,
    pub show_note_tails: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub mode: String,
    pub metronome_volume: f64,
    pub kit_volume: f64,
    pub backtrack_volume: f64,
    pub input_offset_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackConfig {
    pub default_bpm: u32,
    pub default_kit: String,
    pub default_difficulty: String,
    pub default_timing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysConfig {
    pub preset: String,
    // Individual overrides (only needed if preset = "custom")
    pub kick: Option<String>,
    pub snare: Option<String>,
    pub cross_stick: Option<String>,
    pub hihat_closed: Option<String>,
    pub hihat_open: Option<String>,
    pub hihat_pedal: Option<String>,
    pub ride: Option<String>,
    pub crash1: Option<String>,
    pub crash2: Option<String>,
    pub tom_high: Option<String>,
    pub tom_mid: Option<String>,
    pub tom_low: Option<String>,
    pub splash: Option<String>,
    pub china: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub tracks_dir: String,
    pub kits_dir: String,
    pub data_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            display: DisplayConfig {
                theme: "gruvbox".to_string(),
                fps: 60,
                look_ahead_ms: 3000,
                show_velocity: true,
                show_note_tails: true,
            },
            audio: AudioConfig {
                mode: "visual_audio".to_string(),
                metronome_volume: 0.7,
                kit_volume: 1.0,
                backtrack_volume: 0.8,
                input_offset_ms: 0,
            },
            playback: PlaybackConfig {
                default_bpm: 0,
                default_kit: "acoustic".to_string(),
                default_difficulty: "hard".to_string(),
                default_timing: "standard".to_string(),
            },
            keys: KeysConfig {
                preset: "split".to_string(),
                kick: None,
                snare: None,
                cross_stick: None,
                hihat_closed: None,
                hihat_open: None,
                hihat_pedal: None,
                ride: None,
                crash1: None,
                crash2: None,
                tom_high: None,
                tom_mid: None,
                tom_low: None,
                splash: None,
                china: None,
            },
            paths: PathsConfig {
                tracks_dir: "~/.config/terminal-drums/tracks".to_string(),
                kits_dir: "~/.config/terminal-drums/kits".to_string(),
                data_dir: "~/.local/share/terminal-drums".to_string(),
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from `path`.
    ///
    /// If the file does not exist, returns `Default::default()` (first-run
    /// behaviour — the defaults will be written on the next `save()` call).
    /// Any other I/O or parse error is propagated.
    pub fn load(path: &Path) -> Result<Self, AppError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)?;
        toml::from_str(&contents).map_err(|e| AppError::Config(e.to_string()))
    }

    /// Serialize and write the configuration to `path`.
    ///
    /// The parent directory must already exist (call `ensure_dirs()` first).
    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        let contents =
            toml::to_string_pretty(self).map_err(|e| AppError::Config(e.to_string()))?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Returns `~/.config/terminal-drums/`.
    ///
    /// Falls back to `$HOME/.config/terminal-drums/` if the `dirs` crate
    /// cannot determine the config directory.
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
            })
            .join("terminal-drums")
    }

    /// Returns `~/.local/share/terminal-drums/`.
    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".local")
                    .join("share")
            })
            .join("terminal-drums")
    }

    /// Create the config and data directories (and their subdirectories) if
    /// they don't already exist.
    pub fn ensure_dirs() -> Result<(), AppError> {
        let config = Self::config_dir();
        fs::create_dir_all(config.join("tracks"))?;
        fs::create_dir_all(config.join("kits"))?;
        fs::create_dir_all(Self::data_dir())?;
        Ok(())
    }

    /// Returns the canonical path to the config file.
    pub fn default_config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Returns the canonical path to the SQLite database file.
    pub fn default_db_path() -> PathBuf {
        Self::data_dir().join("scores.db")
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_default_roundtrip() {
        let original = AppConfig::default();
        let serialized =
            toml::to_string_pretty(&original).expect("serialize");
        let deserialized: AppConfig =
            toml::from_str(&serialized).expect("deserialize");

        // Spot-check a handful of fields.
        assert_eq!(deserialized.display.theme, original.display.theme);
        assert_eq!(deserialized.display.fps, original.display.fps);
        assert_eq!(deserialized.audio.kit_volume, original.audio.kit_volume);
        assert_eq!(deserialized.playback.default_difficulty, original.playback.default_difficulty);
        assert_eq!(deserialized.keys.preset, original.keys.preset);
        assert_eq!(deserialized.paths.tracks_dir, original.paths.tracks_dir);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let cfg = AppConfig::load(&path).expect("load missing file");
        assert_eq!(cfg.display.theme, "gruvbox");
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let mut cfg = AppConfig::default();
        cfg.display.theme = "desert".to_string();
        cfg.display.fps = 30;
        cfg.audio.input_offset_ms = 12;
        cfg.keys.kick = Some("x".to_string());

        cfg.save(&path).expect("save");

        let loaded = AppConfig::load(&path).expect("load");
        assert_eq!(loaded.display.theme, "desert");
        assert_eq!(loaded.display.fps, 30);
        assert_eq!(loaded.audio.input_offset_ms, 12);
        assert_eq!(loaded.keys.kick.as_deref(), Some("x"));
    }

    #[test]
    fn test_config_dir_is_non_empty() {
        let dir = AppConfig::config_dir();
        assert!(dir.to_str().unwrap().contains("terminal-drums"));
    }

    #[test]
    fn test_data_dir_is_non_empty() {
        let dir = AppConfig::data_dir();
        assert!(dir.to_str().unwrap().contains("terminal-drums"));
    }
}
