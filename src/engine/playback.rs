use std::time::Instant;

use crate::engine::timing::ticks_to_ms;
use crate::midi::types::{DrumNote, DrumTrack};

/// The playback engine maintains position in milliseconds, advanced by the game loop.
pub struct PlaybackEngine {
    pub position_ms: f64,
    /// 1.0 = normal speed, 0.5 = half speed, etc.
    pub speed_factor: f64,
    pub playing: bool,
    pub start_instant: Instant,
    /// Accumulated time (in ms) before the last pause / loop wrap.
    pub pause_offset_ms: f64,

    // Loop state
    pub loop_active: bool,
    pub loop_start_ms: f64,
    pub loop_end_ms: f64,

    // Precomputed note windows
    pub track: DrumTrack,
    /// Index of the next upcoming note (for miss detection / efficiency).
    pub note_index: usize,

    /// BPM extracted from the track's first tempo event.
    pub original_bpm: f64,

    /// Look-ahead window for visible notes (default 3000.0 ms).
    pub look_ahead_ms: f64,
}

impl PlaybackEngine {
    pub fn new(track: DrumTrack) -> Self {
        // Extract BPM from the first tempo event (or default to 120).
        let original_bpm = if let Some(first_tempo) = track.tempo_map.first() {
            60_000_000.0 / first_tempo.microseconds_per_quarter as f64
        } else {
            120.0
        };

        Self {
            position_ms: 0.0,
            speed_factor: 1.0,
            playing: false,
            start_instant: Instant::now(),
            pause_offset_ms: 0.0,
            loop_active: false,
            loop_start_ms: 0.0,
            loop_end_ms: 0.0,
            note_index: 0,
            original_bpm,
            look_ahead_ms: 3000.0,
            track,
        }
    }

    /// Called at 120 Hz by the game loop. Advances playback position.
    pub fn tick(&mut self) {
        if !self.playing {
            return;
        }

        let elapsed = self.start_instant.elapsed().as_secs_f64() * 1000.0;
        self.position_ms = self.pause_offset_ms + (elapsed * self.speed_factor);

        if self.loop_active && self.position_ms >= self.loop_end_ms {
            // Wrap: compute overshoot and re-anchor at loop_start_ms.
            let overshoot = self.position_ms - self.loop_end_ms;
            self.position_ms = self.loop_start_ms + overshoot;
            self.pause_offset_ms = self.loop_start_ms;
            self.start_instant = Instant::now();
            // Reset note_index to the first note at or after loop_start_ms.
            self.note_index = self.note_index_for(self.loop_start_ms);
            return;
        }

        if self.position_ms >= self.track.duration_ms {
            self.playing = false;
        }

        // Advance note_index past notes that are now behind position.
        // (Callers that do hit detection use hittable_notes; we just keep
        //  note_index roughly in sync so the binary search is fast.)
        while self.note_index < self.track.notes.len()
            && self.track.notes[self.note_index].time_ms < self.position_ms
        {
            self.note_index += 1;
        }
    }

    /// Start or resume playback. No-op if already playing.
    pub fn play(&mut self) {
        if self.playing {
            return;
        }
        self.playing = true;
        self.start_instant = Instant::now();
    }

    /// Pause playback. Accumulates elapsed time into pause_offset_ms.
    pub fn pause(&mut self) {
        if !self.playing {
            return;
        }
        let elapsed = self.start_instant.elapsed().as_secs_f64() * 1000.0;
        self.pause_offset_ms += elapsed * self.speed_factor;
        self.playing = false;
    }

    /// Reset everything and start playing from the beginning.
    pub fn replay(&mut self) {
        self.position_ms = 0.0;
        self.pause_offset_ms = 0.0;
        self.note_index = 0;
        self.playing = true;
        self.start_instant = Instant::now();
    }

    /// Set a loop region defined by bar number and length.
    ///
    /// Computes ms boundaries from the track's bar positions using tick-based
    /// lookup: we walk the notes to find the tick at each bar boundary, then
    /// convert via ticks_to_ms.
    pub fn set_loop(&mut self, start_bar: u32, num_bars: u32) {
        let end_bar = start_bar + num_bars;
        self.loop_start_ms = self.ms_for_bar(start_bar);
        self.loop_end_ms = self.ms_for_bar(end_bar);
        self.loop_active = true;
    }

    /// Override playback BPM. Recomputes speed_factor relative to original_bpm.
    pub fn set_bpm(&mut self, bpm: f64) {
        if self.original_bpm <= 0.0 {
            return;
        }
        self.speed_factor = bpm / self.original_bpm;
    }

    // -------------------------------------------------------------------------
    // Helper methods
    // -------------------------------------------------------------------------

    /// Returns the slice of notes that fall within the look-ahead window
    /// [position_ms, position_ms + look_ahead_ms].
    pub fn visible_notes(&self, look_ahead_ms: f64) -> &[DrumNote] {
        let start = self.position_ms;
        let end = self.position_ms + look_ahead_ms;
        let start_idx = self
            .track
            .notes
            .partition_point(|n| n.time_ms < start);
        let end_idx = self
            .track
            .notes
            .partition_point(|n| n.time_ms <= end);
        &self.track.notes[start_idx..end_idx]
    }

