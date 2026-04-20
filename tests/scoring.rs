use std::collections::HashSet;

use terminal_drums::engine::scoring::{
    classify_accuracy, HitAccuracy, NoteOutcome, NoteResult, RollingScoreWindow,
    ScoreAccumulator, ScoringEngine, TimingPreset, TimingWindows,
};
use terminal_drums::midi::types::{DrumNote, DrumPiece, DrumTrack, TempoEvent, TimeSignatureEvent};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_note(piece: DrumPiece, time_ms: f64) -> DrumNote {
    DrumNote {
        piece,
        tick: 0,
        time_ms,
        velocity: 80,
        duration_ms: 0.0,
        bar: 0,
        beat: 0.0,
    }
}

#[allow(dead_code)]
fn make_note_at_bar(piece: DrumPiece, time_ms: f64, bar: u32) -> DrumNote {
    DrumNote {
        piece,
        tick: 0,
        time_ms,
        velocity: 80,
        duration_ms: 0.0,
        bar,
        beat: 0.0,
    }
}

fn single_note_track(note: DrumNote) -> DrumTrack {
    let mut pieces = HashSet::new();
    pieces.insert(note.piece);
    DrumTrack {
        name: "test".to_string(),
        notes: vec![note],
        tempo_map: vec![TempoEvent {
            tick: 0,
            microseconds_per_quarter: 500_000,
        }],
        time_signatures: vec![TimeSignatureEvent {
            tick: 0,
            numerator: 4,
            denominator: 4,
        }],
        ticks_per_quarter: 480,
        duration_ms: 10_000.0,
        total_bars: 10,
        pieces_used: pieces,
    }
}

fn empty_track() -> DrumTrack {
    DrumTrack {
        name: "empty".to_string(),
        notes: vec![],
        tempo_map: vec![],
        time_signatures: vec![],
        ticks_per_quarter: 480,
        duration_ms: 10_000.0,
        total_bars: 10,
        pieces_used: HashSet::new(),
    }
}

fn standard_windows() -> TimingWindows {
    TimingPreset::Standard.windows()
}

// ---------------------------------------------------------------------------
// classify_accuracy
// ---------------------------------------------------------------------------

#[test]
fn test_classify_perfect() {
    let w = standard_windows(); // perfect = 15 ms
    assert_eq!(classify_accuracy(0.0, &w), HitAccuracy::Perfect);
    assert_eq!(classify_accuracy(15.0, &w), HitAccuracy::Perfect);
}

#[test]
fn test_classify_great() {
    let w = standard_windows(); // great = 30 ms
    assert_eq!(classify_accuracy(16.0, &w), HitAccuracy::Great);
    assert_eq!(classify_accuracy(30.0, &w), HitAccuracy::Great);
}

#[test]
fn test_classify_good() {
    let w = standard_windows(); // good = 50 ms
    assert_eq!(classify_accuracy(31.0, &w), HitAccuracy::Good);
    assert_eq!(classify_accuracy(50.0, &w), HitAccuracy::Good);
}

#[test]
fn test_classify_ok() {
    let w = standard_windows(); // ok = 80 ms
    assert_eq!(classify_accuracy(51.0, &w), HitAccuracy::Ok);
    assert_eq!(classify_accuracy(80.0, &w), HitAccuracy::Ok);
}

#[test]
fn test_classify_miss_beyond_window() {
    let w = standard_windows();
    assert_eq!(classify_accuracy(81.0, &w), HitAccuracy::Miss);
    assert_eq!(classify_accuracy(200.0, &w), HitAccuracy::Miss);
}

// ---------------------------------------------------------------------------
// Timing presets — boundary values
// ---------------------------------------------------------------------------

