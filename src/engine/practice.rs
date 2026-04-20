/// Practice mode state.
///
/// When active, speed_factor starts at 0.5 and ramps up by 0.05
/// after each loop iteration with >= 90% accuracy. Drops by 0.05
/// if accuracy falls below 70%.
pub struct PracticeMode {
    pub active: bool,
    /// Current playback speed factor (0.3 – 1.0).
    pub current_speed: f64,
    /// Target speed factor (usually 1.0 = original BPM).
    pub target_speed: f64,
}

impl PracticeMode {
    pub fn new() -> Self {
        Self {
            active: false,
            current_speed: 0.5,
            target_speed: 1.0,
        }
    }

    /// Evaluate the score for a completed loop iteration and adjust speed.
    ///
    /// - score_pct >= 90 → increment by 0.05, cap at target_speed (max 1.0).
    /// - score_pct < 70  → decrement by 0.05, floor at 0.3.
    /// - 70 <= score_pct < 90 → no change.
    pub fn evaluate_loop(&mut self, score_pct: f64) {
        if !self.active {
            return;
        }

        if score_pct >= 90.0 {
            self.current_speed = (self.current_speed + 0.05).min(self.target_speed).min(1.0);
        } else if score_pct < 70.0 {
            self.current_speed = (self.current_speed - 0.05).max(0.3);
        }
        // 70 <= score_pct < 90: hold speed
    }

    /// Activate practice mode, resetting speed to 0.5.
    pub fn activate(&mut self) {
        self.active = true;
        self.current_speed = 0.5;
    }

    /// Deactivate practice mode.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl Default for PracticeMode {
    fn default() -> Self {
        Self::new()
    }
}
