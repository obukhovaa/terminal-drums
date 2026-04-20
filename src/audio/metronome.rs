use std::path::Path;

use kira::manager::AudioManager;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use kira::tween::Tween;
use kira::Volume;

use crate::error::AppError;

/// Manages metronome audio, triggering hi/lo click sounds on the beat.
///
/// The metronome is driven by the game loop: each tick, the game loop
/// passes in the current beat position. The metronome tracks which beat
/// it last triggered and fires the appropriate sample when a new beat
/// boundary is crossed.
pub struct Metronome {
    /// Sound for the downbeat (beat 0 of each bar).
    hi_sound: StaticSoundData,
    /// Sound for other beats.
    lo_sound: StaticSoundData,
    /// Current BPM (informational; scheduling is beat-driven).
    bpm: f64,
    /// Beats per bar (numerator of time signature).
    numerator: u8,
    /// The last integer beat we triggered a click for, within the bar.
    /// None means no beat has been triggered yet.
    last_triggered_beat: Option<u32>,
    /// The last absolute beat index triggered (to avoid double-triggers).
    last_absolute_beat: Option<u64>,
}

impl Metronome {
    /// Create a new metronome by loading the hi (downbeat) and lo (other beat)
    /// click samples from the given file paths.
    pub fn new(hi_path: &Path, lo_path: &Path) -> Result<Self, AppError> {
        let hi_sound = StaticSoundData::from_file(hi_path)
            .map_err(|e| AppError::Audio(format!("Failed to load metronome hi: {}", e)))?;
        let lo_sound = StaticSoundData::from_file(lo_path)
            .map_err(|e| AppError::Audio(format!("Failed to load metronome lo: {}", e)))?;

        Ok(Self {
            hi_sound,
            lo_sound,
            bpm: 120.0,
            numerator: 4,
            last_triggered_beat: None,
            last_absolute_beat: None,
        })
    }

    /// Update the BPM. This is informational for the metronome since
    /// beat-based triggering is driven externally.
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
    }

    /// Update the time signature. Only the numerator matters for
    /// metronome click patterns (beats per bar).
    pub fn set_time_signature(&mut self, numerator: u8, _denominator: u8) {
        self.numerator = numerator;
    }

    /// Reset the metronome state (e.g., when playback restarts).
    pub fn reset(&mut self) {
        self.last_triggered_beat = None;
        self.last_absolute_beat = None;
    }

    /// Called by the game loop each tick with the current beat position
    /// (absolute beat count from the start of the track, as a float).
    ///
    /// When a new integer beat boundary is crossed, the appropriate
    /// click sound is triggered on the given track.
    ///
    /// `absolute_beat` is the total beat position (e.g., beat 9.3 means
    /// we are 0.3 into the 10th beat). The metronome uses the numerator
    /// to determine which beat within the bar this is.
    pub fn tick(
        &mut self,
        absolute_beat: f64,
        manager: &mut AudioManager,
        track: &TrackHandle,
    ) -> Result<(), AppError> {
        if absolute_beat < 0.0 {
            return Ok(());
        }

        let beat_index = absolute_beat.floor() as u64;

        // Check if we already triggered this beat
        if self.last_absolute_beat == Some(beat_index) {
            return Ok(());
        }

        // A new beat boundary has been crossed
        self.last_absolute_beat = Some(beat_index);

        let beat_in_bar = (beat_index % self.numerator as u64) as u32;
        self.last_triggered_beat = Some(beat_in_bar);

        // Pick the right sample: hi for downbeat, lo for others
        let sound_data = if beat_in_bar == 0 {
            &self.hi_sound
        } else {
            &self.lo_sound
        };

        // Play on the metronome track
        let sound = sound_data
            .output_destination(track)
            .volume(Volume::Amplitude(1.0));

        manager
            .play(sound)
            .map_err(|e| AppError::Audio(format!("Failed to play metronome click: {}", e)))?;

        Ok(())
    }

    /// Returns the current BPM.
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Returns the current beats per bar.
    pub fn numerator(&self) -> u8 {
        self.numerator
    }

    /// Set the track volume for the metronome (convenience for mute).
    pub fn set_volume(track: &mut TrackHandle, volume: f64) {
        track.set_volume(Volume::Amplitude(volume), Tween::default());
    }
}
