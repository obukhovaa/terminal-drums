use std::collections::VecDeque;

use crate::midi::types::{Difficulty, DrumNote, DrumPiece};

/// Timing window thresholds in milliseconds.
#[derive(Debug, Clone, Copy)]
pub struct TimingWindows {
    pub perfect_ms: f64,
    pub great_ms: f64,
    pub good_ms: f64,
    pub ok_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingPreset {
    Relaxed,
    Standard,
    Strict,
}

impl TimingPreset {
    pub fn windows(&self) -> TimingWindows {
        match self {
            TimingPreset::Relaxed => TimingWindows {
                perfect_ms: 25.0,
                great_ms: 50.0,
                good_ms: 80.0,
                ok_ms: 120.0,
            },
            TimingPreset::Standard => TimingWindows {
                perfect_ms: 15.0,
                great_ms: 30.0,
                good_ms: 50.0,
                ok_ms: 80.0,
            },
            TimingPreset::Strict => TimingWindows {
                perfect_ms: 8.0,
                great_ms: 15.0,
                good_ms: 25.0,
                ok_ms: 40.0,
            },
        }
    }
}

/// Accuracy classification for a single hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitAccuracy {
    Perfect,
    Great,
    Good,
    Ok,
    Miss,
}

impl HitAccuracy {
    pub fn score_points(&self) -> u32 {
        match self {
            HitAccuracy::Perfect => 100,
            HitAccuracy::Great => 80,
            HitAccuracy::Good => 50,
            HitAccuracy::Ok => 20,
            HitAccuracy::Miss => 0,
        }
    }
}

/// Result of evaluating a single note against user input.
#[derive(Debug, Clone)]
pub enum NoteResult {
    /// User hit the correct drum within the timing window.
    Hit {
        note: DrumNote,
        /// Negative = early, positive = late.
        delta_ms: f64,
        accuracy: HitAccuracy,
    },
    /// User hit a drum at the right time but wrong piece.
    WrongPiece {
        expected: DrumNote,
        actual_piece: DrumPiece,
        delta_ms: f64,
    },
    /// Note passed the hit zone without being hit.
    Miss { note: DrumNote },
    /// User pressed a key when no note was expected.
    Extra { piece: DrumPiece, time_ms: f64 },
}

/// Running score accumulator.
///
/// `total_notes` is incremented for every expected note that is judged (Hit, Miss,
/// or WrongPiece). Extra hits do NOT increment `total_notes` because they have no
/// corresponding expected note. WrongPiece contributes 0 points (same as Miss).
#[derive(Debug, Clone, Default)]
pub struct ScoreAccumulator {
    pub total_notes: u32,
    pub perfect_count: u32,
    pub great_count: u32,
    pub good_count: u32,
    pub ok_count: u32,
    pub miss_count: u32,
    pub wrong_piece_count: u32,
    pub extra_count: u32,
    pub current_combo: u32,
    pub max_combo: u32,
    pub total_points: u32,
}

impl ScoreAccumulator {
    /// Returns score as percentage (0.0 - 100.0).
    pub fn percentage(&self) -> f64 {
        if self.total_notes == 0 {
            return 0.0;
        }
        let max_possible = self.total_notes as f64 * 100.0;
        (self.total_points as f64 / max_possible) * 100.0
    }

    pub fn record_hit(&mut self, accuracy: HitAccuracy) {
        self.total_notes += 1;
        self.total_points += accuracy.score_points();
        match accuracy {
            HitAccuracy::Perfect => self.perfect_count += 1,
            HitAccuracy::Great => self.great_count += 1,
            HitAccuracy::Good => self.good_count += 1,
            HitAccuracy::Ok => self.ok_count += 1,
            HitAccuracy::Miss => unreachable!(),
        }
    }

    pub fn record_miss(&mut self) {
        self.total_notes += 1;
        self.miss_count += 1;
    }

    pub fn record_wrong_piece(&mut self) {
        self.total_notes += 1;
        self.wrong_piece_count += 1;
    }

