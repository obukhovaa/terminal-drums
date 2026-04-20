use std::path::Path;

use kira::manager::AudioManager;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState};
use kira::track::TrackHandle;
use kira::tween::Tween;
use kira::Volume;

use crate::error::AppError;

/// Manages streaming backtrack audio playback.
///
/// The backtrack is a long audio file (e.g., a full song minus drums)
/// that plays in sync with the MIDI track. It uses kira's streaming
/// sound to avoid loading the entire file into memory.
pub struct BacktrackPlayer {
    /// Handle to the currently playing/paused streaming sound.
    /// None if no backtrack is loaded or if it has been stopped.
    handle: Option<StreamingSoundHandle<FromFileError>>,
}

impl BacktrackPlayer {
    /// Create a new empty backtrack player.
    pub fn new() -> Self {
        Self { handle: None }
    }

    /// Load a backtrack audio file and return the StreamingSoundData
    /// ready to be played. Does not start playback yet.
    pub fn load_file(path: &Path) -> Result<StreamingSoundData<FromFileError>, AppError> {
        StreamingSoundData::from_file(path)
            .map_err(|e| AppError::Audio(format!("Failed to load backtrack: {}", e)))
    }

    /// Start playing a backtrack on the given track. If a backtrack is
    /// already playing, it is stopped first.
    pub fn play(
        &mut self,
        sound_data: StreamingSoundData<FromFileError>,
        manager: &mut AudioManager,
        track: &TrackHandle,
    ) -> Result<(), AppError> {
        // Stop any existing backtrack
        self.stop();

        let sound = sound_data.output_destination(track);
        let handle = manager
            .play(sound)
            .map_err(|e| AppError::Audio(format!("Failed to play backtrack: {}", e)))?;
        self.handle = Some(handle);
        Ok(())
    }

    /// Pause backtrack playback. Can be resumed later.
    pub fn pause(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.pause(Tween::default());
        }
    }

    /// Resume backtrack playback after a pause.
    pub fn resume(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.resume(Tween::default());
        }
    }

    /// Seek the backtrack to a position in seconds.
    pub fn seek(&mut self, position_seconds: f64) {
        if let Some(handle) = &mut self.handle {
            handle.seek_to(position_seconds);
        }
    }

    /// Seek the backtrack to a position in milliseconds.
    pub fn seek_ms(&mut self, position_ms: f64) {
        self.seek(position_ms / 1000.0);
    }

    /// Stop and discard the current backtrack.
    pub fn stop(&mut self) {
        if let Some(mut handle) = self.handle.take() {
            handle.stop(Tween::default());
        }
    }

    /// Returns the current playback position in seconds, or None if
    /// no backtrack is loaded.
    pub fn position(&self) -> Option<f64> {
        self.handle.as_ref().map(|h| h.position())
    }

    /// Returns the current playback position in milliseconds, or None
    /// if no backtrack is loaded.
    pub fn position_ms(&self) -> Option<f64> {
        self.position().map(|p| p * 1000.0)
    }

    /// Returns true if a backtrack is currently loaded (playing or paused).
    pub fn is_loaded(&self) -> bool {
        self.handle.is_some()
    }

    /// Returns the playback state of the backtrack.
    pub fn state(&self) -> Option<PlaybackState> {
        self.handle.as_ref().map(|h| h.state())
    }

    /// Check for drift between the backtrack position and the MIDI
    /// position. Returns the drift in milliseconds (positive = backtrack
    /// is ahead of MIDI, negative = behind).
    ///
    /// Returns None if no backtrack is loaded.
    pub fn drift_ms(&self, midi_position_ms: f64) -> Option<f64> {
        self.position_ms().map(|bt_ms| bt_ms - midi_position_ms)
    }

    /// Correct drift by seeking the backtrack to match the MIDI position.
    /// Call this when drift exceeds the acceptable threshold (e.g., 20ms).
    pub fn correct_drift(&mut self, midi_position_ms: f64) {
        self.seek_ms(midi_position_ms);
    }

    /// Set the track volume (convenience for mute controls).
    pub fn set_volume(track: &mut TrackHandle, volume: f64) {
        track.set_volume(Volume::Amplitude(volume), Tween::default());
    }
}

impl Default for BacktrackPlayer {
    fn default() -> Self {
        Self::new()
    }
}
