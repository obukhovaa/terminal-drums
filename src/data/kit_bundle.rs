use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;
use crate::midi::types::DrumPiece;

// ---------------------------------------------------------------------------
// TOML deserialization helpers — mirrors kit.toml from spec §13.2
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct KitToml {
    kit: KitSection,
    #[serde(default)]
    samples: SamplesSection,
}

#[derive(Debug, Deserialize)]
struct KitSection {
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Default, Deserialize)]
struct SamplesSection {
    kick: Option<String>,
    snare: Option<String>,
    cross_stick: Option<String>,
    hihat_closed: Option<String>,
    hihat_open: Option<String>,
    hihat_pedal: Option<String>,
    crash1: Option<String>,
    crash2: Option<String>,
    ride: Option<String>,
    ride_bell: Option<String>,
    tom_high: Option<String>,
    tom_mid: Option<String>,
    tom_low: Option<String>,
    splash: Option<String>,
    china: Option<String>,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Metadata and sample paths for a discovered kit bundle.
#[derive(Debug, Clone)]
pub struct KitInfo {
    pub name: String,
    pub author: String,
    pub description: String,
    /// Map from drum piece to absolute sample file path.
    pub samples: HashMap<DrumPiece, PathBuf>,
    pub base_dir: PathBuf,
}

/// A discovered kit bundle directory with its metadata.
///
/// Thin wrapper kept for backward compatibility with the existing stub.
pub struct KitBundle {
    pub path: PathBuf,
    pub name: String,
    pub author: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Scan each directory in `dirs` for valid kit bundles and return all
/// successfully loaded `KitInfo` values.  Bundles missing `kit.toml` are
/// skipped silently (non-fatal, per spec §13.2).
pub fn discover_kits(dirs: &[PathBuf]) -> Vec<KitInfo> {
    let mut kits = Vec::new();

    for dir in dirs {
        let read_dir = match fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for entry in read_dir.flatten() {
            let bundle_dir = entry.path();
            if !bundle_dir.is_dir() {
                continue;
            }
            match load_kit_bundle(&bundle_dir) {
                Ok(info) => kits.push(info),
                Err(_) => {
                    // Missing kit.toml or parse error — skip.
                }
            }
        }
    }

    kits
}

/// Load a single kit bundle from `dir`.  Returns an error if `kit.toml` is
/// missing or unparseable.  Missing individual sample files are non-fatal —
/// the corresponding DrumPiece simply won't be in the samples map.
fn load_kit_bundle(dir: &Path) -> Result<KitInfo, AppError> {
    let toml_path = dir.join("kit.toml");
    if !toml_path.exists() {
        return Err(AppError::BundleIncomplete {
            bundle: dir.display().to_string(),
            file: "kit.toml".to_string(),
        });
    }

    let toml_str = fs::read_to_string(&toml_path).map_err(|e| AppError::Config(e.to_string()))?;
    let kit: KitToml =
        toml::from_str(&toml_str).map_err(|e| AppError::Config(e.to_string()))?;

    let mut samples: HashMap<DrumPiece, PathBuf> = HashMap::new();

    // Helper: resolve a sample filename to an absolute path, only if it exists.
    let resolve = |filename: &str| -> Option<PathBuf> {
        let p = dir.join(filename);
        if p.exists() { Some(p) } else { None }
    };

    let s = &kit.samples;
    if let Some(f) = s.kick.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::Kick, f);
    }
    if let Some(f) = s.snare.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::Snare, f);
    }
    if let Some(f) = s.cross_stick.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::CrossStick, f);
    }
    if let Some(f) = s.hihat_closed.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::ClosedHiHat, f);
    }
    if let Some(f) = s.hihat_open.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::OpenHiHat, f);
    }
    if let Some(f) = s.hihat_pedal.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::PedalHiHat, f);
    }
    if let Some(f) = s.crash1.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::CrashCymbal1, f);
    }
    if let Some(f) = s.crash2.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::CrashCymbal2, f);
    }
    if let Some(f) = s.ride.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::RideCymbal, f);
    }
    if let Some(f) = s.ride_bell.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::RideBell, f);
    }
    if let Some(f) = s.tom_high.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::HighTom, f);
    }
    if let Some(f) = s.tom_mid.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::MidTom, f);
    }
    if let Some(f) = s.tom_low.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::LowTom, f);
    }
    if let Some(f) = s.splash.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::Splash, f);
    }
    if let Some(f) = s.china.as_deref().and_then(resolve) {
        samples.insert(DrumPiece::China, f);
    }

    Ok(KitInfo {
        name: kit.kit.name,
        author: kit.kit.author,
        description: kit.kit.description,
        samples,
        base_dir: dir.to_path_buf(),
    })
}

