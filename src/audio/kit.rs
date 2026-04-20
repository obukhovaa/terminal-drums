use std::collections::HashMap;
use std::path::Path;

use kira::sound::static_sound::StaticSoundData;
use serde::Deserialize;

use crate::error::AppError;
use crate::midi::types::DrumPiece;

/// Deserialization target for kit.toml.
#[derive(Deserialize)]
struct KitToml {
    kit: KitMeta,
    samples: HashMap<String, String>,
}

#[derive(Deserialize)]
struct KitMeta {
    name: String,
    #[allow(dead_code)]
    author: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
}

/// Represents a loaded drum kit with samples for each piece.
pub struct DrumKit {
    pub name: String,
    pub samples: HashMap<DrumPiece, StaticSoundData>,
}

impl DrumKit {
    /// Load a drum kit from a directory containing kit.toml and sample files.
    ///
    /// Missing samples are non-fatal: the app simply will not play audio for
    /// that piece. Only the kit.toml file is required.
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let toml_path = path.join("kit.toml");
        let toml_str = std::fs::read_to_string(&toml_path).map_err(|e| {
            AppError::Audio(format!("Failed to read {}: {}", toml_path.display(), e))
        })?;
        let kit_toml: KitToml = toml::from_str(&toml_str)
            .map_err(|e| AppError::Audio(format!("Failed to parse kit.toml: {}", e)))?;

        let mut samples = HashMap::new();

        for (key, filename) in &kit_toml.samples {
            let piece = match sample_key_to_drum_piece(key) {
                Some(p) => p,
                None => {
                    eprintln!("Warning: unknown sample key '{}' in kit.toml, skipping", key);
                    continue;
                }
            };

            let sample_path = path.join(filename);
            match StaticSoundData::from_file(&sample_path) {
                Ok(data) => {
                    samples.insert(piece, data);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: could not load sample '{}' ({}): {}",
                        key,
                        sample_path.display(),
                        e
                    );
                }
            }
        }

        Ok(Self {
            name: kit_toml.kit.name,
            samples,
        })
    }
}

/// Map a sample key name from kit.toml to the corresponding DrumPiece.
fn sample_key_to_drum_piece(key: &str) -> Option<DrumPiece> {
    match key {
        "kick" => Some(DrumPiece::Kick),
        "snare" => Some(DrumPiece::Snare),
        "cross_stick" => Some(DrumPiece::CrossStick),
        "hihat_closed" => Some(DrumPiece::ClosedHiHat),
        "hihat_open" => Some(DrumPiece::OpenHiHat),
        "hihat_pedal" => Some(DrumPiece::PedalHiHat),
        "crash1" => Some(DrumPiece::CrashCymbal1),
        "crash2" => Some(DrumPiece::CrashCymbal2),
        "ride" => Some(DrumPiece::RideCymbal),
        "ride_bell" => Some(DrumPiece::RideBell),
        "tom_high" => Some(DrumPiece::HighTom),
        "tom_mid" => Some(DrumPiece::MidTom),
        "tom_low" => Some(DrumPiece::LowTom),
        "splash" => Some(DrumPiece::Splash),
        "china" => Some(DrumPiece::China),
        _ => None,
    }
}

/// Map MIDI velocity (0-127) to a volume multiplier (0.0-1.0).
/// Uses a quadratic curve for more natural dynamics.
pub fn velocity_to_volume(vel: u8) -> f64 {
    let normalized = vel as f64 / 127.0;
    normalized * normalized
}
