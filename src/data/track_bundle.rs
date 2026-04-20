use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::AppError;

// ---------------------------------------------------------------------------
// TOML deserialization helpers — mirrors the meta.toml format from spec §13.1
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MetaToml {
    track: TrackSection,
    #[serde(default)]
    midi: MidiSection,
}

#[derive(Debug, Deserialize)]
struct TrackSection {
    name: String,
    artist: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    difficulty_stars: u8,
    default_bpm: u32,
    #[serde(default)]
    genre: String,
}

#[derive(Debug, Default, Deserialize)]
struct MidiSection {
    /// Optional MIDI channel override (1-indexed, as stored in meta.toml).
    channel: Option<u8>,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Metadata and file paths for a discovered track bundle.
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub name: String,
    pub artist: String,
    pub description: String,
    pub difficulty_stars: u8,
    pub default_bpm: u32,
    pub genre: String,
    pub midi_path: PathBuf,
    pub backtrack_path: Option<PathBuf>,
    pub cover_path: Option<PathBuf>,
    /// Optional MIDI channel override (1-indexed).
    pub midi_channel: Option<u8>,
}

/// A discovered track bundle directory with its metadata.
///
/// Thin wrapper kept for backward compat with the existing stub; new code
/// should prefer `TrackInfo`.
pub struct TrackBundle {
    pub path: PathBuf,
    pub name: String,
    pub artist: String,
    pub description: String,
    pub difficulty_stars: u8,
    pub default_bpm: f64,
    pub genre: String,
    pub has_backtrack: bool,
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Scan each directory in `dirs` for valid track bundles and return all
/// successfully loaded `TrackInfo` values.  Invalid or incomplete bundles are
/// skipped silently (non-fatal, per spec §13.1).
pub fn discover_tracks(dirs: &[PathBuf]) -> Vec<TrackInfo> {
    let mut tracks = Vec::new();

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
            match load_track_bundle(&bundle_dir) {
                Ok(info) => tracks.push(info),
                Err(_) => {
                    // Missing required file or parse error — skip bundle.
                }
            }
        }
    }

    tracks
}

/// Load a single track bundle from `dir`.  Returns an error if `meta.toml` or
/// `track.mid` are missing or unparseable.
fn load_track_bundle(dir: &Path) -> Result<TrackInfo, AppError> {
    // Required: meta.toml
    let meta_path = dir.join("meta.toml");
    if !meta_path.exists() {
        return Err(AppError::BundleIncomplete {
            bundle: dir.display().to_string(),
            file: "meta.toml".to_string(),
        });
    }

    let meta_str = fs::read_to_string(&meta_path).map_err(|e| AppError::Config(e.to_string()))?;
    let meta: MetaToml =
        toml::from_str(&meta_str).map_err(|e| AppError::Config(e.to_string()))?;

    // Required: track.mid
    let midi_path = dir.join("track.mid");
    if !midi_path.exists() {
        return Err(AppError::BundleIncomplete {
            bundle: dir.display().to_string(),
            file: "track.mid".to_string(),
        });
    }

    // Optional: backtrack.ogg
    let backtrack_path = {
        let p = dir.join("backtrack.ogg");
        if p.exists() { Some(p) } else { None }
    };

    // Optional: cover.txt
    let cover_path = {
        let p = dir.join("cover.txt");
        if p.exists() { Some(p) } else { None }
    };

    Ok(TrackInfo {
        name: meta.track.name,
        artist: meta.track.artist,
        description: meta.track.description,
        difficulty_stars: meta.track.difficulty_stars,
        default_bpm: meta.track.default_bpm,
        genre: meta.track.genre,
        midi_path,
        backtrack_path,
        cover_path,
        midi_channel: meta.midi.channel,
    })
}

/// Compute the SHA-256 hash of a MIDI file's bytes, returned as a lowercase
/// hex string.
pub fn compute_track_hash(midi_path: &Path) -> Result<String, AppError> {
    let bytes = fs::read(midi_path)?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{:x}", digest))
}

