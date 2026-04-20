use std::collections::HashMap;

use crossterm::event::KeyCode;

use crate::midi::types::DrumPiece;

/// Maps keyboard keys to drum pieces.
pub struct KeyMap {
    pub map: HashMap<KeyCode, DrumPiece>,
}

impl KeyMap {
    /// Create the split-hand preset (default).
    pub fn split_preset() -> Self {
        let mut map = HashMap::new();
        map.insert(KeyCode::Char(' '), DrumPiece::Kick);
        map.insert(KeyCode::Char('a'), DrumPiece::Snare);
        map.insert(KeyCode::Char('s'), DrumPiece::CrossStick);
        map.insert(KeyCode::Char('j'), DrumPiece::ClosedHiHat);
        map.insert(KeyCode::Char('k'), DrumPiece::OpenHiHat);
        map.insert(KeyCode::Char('d'), DrumPiece::PedalHiHat);
        map.insert(KeyCode::Char('l'), DrumPiece::RideCymbal);
        map.insert(KeyCode::Char('u'), DrumPiece::CrashCymbal1);
        map.insert(KeyCode::Char('i'), DrumPiece::CrashCymbal2);
        map.insert(KeyCode::Char('e'), DrumPiece::HighTom);
        map.insert(KeyCode::Char('w'), DrumPiece::MidTom);
        map.insert(KeyCode::Char('q'), DrumPiece::LowTom);
        map.insert(KeyCode::Char('o'), DrumPiece::Splash);
        map.insert(KeyCode::Char('p'), DrumPiece::China);
        Self { map }
    }

    /// Create the compact preset.
    pub fn compact_preset() -> Self {
        let mut map = HashMap::new();
        map.insert(KeyCode::Char(' '), DrumPiece::Kick);
        map.insert(KeyCode::Char('f'), DrumPiece::Snare);
        map.insert(KeyCode::Char('d'), DrumPiece::CrossStick);
        map.insert(KeyCode::Char('j'), DrumPiece::ClosedHiHat);
        map.insert(KeyCode::Char('k'), DrumPiece::OpenHiHat);
        map.insert(KeyCode::Char('h'), DrumPiece::PedalHiHat);
        map.insert(KeyCode::Char('l'), DrumPiece::RideCymbal);
        map.insert(KeyCode::Char('g'), DrumPiece::CrashCymbal1);
        map.insert(KeyCode::Char(';'), DrumPiece::CrashCymbal2);
        map.insert(KeyCode::Char('r'), DrumPiece::HighTom);
        map.insert(KeyCode::Char('e'), DrumPiece::MidTom);
        map.insert(KeyCode::Char('w'), DrumPiece::LowTom);
        map.insert(KeyCode::Char('u'), DrumPiece::Splash);
        map.insert(KeyCode::Char('i'), DrumPiece::China);
        Self { map }
    }

    /// Look up a key code to find the mapped drum piece.
    pub fn get(&self, key: &KeyCode) -> Option<DrumPiece> {
        self.map.get(key).copied()
    }
}
