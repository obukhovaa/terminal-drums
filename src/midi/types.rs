use std::collections::HashSet;

/// Represents a physical drum piece in a standard kit.
/// Maps to General MIDI drum note numbers on Channel 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrumPiece {
    Kick,
    Snare,
    CrossStick,
    ClosedHiHat,
    OpenHiHat,
    PedalHiHat,
    CrashCymbal1,
    CrashCymbal2,
    RideCymbal,
    RideBell,
    HighTom,
    MidTom,
    LowTom,
    Splash,
    China,
}

/// A single drum note extracted from a MIDI file.
#[derive(Debug, Clone)]
pub struct DrumNote {
    pub piece: DrumPiece,
    pub tick: u64,
    pub time_ms: f64,
    pub velocity: u8,
    pub duration_ms: f64,
    pub bar: u32,
    pub beat: f64,
}

/// Velocity classification for visual rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityLevel {
    /// 1-39 -- rendered as `░`
    Ghost,
    /// 40-69 -- rendered as `▒`
    Soft,
    /// 70-104 -- rendered as `▓`
    Normal,
    /// 105-127 -- rendered as `█`
    Accent,
}

impl From<u8> for VelocityLevel {
    fn from(vel: u8) -> Self {
        match vel {
            0 => VelocityLevel::Ghost,
            1..=39 => VelocityLevel::Ghost,
            40..=69 => VelocityLevel::Soft,
            70..=104 => VelocityLevel::Normal,
            105..=127 => VelocityLevel::Accent,
            _ => VelocityLevel::Accent,
        }
    }
}

/// Tempo event from MIDI (microseconds per quarter note).
#[derive(Debug, Clone)]
pub struct TempoEvent {
    pub tick: u64,
    pub microseconds_per_quarter: u32,
}

/// Time signature event from MIDI.
#[derive(Debug, Clone)]
pub struct TimeSignatureEvent {
    pub tick: u64,
    /// Top number (e.g., 4 in 4/4)
    pub numerator: u8,
    /// Bottom number as actual value (e.g., 4 in 4/4).
    /// Stored as actual denominator, not the MIDI power-of-2 encoding.
    pub denominator: u8,
}

/// A fully parsed and processed drum track ready for playback.
#[derive(Debug)]
pub struct DrumTrack {
    pub name: String,
    /// Sorted by time_ms ascending.
    pub notes: Vec<DrumNote>,
    /// Sorted by tick ascending.
    pub tempo_map: Vec<TempoEvent>,
    /// Sorted by tick ascending.
    pub time_signatures: Vec<TimeSignatureEvent>,
    /// From MIDI header (e.g., 480).
    pub ticks_per_quarter: u16,
    /// Total track duration in milliseconds.
    pub duration_ms: f64,
    /// Total number of bars.
    pub total_bars: u32,
    /// Which drum pieces appear in this track.
    pub pieces_used: HashSet<DrumPiece>,
}

/// Difficulty filtering level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    /// Only downbeat notes, no ghost notes, simplified kick/snare/hh.
    Easy,
    /// All main hits including off-beats, no ghost notes (vel < 40 filtered).
    Medium,
    /// Full MIDI, all notes including ghost notes.
    Hard,
}

impl Difficulty {
    /// Returns true if this note should be visible at this difficulty level.
    pub fn includes_note(&self, note: &DrumNote, is_downbeat: bool) -> bool {
        match self {
            Difficulty::Hard => true,
            Difficulty::Medium => note.velocity >= 40,
            Difficulty::Easy => {
                note.velocity >= 40
                    && is_downbeat
                    && matches!(
                        note.piece,
                        DrumPiece::Kick
                            | DrumPiece::Snare
                            | DrumPiece::ClosedHiHat
                            | DrumPiece::OpenHiHat
                            | DrumPiece::CrashCymbal1
                            | DrumPiece::RideCymbal
                    )
            }
        }
    }
}