// ---------------------------------------------------------------------------
// TrackBundle stub impl (kept for backward compatibility)
// ---------------------------------------------------------------------------

impl TrackBundle {
    /// Discover and load a track bundle from a directory.
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let info = load_track_bundle(path)?;
        Ok(TrackBundle {
            path: path.to_path_buf(),
            name: info.name,
            artist: info.artist,
            description: info.description,
            difficulty_stars: info.difficulty_stars,
            default_bpm: info.default_bpm as f64,
            genre: info.genre,
            has_backtrack: info.backtrack_path.is_some(),
        })
    }

    /// Discover all track bundles in a directory.
    pub fn discover(dir: &Path) -> Result<Vec<Self>, AppError> {
        let infos = discover_tracks(&[dir.to_path_buf()]);
        Ok(infos
            .into_iter()
            .map(|info| {
                let path = info.midi_path.parent().unwrap_or(Path::new(".")).to_path_buf();
                TrackBundle {
                    has_backtrack: info.backtrack_path.is_some(),
                    path,
                    name: info.name,
                    artist: info.artist,
                    description: info.description,
                    difficulty_stars: info.difficulty_stars,
                    default_bpm: info.default_bpm as f64,
                    genre: info.genre,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use super::*;

    fn write_minimal_bundle(dir: &Path, name: &str) {
        let bundle = dir.join(name);
        fs::create_dir_all(&bundle).unwrap();

        fs::write(
            bundle.join("meta.toml"),
            format!(
                r#"[track]
name = "{name}"
artist = "Test Artist"
description = "Test track"
difficulty_stars = 3
default_bpm = 120
genre = "Rock"
"#
            ),
        )
        .unwrap();

        // Minimal valid MIDI-ish bytes (just needs to exist for testing purposes).
        fs::write(bundle.join("track.mid"), b"MThd").unwrap();
    }

    #[test]
    fn test_discover_tracks_finds_bundles() {
        let tmp = TempDir::new().unwrap();
        write_minimal_bundle(tmp.path(), "basic-rock");
        write_minimal_bundle(tmp.path(), "jazz-waltz");

        let tracks = discover_tracks(&[tmp.path().to_path_buf()]);
        assert_eq!(tracks.len(), 2);
        let mut names: Vec<_> = tracks.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["basic-rock", "jazz-waltz"]);
    }

    #[test]
    fn test_discover_skips_incomplete_bundles() {
        let tmp = TempDir::new().unwrap();

        // Valid bundle.
        write_minimal_bundle(tmp.path(), "valid");

        // Bundle missing track.mid.
        let incomplete = tmp.path().join("incomplete");
        fs::create_dir_all(&incomplete).unwrap();
        fs::write(
            incomplete.join("meta.toml"),
            "[track]\nname = \"x\"\nartist = \"x\"\ndefault_bpm = 100\n",
        )
        .unwrap();

        let tracks = discover_tracks(&[tmp.path().to_path_buf()]);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].name, "valid");
    }

    #[test]
    fn test_optional_files_detected() {
        let tmp = TempDir::new().unwrap();
        let name = "full-bundle";
        write_minimal_bundle(tmp.path(), name);

        let bundle = tmp.path().join(name);
        fs::write(bundle.join("backtrack.ogg"), b"ogg").unwrap();
        fs::write(bundle.join("cover.txt"), b"art").unwrap();

        let tracks = discover_tracks(&[tmp.path().to_path_buf()]);
        assert_eq!(tracks.len(), 1);
        assert!(tracks[0].backtrack_path.is_some());
        assert!(tracks[0].cover_path.is_some());
    }

    #[test]
    fn test_compute_track_hash() {
        let tmp = TempDir::new().unwrap();
        let midi_path = tmp.path().join("track.mid");
        fs::write(&midi_path, b"some midi bytes").unwrap();

        let hash = compute_track_hash(&midi_path).unwrap();
        // SHA-256 is 64 hex chars.
        assert_eq!(hash.len(), 64);
        // Deterministic.
        let hash2 = compute_track_hash(&midi_path).unwrap();
        assert_eq!(hash, hash2);
    }
}