    pub fn record_extra(&mut self) {
        self.extra_count += 1;
    }

    /// Update combo counter after a hit (any accuracy except Miss).
    pub fn update_combo(&mut self, accuracy: HitAccuracy) {
        match accuracy {
            HitAccuracy::Miss => {
                self.current_combo = 0;
            }
            _ => {
                self.current_combo += 1;
                if self.current_combo > self.max_combo {
                    self.max_combo = self.current_combo;
                }
            }
        }
    }

    /// Break the combo (Miss or WrongPiece).
    pub fn break_combo(&mut self) {
        self.current_combo = 0;
    }
}

/// Persisted score record for leaderboard.
#[derive(Debug, Clone)]
pub struct ScoreRecord {
    pub id: i64,
    pub track_name: String,
    pub difficulty: Difficulty,
    pub timing_preset: TimingPreset,
    pub bpm: f64,
    pub scope: ScoreScope,
    pub score_pct: f64,
    pub perfect: u32,
    pub great: u32,
    pub good: u32,
    pub ok: u32,
    pub miss: u32,
    pub wrong_piece: u32,
    pub max_combo: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreScope {
    Bars8,
    Bars16,
    Bars32,
    FullTrack,
}

/// What happened to a note, from the rolling window's perspective.
/// Distinct from HitAccuracy because we need to track Miss and WrongPiece
/// separately (HitAccuracy::Miss exists but record_hit panics on it).
#[derive(Debug, Clone, Copy)]
pub enum NoteOutcome {
    Hit(HitAccuracy),
    Miss,
    WrongPiece,
}

pub struct RollingScoreWindow {
    pub window_bars: u32,
    pub entries: VecDeque<(u32, NoteOutcome)>,
}

impl RollingScoreWindow {
    pub fn new(window_bars: u32) -> Self {
        Self {
            window_bars,
            entries: VecDeque::new(),
        }
    }

    pub fn add(&mut self, bar: u32, outcome: NoteOutcome) {
        self.entries.push_back((bar, outcome));
    }

    /// Remove entries that have fallen outside the window.
    pub fn prune(&mut self, current_bar: u32) {
        let cutoff = current_bar.saturating_sub(self.window_bars);
        while let Some(&(bar, _)) = self.entries.front() {
            if bar < cutoff {
                self.entries.pop_front();
            } else {
                break;
            }
        }
    }

    /// Recompute summary from the deque. Called when snapshot is taken.
    pub fn summarize(&self) -> ScoreAccumulator {
        let mut acc = ScoreAccumulator::default();
        for &(_, outcome) in &self.entries {
            match outcome {
                NoteOutcome::Hit(accuracy) => acc.record_hit(accuracy),
                NoteOutcome::Miss => acc.record_miss(),
                NoteOutcome::WrongPiece => acc.record_wrong_piece(),
            }
        }
        acc
    }
}

// -----------------------------------------------------------------------------
// ScoringEngine
// -----------------------------------------------------------------------------

/// Classify hit accuracy from an absolute delta in milliseconds.
pub fn classify_accuracy(abs_delta_ms: f64, windows: &TimingWindows) -> HitAccuracy {
    if abs_delta_ms <= windows.perfect_ms {
        HitAccuracy::Perfect
    } else if abs_delta_ms <= windows.great_ms {
        HitAccuracy::Great
    } else if abs_delta_ms <= windows.good_ms {
        HitAccuracy::Good
    } else if abs_delta_ms <= windows.ok_ms {
        HitAccuracy::Ok
    } else {
        HitAccuracy::Miss
    }
}

/// The main scoring engine, holding all accumulators and per-note judged state.
pub struct ScoringEngine {
    pub timing_windows: TimingWindows,
    pub score_full: ScoreAccumulator,
    pub rolling_8bar: RollingScoreWindow,
    pub rolling_16bar: RollingScoreWindow,
    pub rolling_32bar: RollingScoreWindow,
    /// Parallel to the track's `notes` vec — true if the note has been judged.
    judged: Vec<bool>,
}

impl ScoringEngine {
    pub fn new(timing_windows: TimingWindows, note_count: usize) -> Self {
        Self {
            timing_windows,
            score_full: ScoreAccumulator::default(),
            rolling_8bar: RollingScoreWindow::new(8),
            rolling_16bar: RollingScoreWindow::new(16),
            rolling_32bar: RollingScoreWindow::new(32),
            judged: vec![false; note_count],
        }
    }