#[test]
fn test_relaxed_preset_boundaries() {
    let w = TimingPreset::Relaxed.windows();
    // perfect <= 25
    assert_eq!(classify_accuracy(25.0, &w), HitAccuracy::Perfect);
    assert_eq!(classify_accuracy(26.0, &w), HitAccuracy::Great);
    // great <= 50
    assert_eq!(classify_accuracy(50.0, &w), HitAccuracy::Great);
    assert_eq!(classify_accuracy(51.0, &w), HitAccuracy::Good);
    // good <= 80
    assert_eq!(classify_accuracy(80.0, &w), HitAccuracy::Good);
    assert_eq!(classify_accuracy(81.0, &w), HitAccuracy::Ok);
    // ok <= 120
    assert_eq!(classify_accuracy(120.0, &w), HitAccuracy::Ok);
    assert_eq!(classify_accuracy(121.0, &w), HitAccuracy::Miss);
}

#[test]
fn test_strict_preset_boundaries() {
    let w = TimingPreset::Strict.windows();
    // perfect <= 8
    assert_eq!(classify_accuracy(8.0, &w), HitAccuracy::Perfect);
    assert_eq!(classify_accuracy(9.0, &w), HitAccuracy::Great);
    // great <= 15
    assert_eq!(classify_accuracy(15.0, &w), HitAccuracy::Great);
    assert_eq!(classify_accuracy(16.0, &w), HitAccuracy::Good);
    // good <= 25
    assert_eq!(classify_accuracy(25.0, &w), HitAccuracy::Good);
    assert_eq!(classify_accuracy(26.0, &w), HitAccuracy::Ok);
    // ok <= 40
    assert_eq!(classify_accuracy(40.0, &w), HitAccuracy::Ok);
    assert_eq!(classify_accuracy(41.0, &w), HitAccuracy::Miss);
}

#[test]
fn test_standard_preset_boundaries() {
    let w = TimingPreset::Standard.windows();
    assert_eq!(classify_accuracy(15.0, &w), HitAccuracy::Perfect);
    assert_eq!(classify_accuracy(16.0, &w), HitAccuracy::Great);
    assert_eq!(classify_accuracy(30.0, &w), HitAccuracy::Great);
    assert_eq!(classify_accuracy(31.0, &w), HitAccuracy::Good);
    assert_eq!(classify_accuracy(50.0, &w), HitAccuracy::Good);
    assert_eq!(classify_accuracy(51.0, &w), HitAccuracy::Ok);
    assert_eq!(classify_accuracy(80.0, &w), HitAccuracy::Ok);
    assert_eq!(classify_accuracy(81.0, &w), HitAccuracy::Miss);
}

// ---------------------------------------------------------------------------
// ScoringEngine::process_hit — perfect hit
// ---------------------------------------------------------------------------

