/// Integration tests for MIDI parsing pipeline.
///
/// These tests parse the pre-generated fixture `tests/fixtures/basic_4_4.mid`
/// and verify note count, piece types, tempo extraction, and bar/beat calculation.
///
/// Run `cargo test --test midi_test_gen generate_fixtures` first if fixtures
/// are missing (they are committed, so this should not be necessary in CI).

use std::collections::HashSet;
use std::path::PathBuf;

use terminal_drums::engine::timing::{compute_bar_beat, ticks_to_ms};
use terminal_drums::midi::parser::parse_midi_file;
use terminal_drums::midi::types::{DrumPiece, TempoEvent, TimeSignatureEvent};

fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

// --------------------------------------------------------------------------
// Parser integration tests
// --------------------------------------------------------------------------

#[test]
fn test_parse_basic_4_4_exists() {
    let path = fixture("basic_4_4.mid");
    assert!(
        path.exists(),
        "fixture basic_4_4.mid is missing — run `cargo test --test midi_test_gen generate_fixtures` to regenerate"
    );
}

#[test]
fn test_parse_basic_4_4_note_count() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // The basic_4_4 pattern has:
    //   32 8th notes → 32 hi-hat (GM 42)
    //    8 kick notes  (beats 1 and 3 of 4 bars = 2 kicks/bar × 4 bars)
    //    8 snare notes (beats 2 and 4 of 4 bars = 2 snares/bar × 4 bars)
    // Total = 48
    assert_eq!(
        track.notes.len(),
        48,
        "expected 48 notes, got {}",
        track.notes.len()
    );
}

#[test]
fn test_parse_basic_4_4_piece_types() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    let expected_pieces: HashSet<DrumPiece> = [
        DrumPiece::ClosedHiHat,
        DrumPiece::Kick,
        DrumPiece::Snare,
    ]
    .into_iter()
    .collect();

    assert_eq!(
        track.pieces_used, expected_pieces,
        "pieces_used mismatch: got {:?}",
        track.pieces_used
    );
}

#[test]
fn test_parse_basic_4_4_tempo() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    assert!(!track.tempo_map.is_empty(), "tempo_map should not be empty");
    assert_eq!(
        track.tempo_map[0].microseconds_per_quarter, 500_000,
        "expected 120 BPM (500,000 µs/beat)"
    );
}

#[test]
fn test_parse_basic_4_4_time_signature() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    assert!(
        !track.time_signatures.is_empty(),
        "time_signatures should not be empty"
    );
    let ts = &track.time_signatures[0];
    assert_eq!(ts.numerator, 4, "expected numerator=4");
    assert_eq!(ts.denominator, 4, "expected denominator=4");
}

#[test]
fn test_parse_basic_4_4_tpq() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");
    assert_eq!(track.ticks_per_quarter, 480);
}

#[test]
fn test_parse_basic_4_4_notes_sorted() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    for window in track.notes.windows(2) {
        assert!(
            window[0].time_ms <= window[1].time_ms,
            "notes are not sorted: {:.3} > {:.3}",
            window[0].time_ms,
            window[1].time_ms
        );
    }
}

#[test]
fn test_parse_basic_4_4_bar_beat_first_note() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // The first note is at tick 0 — bar=0, beat=0.0
    let first = &track.notes[0];
    assert_eq!(first.bar, 0, "first note should be in bar 0");
    assert!(
        first.beat.abs() < 0.001,
        "first note beat should be 0.0, got {}",
        first.beat
    );
}

#[test]
fn test_parse_basic_4_4_second_bar() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // Notes on bar 1 should exist (bar index is 0-based)
    let bar1_notes: Vec<_> = track.notes.iter().filter(|n| n.bar == 1).collect();
    assert!(
        !bar1_notes.is_empty(),
        "should have notes in bar 1 (second bar)"
    );
}

#[test]
fn test_parse_basic_4_4_duration_ms() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // 4 bars at 120 BPM: the last 8th-note hi-hat fires at tick 7440 (31st 8th note, 0-indexed).
    // tick 7440 at 120 BPM = (7440 / 480) * 500 = 7750ms
    // With a 10-tick note duration = ~7760ms
    // We allow a window around that value.
    assert!(
        track.duration_ms >= 7700.0 && track.duration_ms <= 8000.0,
        "expected duration between 7700ms and 8000ms, got {:.1}",
        track.duration_ms
    );
    // Must be positive
    assert!(track.duration_ms > 0.0);
}

#[test]
fn test_parse_basic_4_4_total_bars() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // 4 bars → total_bars should be 4
    assert_eq!(track.total_bars, 4, "expected total_bars=4, got {}", track.total_bars);
}