    /// Reset all scores and judged state (e.g., on replay or loop wrap).
    pub fn reset(&mut self) {
        self.score_full = ScoreAccumulator::default();
        self.rolling_8bar.entries.clear();
        self.rolling_16bar.entries.clear();
        self.rolling_32bar.entries.clear();
        for b in &mut self.judged {
            *b = false;
        }
    }

    /// Reset only the judged state for notes in a loop window [start..end) and
    /// clear the rolling windows. Used on loop wrap.
    /// `include` returns true for notes that should be playable (pass difficulty
    /// filter). Excluded notes stay judged so they won't trigger misses.
    pub fn reset_loop_window<F>(
        &mut self,
        notes: &[DrumNote],
        note_start: usize,
        note_end: usize,
        include: F,
    ) where
        F: Fn(&DrumNote) -> bool,
    {
        for i in note_start..note_end {
            if i < self.judged.len() {
                // Only un-judge notes that pass the difficulty filter
                self.judged[i] = !include(&notes[i]);
            }
        }
        self.rolling_8bar.entries.clear();
        self.rolling_16bar.entries.clear();
        self.rolling_32bar.entries.clear();
    }

    /// Reset scores and pre-judge notes that don't pass the difficulty filter.
    /// Notes that are excluded by difficulty are marked as judged so they won't
    /// be counted as misses or matched as hits.
    pub fn reset_for_difficulty<F>(&mut self, notes: &[DrumNote], include: F)
    where
        F: Fn(&DrumNote) -> bool,
    {
        self.score_full = ScoreAccumulator::default();
        self.rolling_8bar.entries.clear();
        self.rolling_16bar.entries.clear();
        self.rolling_32bar.entries.clear();
        for (i, note) in notes.iter().enumerate() {
            if i < self.judged.len() {
                // Mark excluded notes as judged (skip), include notes as unjudged
                self.judged[i] = !include(note);
            }
        }
    }

    /// Returns hittable notes from the full track notes slice within ±ok_ms.
    pub fn hittable_notes_from<'a>(
        &self,
        position_ms: f64,
        notes: &'a [DrumNote],
    ) -> (&'a [DrumNote], usize) {
        let ok_ms = self.timing_windows.ok_ms;
        let start = position_ms - ok_ms;
        let end = position_ms + ok_ms;
        let start_idx = notes.partition_point(|n| n.time_ms < start);
        let end_idx = notes.partition_point(|n| n.time_ms <= end);
        (&notes[start_idx..end_idx], start_idx)
    }

    /// Returns true if the note at absolute index `idx` has been judged.
    pub fn is_judged(&self, idx: usize) -> bool {
        self.judged.get(idx).copied().unwrap_or(true)
    }

