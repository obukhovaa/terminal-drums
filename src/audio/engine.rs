use std::collections::HashMap;
use std::path::PathBuf;

use kira::manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings};
use kira::sound::static_sound::StaticSoundData;
use kira::track::{TrackBuilder, TrackHandle};
use kira::tween::Tween;
use kira::Volume;

use crate::error::AppError;
use crate::midi::types::DrumPiece;

use super::kit::velocity_to_volume;

/// Identifies which audio sub-track to target for volume/mute operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTrack {
    Metronome,
    Kit,
    Backtrack,
}

/// The audio engine wraps kira's AudioManager and manages sub-tracks
/// for metronome, drum kit, and backtrack audio.
///
/// Three independent sub-tracks allow per-category volume control and
/// muting without affecting other audio categories.
pub struct AudioEngine {
    manager: AudioManager,
    metronome_track: TrackHandle,
    kit_track: TrackHandle,
    backtrack_track: TrackHandle,
    /// Loaded kit samples, keyed by drum piece. StaticSoundData is cheap
    /// to clone (audio data is reference-counted).
    kit_samples: HashMap<DrumPiece, StaticSoundData>,
    /// Optional loaded metronome hi (downbeat) sample.
    metronome_hi: Option<StaticSoundData>,
    /// Optional loaded metronome lo (other beats) sample.
    metronome_lo: Option<StaticSoundData>,
}

impl AudioEngine {
    /// Create a new AudioEngine with default backend settings and three
    /// sub-tracks (metronome, kit, backtrack).
    pub fn new() -> Result<Self, AppError> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| AppError::Audio(format!("Failed to create AudioManager: {}", e)))?;

        let metronome_track = manager
            .add_sub_track(TrackBuilder::default())
            .map_err(|e| AppError::Audio(format!("Failed to create metronome track: {}", e)))?;

        let kit_track = manager
            .add_sub_track(TrackBuilder::default())
            .map_err(|e| AppError::Audio(format!("Failed to create kit track: {}", e)))?;

        let backtrack_track = manager
            .add_sub_track(TrackBuilder::default())
            .map_err(|e| AppError::Audio(format!("Failed to create backtrack track: {}", e)))?;

        Ok(Self {
            manager,
            metronome_track,
            kit_track,
            backtrack_track,
            kit_samples: HashMap::new(),
            metronome_hi: None,
            metronome_lo: None,
        })
    }

    /// Returns a mutable reference to the underlying kira AudioManager.
    /// Needed by Metronome and BacktrackPlayer to play sounds.
    pub fn manager_mut(&mut self) -> &mut AudioManager {
        &mut self.manager
    }

    /// Returns a reference to the metronome track handle.
    pub fn metronome_track(&self) -> &TrackHandle {
        &self.metronome_track
    }

    /// Returns a reference to the kit track handle.
    pub fn kit_track(&self) -> &TrackHandle {
        &self.kit_track
    }

    /// Returns a reference to the backtrack track handle.
    pub fn backtrack_track(&self) -> &TrackHandle {
        &self.backtrack_track
    }

    /// Load drum kit samples from a mapping of DrumPiece -> file path.
    ///
    /// Missing samples are non-fatal: a warning is printed and the piece
    /// is skipped. Previous samples are replaced.
    pub fn load_kit(&mut self, samples: &HashMap<DrumPiece, PathBuf>) -> Result<(), AppError> {
        self.kit_samples.clear();

        for (piece, path) in samples {
            match StaticSoundData::from_file(path) {
                Ok(data) => {
                    self.kit_samples.insert(*piece, data);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: could not load sample for {:?} ({}): {}",
                        piece,
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Load kit samples directly from a pre-loaded HashMap (e.g., from DrumKit).
    pub fn load_kit_samples(&mut self, samples: HashMap<DrumPiece, StaticSoundData>) {
        self.kit_samples = samples;
    }

    /// Load metronome click samples.
    pub fn load_metronome_samples(
        &mut self,
        hi_path: &std::path::Path,
        lo_path: &std::path::Path,
    ) -> Result<(), AppError> {
        self.metronome_hi = Some(
            StaticSoundData::from_file(hi_path)
                .map_err(|e| AppError::Audio(format!("Failed to load metronome hi: {}", e)))?,
        );
        self.metronome_lo = Some(
            StaticSoundData::from_file(lo_path)
                .map_err(|e| AppError::Audio(format!("Failed to load metronome lo: {}", e)))?,
        );
        Ok(())
    }

    /// Trigger a drum hit sound for the given piece at the given velocity.
    ///
    /// The sample is played on the kit sub-track with volume scaled by the
    /// velocity curve. If no sample is loaded for the piece, this is a no-op.
    pub fn trigger_hit(&mut self, piece: DrumPiece, velocity: u8) -> Result<(), AppError> {
        let sound_data = match self.kit_samples.get(&piece) {
            Some(data) => data,
            None => return Ok(()), // No sample loaded for this piece
        };

        let volume = velocity_to_volume(velocity);
        let sound = sound_data
            .volume(Volume::Amplitude(volume))
            .output_destination(&self.kit_track);

        self.manager
            .play(sound)
            .map_err(|e| AppError::Audio(format!("Failed to trigger drum hit: {}", e)))?;

        Ok(())
    }

    /// Set the volume of a specific audio track (0.0 = silent, 1.0 = full).
    pub fn set_track_volume(&mut self, track: AudioTrack, volume: f64) {
        let handle = match track {
            AudioTrack::Metronome => &mut self.metronome_track,
            AudioTrack::Kit => &mut self.kit_track,
            AudioTrack::Backtrack => &mut self.backtrack_track,
        };
        handle.set_volume(Volume::Amplitude(volume), Tween::default());
    }

    /// Mute or unmute a specific audio track.
    ///
    /// Muting is implemented via track volume (set to 0) rather than
    /// skipping play commands, so un-muting during playback resumes
    /// audio seamlessly.
    pub fn mute_track(&mut self, track: AudioTrack, muted: bool) {
        let volume = if muted { 0.0 } else { 1.0 };
        self.set_track_volume(track, volume);
    }

    /// Returns true if a sample is loaded for the given drum piece.
    pub fn has_sample(&self, piece: &DrumPiece) -> bool {
        self.kit_samples.contains_key(piece)
    }

    /// Returns the number of loaded kit samples.
    pub fn loaded_sample_count(&self) -> usize {
        self.kit_samples.len()
    }

    /// Returns a mutable reference to the manager and an immutable reference
    /// to the metronome track, avoiding borrow-checker conflicts.
    pub fn manager_and_metronome_track(&mut self) -> (&mut AudioManager, &TrackHandle) {
        (&mut self.manager, &self.metronome_track)
    }

    /// Returns a mutable reference to the manager and an immutable reference
    /// to the backtrack track handle.
    pub fn manager_and_backtrack_track(&mut self) -> (&mut AudioManager, &TrackHandle) {
        (&mut self.manager, &self.backtrack_track)
    }
}