#[test]
fn test_perfect_hit() {
    let note = make_note(DrumPiece::Snare, 1000.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    let result = engine.process_hit(DrumPiece::Snare, 1000.0, &track.notes, 0);

    match result {
        NoteResult::Hit { accuracy, delta_ms, .. } => {
            assert_eq!(accuracy, HitAccuracy::Perfect);
            assert_eq!(delta_ms, 0.0);
        }
        other => panic!("Expected Hit, got {:?}", other),
    }

    assert_eq!(engine.score_full.total_notes, 1);
    assert_eq!(engine.score_full.perfect_count, 1);
    assert_eq!(engine.score_full.total_points, 100);
    assert_eq!(engine.score_full.current_combo, 1);
    assert_eq!(engine.score_full.max_combo, 1);
}

// ---------------------------------------------------------------------------
// ScoringEngine::process_hit — various offsets
// ---------------------------------------------------------------------------

#[test]
fn test_great_hit() {
    let note = make_note(DrumPiece::Kick, 500.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows(); // great = 30 ms
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    let result = engine.process_hit(DrumPiece::Kick, 520.0, &track.notes, 0);
    match result {
        NoteResult::Hit { accuracy, delta_ms, .. } => {
            assert_eq!(accuracy, HitAccuracy::Great);
            assert!((delta_ms - 20.0).abs() < 0.001);
        }
        other => panic!("Expected Hit, got {:?}", other),
    }
    assert_eq!(engine.score_full.great_count, 1);
}

#[test]
fn test_good_hit() {
    let note = make_note(DrumPiece::Kick, 500.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows(); // good = 50 ms
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    let result = engine.process_hit(DrumPiece::Kick, 540.0, &track.notes, 0);
    match result {
        NoteResult::Hit { accuracy, .. } => assert_eq!(accuracy, HitAccuracy::Good),
        other => panic!("Expected Hit, got {:?}", other),
    }
    assert_eq!(engine.score_full.good_count, 1);
}

#[test]
fn test_ok_hit() {
    let note = make_note(DrumPiece::Kick, 500.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows(); // ok = 80 ms
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    let result = engine.process_hit(DrumPiece::Kick, 570.0, &track.notes, 0);
    match result {
        NoteResult::Hit { accuracy, .. } => assert_eq!(accuracy, HitAccuracy::Ok),
        other => panic!("Expected Hit, got {:?}", other),
    }
    assert_eq!(engine.score_full.ok_count, 1);
}

#[test]
fn test_early_hit() {
    let note = make_note(DrumPiece::Kick, 500.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // 20 ms early → Great
    let result = engine.process_hit(DrumPiece::Kick, 480.0, &track.notes, 0);
    match result {
        NoteResult::Hit { accuracy, delta_ms, .. } => {
            assert_eq!(accuracy, HitAccuracy::Great);
            assert!((delta_ms - (-20.0)).abs() < 0.001, "delta should be negative for early hit");
        }
        other => panic!("Expected Hit, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Miss detection
// ---------------------------------------------------------------------------

#[test]
fn test_miss_detection() {
    let note = make_note(DrumPiece::Snare, 100.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows(); // ok = 80 ms
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // position is well past the note: 100 + 80 + 1 = 181 ms
    let misses = engine.check_misses(181.0, &track.notes);

    assert_eq!(misses.len(), 1, "should detect one miss");
    match &misses[0] {
        NoteResult::Miss { note: n } => assert_eq!(n.piece, DrumPiece::Snare),
        other => panic!("Expected Miss, got {:?}", other),
    }

    assert_eq!(engine.score_full.miss_count, 1);
    assert_eq!(engine.score_full.total_notes, 1);
    assert_eq!(engine.score_full.total_points, 0);
    assert_eq!(engine.score_full.current_combo, 0);
}

#[test]
fn test_no_miss_before_window_passes() {
    let note = make_note(DrumPiece::Snare, 1000.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // position is before the note's ok window ends
    let misses = engine.check_misses(1050.0, &track.notes);
    assert!(misses.is_empty(), "note still in window, should not be a miss");
}

// ---------------------------------------------------------------------------
// Wrong piece detection
// ---------------------------------------------------------------------------

#[test]
fn test_wrong_piece() {
    let note = make_note(DrumPiece::Snare, 500.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // Hit a Kick when a Snare is expected
    let result = engine.process_hit(DrumPiece::Kick, 500.0, &track.notes, 0);
    match result {
        NoteResult::WrongPiece { actual_piece, .. } => {
            assert_eq!(actual_piece, DrumPiece::Kick);
        }
        other => panic!("Expected WrongPiece, got {:?}", other),
    }

    assert_eq!(engine.score_full.wrong_piece_count, 1);
    assert_eq!(engine.score_full.total_notes, 1);
    assert_eq!(engine.score_full.total_points, 0);
    assert_eq!(engine.score_full.current_combo, 0);
}

// ---------------------------------------------------------------------------
// Extra hit
// ---------------------------------------------------------------------------

#[test]
fn test_extra_hit_no_note_nearby() {
    let track = empty_track();
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, 0);

    let result = engine.process_hit(DrumPiece::Kick, 5000.0, &track.notes, 0);
    match result {
        NoteResult::Extra { piece, time_ms } => {
            assert_eq!(piece, DrumPiece::Kick);
            assert_eq!(time_ms, 5000.0);
        }
        other => panic!("Expected Extra, got {:?}", other),
    }

    assert_eq!(engine.score_full.extra_count, 1);
    assert_eq!(engine.score_full.total_notes, 0, "extras don't count as notes");
}

#[test]
fn test_extra_does_not_break_combo() {
    let note = make_note(DrumPiece::Snare, 1000.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // Hit the correct note first (build combo to 1)
    engine.process_hit(DrumPiece::Snare, 1000.0, &track.notes, 0);
    assert_eq!(engine.score_full.current_combo, 1);

    // Extra hit on empty space — should NOT break combo
    let empty = empty_track();
    engine.process_hit(DrumPiece::Kick, 2000.0, &empty.notes, 0);
    assert_eq!(engine.score_full.current_combo, 1, "extra should not break combo");
}

// ---------------------------------------------------------------------------
// Combo building and breaking
// ---------------------------------------------------------------------------

#[test]
fn test_combo_builds_on_hits() {
    let mut acc = ScoreAccumulator::default();
    acc.record_hit(HitAccuracy::Perfect);
    acc.update_combo(HitAccuracy::Perfect);
    assert_eq!(acc.current_combo, 1);
    assert_eq!(acc.max_combo, 1);

    acc.record_hit(HitAccuracy::Great);
    acc.update_combo(HitAccuracy::Great);
    assert_eq!(acc.current_combo, 2);
    assert_eq!(acc.max_combo, 2);

    acc.record_hit(HitAccuracy::Ok);
    acc.update_combo(HitAccuracy::Ok);
    assert_eq!(acc.current_combo, 3);
    assert_eq!(acc.max_combo, 3);
}

#[test]
fn test_combo_breaks_on_miss() {
    let mut acc = ScoreAccumulator::default();
    acc.record_hit(HitAccuracy::Perfect);
    acc.update_combo(HitAccuracy::Perfect);
    acc.record_hit(HitAccuracy::Perfect);
    acc.update_combo(HitAccuracy::Perfect);
    assert_eq!(acc.current_combo, 2);
    assert_eq!(acc.max_combo, 2);

    acc.record_miss();
    acc.break_combo();
    assert_eq!(acc.current_combo, 0);
    assert_eq!(acc.max_combo, 2, "max_combo should be preserved after break");
}

#[test]
fn test_combo_breaks_on_wrong_piece() {
    let mut acc = ScoreAccumulator::default();
    acc.record_hit(HitAccuracy::Perfect);
    acc.update_combo(HitAccuracy::Perfect);
    assert_eq!(acc.current_combo, 1);

    acc.record_wrong_piece();
    acc.break_combo();
    assert_eq!(acc.current_combo, 0);
    assert_eq!(acc.max_combo, 1);
}

#[test]
fn test_combo_via_scoring_engine() {
    // Build a track with multiple notes and verify combo tracking end-to-end.
    let notes: Vec<DrumNote> = (0..5)
        .map(|i| make_note(DrumPiece::Snare, (i as f64) * 500.0))
        .collect();
    let mut pieces = HashSet::new();
    pieces.insert(DrumPiece::Snare);
    let track = DrumTrack {
        name: "multi".to_string(),
        notes: notes.clone(),
        tempo_map: vec![],
        time_signatures: vec![],
        ticks_per_quarter: 480,
        duration_ms: 3000.0,
        total_bars: 4,
        pieces_used: pieces,
    };

    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // Hit notes 0, 1, 2 perfectly.
    for i in 0..3usize {
        let t = notes[i].time_ms;
        engine.process_hit(DrumPiece::Snare, t, &track.notes[i..=i], i);
    }
    assert_eq!(engine.score_full.current_combo, 3);
    assert_eq!(engine.score_full.max_combo, 3);

    // Miss note 3 (we don't hit it; advance position past it and call check_misses).
    let miss_cutoff = notes[3].time_ms + windows.ok_ms + 1.0;
    engine.check_misses(miss_cutoff, &track.notes);
    assert_eq!(engine.score_full.current_combo, 0, "combo should break on miss");
    assert_eq!(engine.score_full.max_combo, 3, "max_combo preserved");

    // Hit note 4 — combo restarts from 1.
    let i = 4;
    let t = notes[i].time_ms;
    engine.process_hit(DrumPiece::Snare, t, &track.notes[i..=i], i);
    assert_eq!(engine.score_full.current_combo, 1);
    assert_eq!(engine.score_full.max_combo, 3);
}

// ---------------------------------------------------------------------------
// Score percentage
// ---------------------------------------------------------------------------

#[test]
fn test_score_percentage_all_perfect() {
    let mut acc = ScoreAccumulator::default();
    for _ in 0..10 {
        acc.record_hit(HitAccuracy::Perfect);
    }
    assert!((acc.percentage() - 100.0).abs() < 0.001);
}

#[test]
fn test_score_percentage_all_miss() {
    let mut acc = ScoreAccumulator::default();
    for _ in 0..10 {
        acc.record_miss();
    }
    assert!((acc.percentage() - 0.0).abs() < 0.001);
}

#[test]
fn test_score_percentage_empty() {
    let acc = ScoreAccumulator::default();
    assert!((acc.percentage() - 100.0).abs() < 0.001, "empty track = 100%");
}

#[test]
fn test_score_percentage_mixed() {
    let mut acc = ScoreAccumulator::default();
    // 5 perfect (100 pts each) + 5 miss (0 pts) = 500/1000 = 50%
    for _ in 0..5 {
        acc.record_hit(HitAccuracy::Perfect);
    }
    for _ in 0..5 {
        acc.record_miss();
    }
    assert!((acc.percentage() - 50.0).abs() < 0.001);
}

#[test]
fn test_score_percentage_great_only() {
    let mut acc = ScoreAccumulator::default();
    // 10 great (80 pts each) = 800/1000 = 80%
    for _ in 0..10 {
        acc.record_hit(HitAccuracy::Great);
    }
    assert!((acc.percentage() - 80.0).abs() < 0.001);
}

// ---------------------------------------------------------------------------
// Rolling window
// ---------------------------------------------------------------------------

#[test]
fn test_rolling_window_prune() {
    let mut win = RollingScoreWindow::new(8);
    // Add entries for bars 0..7
    for bar in 0u32..8 {
        win.add(bar, NoteOutcome::Hit(HitAccuracy::Perfect));
    }
    assert_eq!(win.entries.len(), 8);

    // Advance to bar 10 — cutoff = 10 - 8 = 2, so bars 0 and 1 should be pruned.
    win.prune(10);
    assert_eq!(win.entries.len(), 6);
    assert_eq!(win.entries.front().unwrap().0, 2);
}

#[test]
fn test_rolling_window_summarize() {
    let mut win = RollingScoreWindow::new(16);
    win.add(0, NoteOutcome::Hit(HitAccuracy::Perfect));
    win.add(1, NoteOutcome::Hit(HitAccuracy::Great));
    win.add(2, NoteOutcome::Miss);
    win.add(3, NoteOutcome::WrongPiece);

    let acc = win.summarize();
    assert_eq!(acc.total_notes, 4);
    assert_eq!(acc.perfect_count, 1);
    assert_eq!(acc.great_count, 1);
    assert_eq!(acc.miss_count, 1);
    assert_eq!(acc.wrong_piece_count, 1);
    // 100 + 80 = 180 points out of 400 max = 45%
    assert!((acc.percentage() - 45.0).abs() < 0.001);
}

// ---------------------------------------------------------------------------
// Already-judged notes are not double-counted
// ---------------------------------------------------------------------------

#[test]
fn test_no_double_judgment() {
    let note = make_note(DrumPiece::Snare, 500.0);
    let track = single_note_track(note.clone());
    let windows = standard_windows();
    let mut engine = ScoringEngine::new(windows, track.notes.len());

    // First hit — should register
    engine.process_hit(DrumPiece::Snare, 500.0, &track.notes, 0);
    assert_eq!(engine.score_full.total_notes, 1);

    // Second hit on same note — should be treated as Extra
    let result = engine.process_hit(DrumPiece::Snare, 500.0, &track.notes, 0);
    match result {
        NoteResult::Extra { .. } => {}
        other => panic!("Expected Extra on double hit, got {:?}", other),
    }
    // total_notes still 1 (extras don't increment it)
    assert_eq!(engine.score_full.total_notes, 1);
    assert_eq!(engine.score_full.extra_count, 1);
}