    /// Process a drum hit from the player.
    ///
    /// - `piece`:           Which drum piece was struck.
    /// - `event_time_ms`:   Playback-time of the key event (ms).
    /// - `hittable_notes`:  Slice of notes within the ±ok_ms window (from PlaybackEngine).
    /// - `hittable_offset`: Absolute index of `hittable_notes[0]` in the full track notes vec.
    ///
    /// Returns the `NoteResult` and records it in all accumulators.
    pub fn process_hit(
        &mut self,
        piece: DrumPiece,
        event_time_ms: f64,
        hittable_notes: &[DrumNote],
        hittable_offset: usize,
    ) -> NoteResult {
        let ok_ms = self.timing_windows.ok_ms;

        // Step 1: Find the closest same-piece note AND the closest any-piece note.
        let mut same_piece_match: Option<(usize, f64)> = None; // (local_idx, abs_delta)
        let mut any_piece_match: Option<(usize, f64)> = None;

        for (local_idx, note) in hittable_notes.iter().enumerate() {
            let abs_idx = hittable_offset + local_idx;
            if self.is_judged(abs_idx) {
                continue;
            }
            let delta = event_time_ms - note.time_ms;
            let abs_delta = delta.abs();
            if abs_delta > ok_ms {
                continue;
            }

            // Track closest same-piece match.
            if note.piece == piece {
                match same_piece_match {
                    None => same_piece_match = Some((local_idx, abs_delta)),
                    Some((_, best)) if abs_delta < best => {
                        same_piece_match = Some((local_idx, abs_delta));
                    }
                    _ => {}
                }
            }

            // Track closest any-piece match (fallback for WrongPiece).
            match any_piece_match {
                None => any_piece_match = Some((local_idx, abs_delta)),
                Some((_, best)) if abs_delta < best => {
                    any_piece_match = Some((local_idx, abs_delta));
                }
                _ => {}
            }
        }

        // Step 2: Resolve.
        if let Some((local_idx, _)) = same_piece_match {
            let abs_idx = hittable_offset + local_idx;
            let note = hittable_notes[local_idx].clone();
            let delta_ms = event_time_ms - note.time_ms;
            let accuracy = classify_accuracy(delta_ms.abs(), &self.timing_windows);

            self.judged[abs_idx] = true;
            self.score_full.record_hit(accuracy);
            self.score_full.update_combo(accuracy);
            self.record_rolling(note.bar, NoteOutcome::Hit(accuracy));

            NoteResult::Hit {
                note,
                delta_ms,
                accuracy,
            }
        } else if let Some((local_idx, _)) = any_piece_match {
            let abs_idx = hittable_offset + local_idx;
            let note = hittable_notes[local_idx].clone();
            let delta_ms = event_time_ms - note.time_ms;

            self.judged[abs_idx] = true;
            self.score_full.record_wrong_piece();
            self.score_full.break_combo();
            self.record_rolling(note.bar, NoteOutcome::WrongPiece);

            NoteResult::WrongPiece {
                expected: note,
                actual_piece: piece,
                delta_ms,
            }
        } else {
            // No note in window at all.
            self.score_full.record_extra();
            NoteResult::Extra {
                piece,
                time_ms: event_time_ms,
            }
        }
    }

    /// Check for notes whose window has passed (misses).
    ///
    /// Any unjudged note whose `time_ms` is more than `ok_ms` before `position_ms`
    /// is a miss — but only if `include` returns true for that note (difficulty filter).
    ///
    /// Returns a vec of `NoteResult::Miss` events (may be empty).
    pub fn check_misses<F>(
        &mut self,
        position_ms: f64,
        notes: &[DrumNote],
        include: F,
    ) -> Vec<NoteResult>
    where
        F: Fn(&DrumNote) -> bool,
    {
        let cutoff = position_ms - self.timing_windows.ok_ms;
        let mut results = Vec::new();

        for (idx, note) in notes.iter().enumerate() {
            if note.time_ms >= cutoff {
                break;
            }
            if !self.is_judged(idx) {
                self.judged[idx] = true;
                if include(note) {
                    self.score_full.record_miss();
                    self.score_full.break_combo();
                    self.record_rolling(note.bar, NoteOutcome::Miss);
                    results.push(NoteResult::Miss { note: note.clone() });
                }
                // Excluded notes are silently judged (no miss penalty)
            }
        }

        results
    }

    /// Prune all rolling windows based on the current bar.
    pub fn prune_rolling(&mut self, current_bar: u32) {
        self.rolling_8bar.prune(current_bar);
        self.rolling_16bar.prune(current_bar);
        self.rolling_32bar.prune(current_bar);
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn record_rolling(&mut self, bar: u32, outcome: NoteOutcome) {
        self.rolling_8bar.add(bar, outcome);
        self.rolling_16bar.add(bar, outcome);
        self.rolling_32bar.add(bar, outcome);
    }
}