// ---------------------------------------------------------------------------
// KitBundle stub impl (kept for backward compatibility)
// ---------------------------------------------------------------------------

impl KitBundle {
    /// Load a kit bundle from a directory.
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let info = load_kit_bundle(path)?;
        Ok(KitBundle {
            path: path.to_path_buf(),
            name: info.name,
            author: info.author,
            description: info.description,
        })
    }

    /// Discover all kit bundles in a directory.
    pub fn discover(dir: &Path) -> Result<Vec<Self>, AppError> {
        let infos = discover_kits(&[dir.to_path_buf()]);
        Ok(infos
            .into_iter()
            .map(|info| KitBundle {
                path: info.base_dir,
                name: info.name,
                author: info.author,
                description: info.description,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use super::*;

    fn write_minimal_kit(dir: &Path, name: &str) {
        let bundle = dir.join(name);
        fs::create_dir_all(&bundle).unwrap();

        fs::write(
            bundle.join("kit.toml"),
            format!(
                r#"[kit]
name = "{name}"
author = "Test Author"
description = "A test kit"
"#
            ),
        )
        .unwrap();
    }

    fn write_full_kit(dir: &Path, name: &str) {
        write_minimal_kit(dir, name);
        let bundle = dir.join(name);

        // Write fake sample files for a couple of pieces.
        fs::write(bundle.join("kick.wav"), b"fake wav").unwrap();
        fs::write(bundle.join("snare.wav"), b"fake wav").unwrap();

        // Add sample references to the kit.toml.
        let toml = format!(
            r#"[kit]
name = "{name}"
author = "Test Author"
description = "A test kit"

[samples]
kick = "kick.wav"
snare = "snare.wav"
"#
        );
        fs::write(bundle.join("kit.toml"), toml).unwrap();
    }

    #[test]
    fn test_discover_kits_finds_bundles() {
        let tmp = TempDir::new().unwrap();
        write_minimal_kit(tmp.path(), "acoustic");
        write_minimal_kit(tmp.path(), "electronic");

        let kits = discover_kits(&[tmp.path().to_path_buf()]);
        assert_eq!(kits.len(), 2);
        let mut names: Vec<_> = kits.iter().map(|k| k.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["acoustic", "electronic"]);
    }

    #[test]
    fn test_discover_skips_dirs_without_kit_toml() {
        let tmp = TempDir::new().unwrap();

        write_minimal_kit(tmp.path(), "valid-kit");

        // A directory without kit.toml.
        fs::create_dir_all(tmp.path().join("not-a-kit")).unwrap();

        let kits = discover_kits(&[tmp.path().to_path_buf()]);
        assert_eq!(kits.len(), 1);
        assert_eq!(kits[0].name, "valid-kit");
    }

    #[test]
    fn test_sample_paths_resolved() {
        let tmp = TempDir::new().unwrap();
        write_full_kit(tmp.path(), "my-kit");

        let kits = discover_kits(&[tmp.path().to_path_buf()]);
        assert_eq!(kits.len(), 1);

        let kit = &kits[0];
        assert!(kit.samples.contains_key(&DrumPiece::Kick));
        assert!(kit.samples.contains_key(&DrumPiece::Snare));
        // Pieces not listed in kit.toml should be absent.
        assert!(!kit.samples.contains_key(&DrumPiece::China));
    }

    #[test]
    fn test_missing_sample_file_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let bundle = tmp.path().join("my-kit");
        fs::create_dir_all(&bundle).unwrap();

        // kit.toml references a kick but the file doesn't exist.
        fs::write(
            bundle.join("kit.toml"),
            r#"[kit]
name = "my-kit"
author = "X"

[samples]
kick = "kick_missing.wav"
"#,
        )
        .unwrap();

        let kits = discover_kits(&[tmp.path().to_path_buf()]);
        assert_eq!(kits.len(), 1);
        // Missing file → not in samples map.
        assert!(!kits[0].samples.contains_key(&DrumPiece::Kick));
    }
}
