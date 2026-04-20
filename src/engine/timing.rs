use crate::midi::types::{TempoEvent, TimeSignatureEvent};

/// Convert a MIDI tick position to milliseconds using the tempo map.
///
/// Implements the algorithm from spec §4.3:
/// - Walk tempo events in order, accumulating ms for each section between changes.
/// - Default tempo: 500,000 µs/quarter (120 BPM).
pub fn ticks_to_ms(tick: u64, tempo_map: &[TempoEvent], tpq: u16) -> f64 {
    let mut accumulated_ms: f64 = 0.0;
    let mut current_tick: u64 = 0;
    let mut current_tempo: u32 = 500_000; // Default: 120 BPM

    for tempo_event in tempo_map {
        if tempo_event.tick > tick {
            break;
        }
        let delta_ticks = tempo_event.tick - current_tick;
        accumulated_ms +=
            (delta_ticks as f64 / tpq as f64) * (current_tempo as f64 / 1000.0);
        current_tick = tempo_event.tick;
        current_tempo = tempo_event.microseconds_per_quarter;
    }

    // Remaining ticks after the last tempo change
    let delta_ticks = tick - current_tick;
    accumulated_ms += (delta_ticks as f64 / tpq as f64) * (current_tempo as f64 / 1000.0);

    accumulated_ms
}

/// Compute the bar and beat position for a given tick.
/// Returns `(bar_number, beat_within_bar)` where bar is 0-indexed.
///
/// Implements the algorithm from spec §4.5:
/// - Walk time signature events, counting complete bars in each section.
/// - Default time signature: 4/4.
pub fn compute_bar_beat(
    tick: u64,
    time_sigs: &[TimeSignatureEvent],
    tpq: u16,
) -> (u32, f64) {
    let mut current_tick: u64 = 0;
    let mut current_bar: u32 = 0;
    let mut current_num: u8 = 4;
    let mut current_denom: u8 = 4;

    for ts_event in time_sigs {
        if ts_event.tick > tick {
            break;
        }
        // Count complete bars between current_tick and ts_event.tick
        let ticks_per_bar =
            tpq as u64 * 4 * current_num as u64 / current_denom as u64;
        let delta_ticks = ts_event.tick - current_tick;
        let bars_in_section = delta_ticks / ticks_per_bar;
        current_bar += bars_in_section as u32;
        // Advance current_tick to the start of the new time signature
        // (skip any partial bar at the boundary — spec says we only advance by
        // whole bars; the time sig change is expected to land on a bar line)
        current_tick = ts_event.tick;
        current_num = ts_event.numerator;
        current_denom = ts_event.denominator;
    }

    // Remaining ticks after the last time signature change
    let ticks_per_bar = tpq as u64 * 4 * current_num as u64 / current_denom as u64;
    let ticks_per_beat = tpq as u64 * 4 / current_denom as u64;
    let delta_ticks = tick - current_tick;
    let bars_remaining = delta_ticks / ticks_per_bar;
    let tick_in_bar = delta_ticks % ticks_per_bar;
    let beat = tick_in_bar as f64 / ticks_per_beat as f64;

    (current_bar + bars_remaining as u32, beat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticks_to_ms_no_tempo_map() {
        // With no tempo events, default tempo (120 BPM = 500,000 µs/quarter) is used.
        // tpq=480, 480 ticks = 1 quarter = 500ms
        let ms = ticks_to_ms(480, &[], 480);
        assert!((ms - 500.0).abs() < 0.001, "expected 500ms, got {ms}");
    }

    #[test]
    fn test_ticks_to_ms_constant_tempo() {
        // 120 BPM, tpq=480, 4 bars of 4/4 = 16 beats = 8000ms
        let tempo_map = vec![TempoEvent {
            tick: 0,
            microseconds_per_quarter: 500_000,
        }];
        let ms = ticks_to_ms(480 * 16, &tempo_map, 480);
        assert!((ms - 8000.0).abs() < 0.001, "expected 8000ms, got {ms}");
    }

    #[test]
    fn test_ticks_to_ms_tempo_change() {
        // First 480 ticks at 500,000 µs/q = 500ms
        // Then next 480 ticks at 1,000,000 µs/q (60 BPM) = 1000ms
        let tempo_map = vec![
            TempoEvent { tick: 0, microseconds_per_quarter: 500_000 },
            TempoEvent { tick: 480, microseconds_per_quarter: 1_000_000 },
        ];
        let ms = ticks_to_ms(960, &tempo_map, 480);
        assert!((ms - 1500.0).abs() < 0.001, "expected 1500ms, got {ms}");
    }

    #[test]
    fn test_compute_bar_beat_4_4() {
        // tpq=480, 4/4: ticks_per_bar=1920
        // tick=1920 → bar=1, beat=0.0
        let (bar, beat) = compute_bar_beat(1920, &[], 480);
        assert_eq!(bar, 1);
        assert!((beat - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_bar_beat_downbeat() {
        // tick=0 → bar=0, beat=0.0
        let (bar, beat) = compute_bar_beat(0, &[], 480);
        assert_eq!(bar, 0);
        assert!((beat - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_bar_beat_mid_bar() {
        // tick=480 (beat 1 in a 4/4 bar), tpq=480
        // ticks_per_beat=480, so beat=1.0
        let (bar, beat) = compute_bar_beat(480, &[], 480);
        assert_eq!(bar, 0);
        assert!((beat - 1.0).abs() < 0.001);
    }
}