    /// Returns the slice of notes within ±ok_ms of position_ms.
    /// Also returns the absolute start index so callers can map back to
    /// the full notes vec for judging.
    pub fn hittable_notes(&self, ok_ms: f64) -> (&[DrumNote], usize) {
        let start = self.position_ms - ok_ms;
        let end = self.position_ms + ok_ms;
        let start_idx = self
            .track
            .notes
            .partition_point(|n| n.time_ms < start);
        let end_idx = self
            .track
            .notes
            .partition_point(|n| n.time_ms <= end);
        (&self.track.notes[start_idx..end_idx], start_idx)
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Binary-search for the first note at or after `position_ms` and return
    /// its index in the full notes vec.
    fn note_index_for(&self, position_ms: f64) -> usize {
        self.track
            .notes
            .partition_point(|n| n.time_ms < position_ms)
    }

    /// Convert a bar number to a millisecond position.
    ///
    /// Strategy: scan the notes vec for the first note whose `bar` field equals
    /// `target_bar` and return its `time_ms`. If no note exists for that bar
    /// (e.g., bar is past the end) we extrapolate from the track's tempo/time-sig
    /// using tick arithmetic.
    fn ms_for_bar(&self, target_bar: u32) -> f64 {
        // Fast path: find first note in that bar.
        for note in &self.track.notes {
            if note.bar >= target_bar {
                if note.bar == target_bar {
                    // The bar starts at the note's time minus its beat offset.
                    // For notes on beat 0 this is exact; for others we subtract
                    // the beat offset in ms. We'll use a tick-based approach for
                    // accuracy.
                    return self.bar_start_ms_via_tick(target_bar, note);
                }
                // We've passed target_bar with no note in it — bar is empty.
                // Use the previous note's bar to extrapolate.
                break;
            }
        }

        // Fallback: extrapolate using the last known time signature and tempo.
        self.extrapolate_bar_ms(target_bar)
    }

    /// Given a note that lives in `target_bar`, compute the ms of the start of
    /// that bar by converting the note's tick minus its beat-offset ticks.
    fn bar_start_ms_via_tick(&self, target_bar: u32, note: &DrumNote) -> f64 {
        // Find ticks_per_bar under the active time signature at this note's tick.
        let tpq = self.track.ticks_per_quarter as u64;
        let (num, denom) = self.time_sig_at(note.tick);
        let ticks_per_bar = tpq * 4 * num as u64 / denom as u64;
        let ticks_per_beat = tpq * 4 / denom as u64;

        // The note's tick_in_bar = beat * ticks_per_beat
        let tick_in_bar = (note.beat * ticks_per_beat as f64).round() as u64;
        let bar_start_tick = note.tick.saturating_sub(tick_in_bar);

        // Sanity-check: compute which bar bar_start_tick falls in.
        // If this note is the first in target_bar, bar_start_tick should be
        // ticks_per_bar * target_bar (approximately) relative to the time sig start.
        // We trust the parser's bar field and use bar_start_tick directly.
        let _ = (target_bar, ticks_per_bar); // used implicitly via bar field

        ticks_to_ms(
            bar_start_tick,
            &self.track.tempo_map,
            self.track.ticks_per_quarter,
        )
    }

    /// Extrapolate the ms position for a bar that has no notes, using the last
    /// time signature and tempo event.
    fn extrapolate_bar_ms(&self, target_bar: u32) -> f64 {
        if self.track.notes.is_empty() {
            return 0.0;
        }

        // Find the last note to anchor our extrapolation.
        let last_note = self.track.notes.last().unwrap();
        let last_bar = last_note.bar;
        if target_bar <= last_bar {
            // Should have found it in the fast path — return duration as fallback.
            return self.track.duration_ms;
        }

        let tpq = self.track.ticks_per_quarter as u64;
        let (num, denom) = self.time_sig_at(last_note.tick);
        let ticks_per_bar = tpq * 4 * num as u64 / denom as u64;

        // Approximate: tick of last note's bar start + (target_bar - last_bar) bars.
        let tick_in_bar = (last_note.beat * (tpq * 4 / denom as u64) as f64).round() as u64;
        let last_bar_start_tick = last_note.tick.saturating_sub(tick_in_bar);
        let extra_bars = target_bar.saturating_sub(last_bar) as u64;
        let target_tick = last_bar_start_tick + extra_bars * ticks_per_bar;

        ticks_to_ms(
            target_tick,
            &self.track.tempo_map,
            self.track.ticks_per_quarter,
        )
    }

    /// Return the (numerator, denominator) of the time signature active at `tick`.
    fn time_sig_at(&self, tick: u64) -> (u8, u8) {
        let mut num = 4u8;
        let mut denom = 4u8;
        for ts in &self.track.time_signatures {
            if ts.tick > tick {
                break;
            }
            num = ts.numerator;
            denom = ts.denominator;
        }
        (num, denom)
    }
}
