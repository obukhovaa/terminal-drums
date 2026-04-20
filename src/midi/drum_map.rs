use crate::midi::types::DrumPiece;

/// Maps GM MIDI note number to DrumPiece.
/// Returns None for percussion notes outside the standard kit
/// (cowbell, tambourine, bongos, etc.)
pub fn midi_note_to_drum_piece(note: u8) -> Option<DrumPiece> {
    match note {
        35 | 36 => Some(DrumPiece::Kick),
        37 => Some(DrumPiece::CrossStick),
        38 | 40 => Some(DrumPiece::Snare),
        41 | 43 | 45 => Some(DrumPiece::LowTom),
        42 => Some(DrumPiece::ClosedHiHat),
        44 => Some(DrumPiece::PedalHiHat),
        46 => Some(DrumPiece::OpenHiHat),
        47 => Some(DrumPiece::MidTom),
        48 | 50 => Some(DrumPiece::HighTom),
        49 => Some(DrumPiece::CrashCymbal1),
        51 => Some(DrumPiece::RideCymbal),
        52 => Some(DrumPiece::China),
        53 => Some(DrumPiece::RideBell),
        55 => Some(DrumPiece::Splash),
        57 => Some(DrumPiece::CrashCymbal2),
        _ => None,
    }
}