#[test]
fn test_parse_basic_4_4_kick_on_beats() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // All kick notes should be on beat 0 (beats 1 or 3 in music speak → beat 0.0 or 2.0)
    let kick_beats: Vec<f64> = track
        .notes
        .iter()
        .filter(|n| n.piece == DrumPiece::Kick)
        .map(|n| n.beat)
        .collect();

    assert_eq!(kick_beats.len(), 8, "expected 8 kick notes");
    for &beat in &kick_beats {
        // kick should be on beat 0 or beat 2 (0-indexed)
        assert!(
            (beat - 0.0).abs() < 0.01 || (beat - 2.0).abs() < 0.01,
            "kick note at unexpected beat position: {}",
            beat
        );
    }
}

#[test]
fn test_parse_basic_4_4_snare_on_beats() {
    let path = fixture("basic_4_4.mid");
    let track = parse_midi_file(&path).expect("parse_midi_file failed");

    // All snare notes should be on beat 1 or beat 3 (0-indexed)
    let snare_beats: Vec<f64> = track
        .notes
        .iter()
        .filter(|n| n.piece == DrumPiece::Snare)
        .map(|n| n.beat)
        .collect();

    assert_eq!(snare_beats.len(), 8, "expected 8 snare notes");
    for &beat in &snare_beats {
        assert!(
            (beat - 1.0).abs() < 0.01 || (beat - 3.0).abs() < 0.01,
            "snare note at unexpected beat position: {}",
            beat
        );
    }
}

// --------------------------------------------------------------------------
// Unit tests for timing helpers
// --------------------------------------------------------------------------

#[test]
fn test_ticks_to_ms_120bpm() {
    // 120 BPM = 500,000 µs/quarter, tpq=480
    // 1 quarter = 480 ticks = 500ms
    let tempo_map = vec![TempoEvent {
        tick: 0,
        microseconds_per_quarter: 500_000,
    }];
    let ms = ticks_to_ms(480, &tempo_map, 480);
    assert!((ms - 500.0).abs() < 0.001, "expected 500ms got {ms}");
}

#[test]
fn test_ticks_to_ms_default_tempo() {
    // No tempo map → default 500,000 µs/quarter
    let ms = ticks_to_ms(480, &[], 480);
    assert!((ms - 500.0).abs() < 0.001, "expected 500ms got {ms}");
}

#[test]
fn test_ticks_to_ms_tempo_change() {
    // 120 BPM for first 480 ticks (500ms), then 60 BPM (1000,000 µs) for next 480 ticks (1000ms)
    let tempo_map = vec![
        TempoEvent { tick: 0, microseconds_per_quarter: 500_000 },
        TempoEvent { tick: 480, microseconds_per_quarter: 1_000_000 },
    ];
    let ms = ticks_to_ms(960, &tempo_map, 480);
    assert!((ms - 1500.0).abs() < 0.001, "expected 1500ms got {ms}");
}

#[test]
fn test_compute_bar_beat_downbeat() {
    let (bar, beat) = compute_bar_beat(0, &[], 480);
    assert_eq!(bar, 0);
    assert!((beat - 0.0).abs() < 0.001);
}

#[test]
fn test_compute_bar_beat_second_bar() {
    // 4/4, tpq=480: ticks_per_bar=1920 → bar 1 starts at tick 1920
    let (bar, beat) = compute_bar_beat(1920, &[], 480);
    assert_eq!(bar, 1);
    assert!((beat - 0.0).abs() < 0.001);
}

#[test]
fn test_compute_bar_beat_beat_2() {
    // tick=480 = beat 1 within bar 0 (0-indexed beats)
    let (bar, beat) = compute_bar_beat(480, &[], 480);
    assert_eq!(bar, 0);
    assert!((beat - 1.0).abs() < 0.001, "expected beat=1.0, got {beat}");
}

#[test]
fn test_compute_bar_beat_with_time_sig() {
    // 3/4 time: ticks_per_bar = 480*4*3/4 = 1440
    let time_sigs = vec![TimeSignatureEvent {
        tick: 0,
        numerator: 3,
        denominator: 4,
    }];
    // tick=1440 → bar 1, beat 0
    let (bar, beat) = compute_bar_beat(1440, &time_sigs, 480);
    assert_eq!(bar, 1);
    assert!((beat - 0.0).abs() < 0.001);
}

#[test]
fn test_compute_bar_beat_hihat_8th_note() {
    // 4/4 tpq=480: 8th note = 240 ticks → beat=0.5 within bar 0
    let (bar, beat) = compute_bar_beat(240, &[], 480);
    assert_eq!(bar, 0);
    assert!((beat - 0.5).abs() < 0.001, "expected beat=0.5, got {beat}");
}
