# Terminal Drums - Technical Specification

**Version:** 1.0-draft
**Status:** Pre-development
**Based on:** PLAN.md (all open questions resolved)

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Architecture](#2-architecture)
3. [Data Types & Structures](#3-data-types--structures)
4. [MIDI Engine](#4-midi-engine)
5. [Playback Engine](#5-playback-engine)
6. [Scoring Engine](#6-scoring-engine)
7. [Audio Engine](#7-audio-engine)
8. [Input System](#8-input-system)
9. [Command System](#9-command-system)
10. [UI & Rendering](#10-ui--rendering)
11. [Theme System](#11-theme-system)
12. [Data Persistence](#12-data-persistence)
13. [File Formats](#13-file-formats)
14. [Configuration](#14-configuration)
15. [Application Lifecycle](#15-application-lifecycle)
16. [Error Handling](#16-error-handling)
17. [Performance Requirements](#17-performance-requirements)
18. [Testing Specification](#18-testing-specification)
19. [Project Structure](#19-project-structure)
20. [Dependency Manifest](#20-dependency-manifest)
21. [Future: Network Architecture](#21-future-network-architecture)

---

## 1. System Overview

### 1.1 Purpose

Terminal Drums is a TUI rhythm training application. It parses MIDI drum tracks, visualizes them as a top-to-bottom scrolling note highway, accepts vim-style key input, and scores the user's timing accuracy. It runs inside terminal emulators (iTerm2, Kitty, Ghostty, Alacritty) and multiplexers (tmux).

### 1.2 Target Environment

- **OS:** macOS (primary), Linux (secondary)
- **Terminal:** Any terminal supporting 256-color or truecolor ANSI escape codes
- **Multiplexer:** tmux compatible (no Kitty keyboard protocol dependency)
- **Minimum terminal size:** 80 columns x 24 rows
- **Recommended terminal size:** 120 columns x 40 rows

### 1.3 Binary Name

```
tdrums
```

### 1.4 CLI Interface

```
tdrums                          # Launch app (loads last session or welcome screen)
tdrums <path/to/track.mid>      # Launch with specific MIDI file
tdrums --track <name>           # Launch with named track from library
tdrums --bpm <number>           # Override BPM
tdrums --kit <name>             # Override kit
tdrums --theme <name>           # Override theme
tdrums --visual-only            # Launch in visual-only mode (no audio)
tdrums --config <path>          # Custom config file path
tdrums --version                # Print version
tdrums --help                   # Print help
```

---

## 2. Architecture

### 2.1 Threading Model

Four logical threads communicate via channels and shared state:

```
┌─────────────┐     crossbeam channel      ┌─────────────────┐
│ Input Thread │ ──── TimestampedEvent ────→ │   Game Thread    │
│ (dedicated)  │                             │ (main loop @120) │
└─────────────┘                             └────────┬────────┘
                                                     │
                                          SwapBuffer<GameState>
                                            (lock-free atomic)
                                                     │
                                            ┌────────┴────────┐
                                            │  Render Thread   │
                                            │  (ratatui @60)   │
                                            └─────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                     Audio Thread (kira)                       │
│  Managed internally by kira::AudioManager                    │
│  Receives commands via kira's command queue                   │
└──────────────────────────────────────────────────────────────┘
```

**Input Thread:**
- Polls for events via `crossterm::event::poll()` with 10ms timeout, then `read()`
- Timestamps each event with `std::time::Instant::now()`
- Sends `TimestampedEvent` over `crossbeam::channel::unbounded()`
- Checks `AtomicBool` shutdown flag between polls for clean exit

**Game Thread (main):**
- Runs at 120 ticks/second via `spin_sleep::LoopHelper`
- Drains input channel each tick
- Advances playback position
- Performs hit detection
- Writes `GameState` snapshot into the inactive slot of a lock-free `SwapBuffer`
- Atomically swaps the read index so the render thread sees the new snapshot
- Sends audio commands to kira

**Render Thread:**
- Runs at 60 FPS via `spin_sleep::LoopHelper`
- Reads the current `GameState` snapshot from `SwapBuffer` (lock-free atomic read)
- Calls `terminal.draw()` with all widgets
- Never mutates game state, never contends with the game thread

**Audio Thread:**
- Managed by `kira::AudioManager`
- Receives play/stop/seek commands via kira's API
- Clock-driven scheduling for metronome
- Low-latency sample triggering for drum hits

### 2.2 State Machine

The application operates as a state machine:

```
                    ┌──────────┐
            ┌──────│ Welcome   │ (first run only)
            │      └──────────┘
            │            │ name entered
            │            v
            │      ┌──────────┐
            └─────→│ TrackSel  │←──── /cassette
                   └──────────┘
                         │ track selected
                         v
                   ┌──────────┐
              ┌───→│  Ready    │←──── /pause (from Playing)
              │    └──────────┘
              │          │ /play
              │          v
              │    ┌──────────┐         ┌─────────────┐
              │    │ Playing   │──/cal──→│ Calibrating  │
              │    └──────────┘         │ (esc=back)   │
              │          │              └──────┬───────┘
              │          │ track ends      done│
              │          v                     │ returns to
              │    ┌──────────┐                │ previous state
              └────│Scoreboard│←──── /scoreboard
                   └──────────┘
```

Note: `/calibrate` can be invoked from any Session state (Ready, Playing, or Paused).
The pre-calibration state is saved and restored on completion or cancellation (Esc).

Top-level states:
```rust
enum AppState {
    Welcome,                    // First-run name entry
    TrackSelect,                // Cassette browser
    Session(SessionState),      // Active session
    Scoreboard,                 // Viewing scores
    Calibrating,                // Input latency calibration (see §8.4)
    Quitting,                   // Cleanup and exit
}

/// Session sub-state. Maps 1:1 to PlaybackEngine's running/paused state.
/// There is no separate PlaybackState enum; SessionState IS the playback state.
enum SessionState {
    Ready,      // Track loaded, not yet started (position = 0)
    Playing,    // Actively playing (PlaybackEngine advancing)
    Paused,     // Paused mid-track (PlaybackEngine frozen)
}
```

---

## 3. Data Types & Structures

### 3.1 Drum Pieces

```rust
/// Represents a physical drum piece in a standard kit.
/// Maps to General MIDI drum note numbers on Channel 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DrumPiece {
    Kick,           // GM notes: 35, 36
    Snare,          // GM notes: 38, 40
    CrossStick,     // GM note:  37
    ClosedHiHat,    // GM note:  42
    OpenHiHat,      // GM note:  46
    PedalHiHat,     // GM note:  44
    CrashCymbal1,   // GM note:  49
    CrashCymbal2,   // GM note:  57
    RideCymbal,     // GM note:  51
    RideBell,       // GM note:  53
    HighTom,        // GM notes: 48, 50
    MidTom,         // GM note:  47
    LowTom,         // GM notes: 41, 43, 45
    Splash,         // GM note:  55
    China,          // GM note:  52
}
```

### 3.2 MIDI Note Mapping

```rust
/// Maps GM MIDI note number to DrumPiece.
/// Returns None for percussion notes outside the standard kit
/// (cowbell, tambourine, bongos, etc.)
fn midi_note_to_drum_piece(note: u8) -> Option<DrumPiece> {
    match note {
        35 | 36 => Some(DrumPiece::Kick),
        37      => Some(DrumPiece::CrossStick),
        38 | 40 => Some(DrumPiece::Snare),
        41 | 43 | 45 => Some(DrumPiece::LowTom),
        42      => Some(DrumPiece::ClosedHiHat),
        44      => Some(DrumPiece::PedalHiHat),
        46      => Some(DrumPiece::OpenHiHat),
        47      => Some(DrumPiece::MidTom),
        48 | 50 => Some(DrumPiece::HighTom),
        49      => Some(DrumPiece::CrashCymbal1),
        51      => Some(DrumPiece::RideCymbal),
        52      => Some(DrumPiece::China),
        53      => Some(DrumPiece::RideBell),
        55      => Some(DrumPiece::Splash),
        57      => Some(DrumPiece::CrashCymbal2),
        _       => None,
    }
}
```

### 3.3 Parsed Track Data

```rust
/// A single drum note extracted from a MIDI file.
#[derive(Debug, Clone)]
struct DrumNote {
    piece: DrumPiece,
    tick: u64,              // Absolute tick position in the MIDI file
    time_ms: f64,           // Absolute time in milliseconds (computed from tempo map)
    velocity: u8,           // 1-127 (parser MUST discard NoteOn with vel=0; these are note-offs)
    duration_ms: f64,       // Duration in ms (NoteOn to NoteOff). 0 for instantaneous hits.
    bar: u32,               // Which bar this note falls in (0-indexed)
    beat: f64,              // Beat position within the bar (0.0 = downbeat)
}

/// Velocity classification for visual rendering.
#[derive(Debug, Clone, Copy)]
enum VelocityLevel {
    Ghost,      // 1-39    → rendered as ░
    Soft,       // 40-69   → rendered as ▒
    Normal,     // 70-104  → rendered as ▓
    Accent,     // 105-127 → rendered as █
}

impl From<u8> for VelocityLevel {
    fn from(vel: u8) -> Self {
        match vel {
            0        => VelocityLevel::Ghost, // Should never appear; parser filters vel=0
            1..=39   => VelocityLevel::Ghost,
            40..=69  => VelocityLevel::Soft,
            70..=104 => VelocityLevel::Normal,
            105..=127 => VelocityLevel::Accent,
            // u8 max is 255 but MIDI velocity is 0-127
            _ => VelocityLevel::Accent,
        }
    }
}

/// Tempo event from MIDI (microseconds per quarter note).
#[derive(Debug, Clone)]
struct TempoEvent {
    tick: u64,
    microseconds_per_quarter: u32,
}

/// Time signature event from MIDI.
#[derive(Debug, Clone)]
struct TimeSignatureEvent {
    tick: u64,
    numerator: u8,          // Top number (e.g., 4 in 4/4)
    denominator: u8,        // Bottom number as actual value (e.g., 4 in 4/4)
    // Stored as actual denominator, not the MIDI power-of-2 encoding
}

/// A fully parsed and processed drum track ready for playback.
#[derive(Debug)]
struct DrumTrack {
    name: String,
    notes: Vec<DrumNote>,           // Sorted by time_ms ascending
    tempo_map: Vec<TempoEvent>,     // Sorted by tick ascending
    time_signatures: Vec<TimeSignatureEvent>,  // Sorted by tick ascending
    ticks_per_quarter: u16,         // From MIDI header (e.g., 480)
    duration_ms: f64,               // Total track duration
    total_bars: u32,                // Total number of bars
    pieces_used: HashSet<DrumPiece>, // Which drum pieces appear in this track
}
```

### 3.4 Difficulty Filtering

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Easy,       // Only downbeat notes, no ghost notes, simplified kick/snare/hh
    Medium,     // All main hits including off-beats, no ghost notes (vel < 40 filtered)
    Hard,       // Full MIDI, all notes including ghost notes
}

impl Difficulty {
    /// Returns true if this note should be visible at this difficulty level.
    fn includes_note(&self, note: &DrumNote, is_downbeat: bool) -> bool {
        match self {
            Difficulty::Hard => true,
            Difficulty::Medium => note.velocity >= 40,
            Difficulty::Easy => {
                note.velocity >= 40
                    && is_downbeat
                    && matches!(note.piece,
                        DrumPiece::Kick | DrumPiece::Snare | DrumPiece::ClosedHiHat
                        | DrumPiece::OpenHiHat | DrumPiece::CrashCymbal1 | DrumPiece::RideCymbal
                    )
            }
        }
    }
}
```

### 3.5 Scoring Types

```rust
/// Timing window thresholds in milliseconds.
#[derive(Debug, Clone, Copy)]
struct TimingWindows {
    perfect_ms: f64,
    great_ms: f64,
    good_ms: f64,
    ok_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimingPreset {
    Relaxed,    // 25 / 50 / 80 / 120 ms
    Standard,   // 15 / 30 / 50 / 80 ms
    Strict,     // 8  / 15 / 25 / 40 ms
}

impl TimingPreset {
    fn windows(&self) -> TimingWindows {
        match self {
            TimingPreset::Relaxed  => TimingWindows { perfect_ms: 25.0, great_ms: 50.0, good_ms: 80.0, ok_ms: 120.0 },
            TimingPreset::Standard => TimingWindows { perfect_ms: 15.0, great_ms: 30.0, good_ms: 50.0, ok_ms: 80.0 },
            TimingPreset::Strict   => TimingWindows { perfect_ms: 8.0,  great_ms: 15.0, good_ms: 25.0, ok_ms: 40.0 },
        }
    }
}

/// Accuracy classification for a single hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitAccuracy {
    Perfect,
    Great,
    Good,
    Ok,
    Miss,
}

impl HitAccuracy {
    fn score_points(&self) -> u32 {
        match self {
            HitAccuracy::Perfect => 100,
            HitAccuracy::Great   => 80,
            HitAccuracy::Good    => 50,
            HitAccuracy::Ok      => 20,
            HitAccuracy::Miss    => 0,
        }
    }
}

/// Result of evaluating a single note against user input.
#[derive(Debug, Clone)]
enum NoteResult {
    /// User hit the correct drum within the timing window.
    Hit {
        note: DrumNote,
        delta_ms: f64,          // Negative = early, positive = late
        accuracy: HitAccuracy,
    },
    /// User hit a drum at the right time but wrong piece.
    WrongPiece {
        expected: DrumNote,
        actual_piece: DrumPiece,
        delta_ms: f64,
    },
    /// Note passed the hit zone without being hit.
    Miss {
        note: DrumNote,
    },
    /// User pressed a key when no note was expected.
    Extra {
        piece: DrumPiece,
        time_ms: f64,
    },
}

/// Running score accumulator.
///
/// `total_notes` is incremented for every expected note that is judged (Hit, Miss,
/// or WrongPiece). Extra hits do NOT increment `total_notes` because they have no
/// corresponding expected note. WrongPiece contributes 0 points (same as Miss).
#[derive(Debug, Clone, Default)]
struct ScoreAccumulator {
    total_notes: u32,       // Hit + Miss + WrongPiece (NOT extras)
    perfect_count: u32,
    great_count: u32,
    good_count: u32,
    ok_count: u32,
    miss_count: u32,
    wrong_piece_count: u32, // Increments total_notes, contributes 0 points
    extra_count: u32,       // Does NOT increment total_notes
    current_combo: u32,
    max_combo: u32,
    total_points: u32,
}

impl ScoreAccumulator {
    /// Returns score as percentage (0.0 - 100.0).
    fn percentage(&self) -> f64 {
        if self.total_notes == 0 {
            return 100.0;
        }
        let max_possible = self.total_notes as f64 * 100.0;
        (self.total_points as f64 / max_possible) * 100.0
    }

    fn record_hit(&mut self, accuracy: HitAccuracy) {
        self.total_notes += 1;
        self.total_points += accuracy.score_points();
        match accuracy {
            HitAccuracy::Perfect => self.perfect_count += 1,
            HitAccuracy::Great   => self.great_count += 1,
            HitAccuracy::Good    => self.good_count += 1,
            HitAccuracy::Ok      => self.ok_count += 1,
            HitAccuracy::Miss    => unreachable!(),
        }
    }

    fn record_miss(&mut self) {
        self.total_notes += 1;
        self.miss_count += 1;
        // 0 points
    }

    fn record_wrong_piece(&mut self) {
        self.total_notes += 1;
        self.wrong_piece_count += 1;
        // 0 points (counts against score like a miss)
    }

    fn record_extra(&mut self) {
        self.extra_count += 1;
        // Does NOT affect total_notes or total_points
    }
}

/// Persisted score record for leaderboard.
#[derive(Debug, Clone)]
struct ScoreRecord {
    id: i64,                    // SQLite rowid
    track_name: String,
    difficulty: Difficulty,
    timing_preset: TimingPreset,
    bpm: f64,                   // Actual BPM used (may differ from track default)
    scope: ScoreScope,
    score_pct: f64,
    perfect: u32,
    great: u32,
    good: u32,
    ok: u32,
    miss: u32,
    wrong_piece: u32,           // Matches wrong_piece_count in SQLite schema
    max_combo: u32,
    timestamp: i64,             // Unix timestamp
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScoreScope {
    Bars8,
    Bars16,
    Bars32,
    FullTrack,
}
```

### 3.6 Game State

```rust
/// Render-ready snapshot of game state. Produced by the game thread each tick,
/// consumed by the render thread. This is a SNAPSHOT — the render thread never
/// mutates it. The game thread owns the canonical state in PlaybackEngine,
/// ScoringEngine, etc. and writes snapshots into a swap buffer.
///
/// State ownership:
///   PlaybackEngine  → owns position_ms, speed_factor, loop state, note_index
///   ScoringEngine   → owns score accumulators, recent results
///   GameState       → read-only snapshot for rendering
struct GameState {
    // Session
    app_state: AppState,
    track_name: Option<String>,
    difficulty: Difficulty,
    timing_preset: TimingPreset,
    pieces_used: HashSet<DrumPiece>,    // Which lanes to render

    // Playback (snapshot from PlaybackEngine — NOT duplicated ownership)
    position_ms: f64,           // Current playback position
    effective_bpm: f64,         // Active BPM after override/practice scaling
    time_sig: (u8, u8),         // Active time signature (num, denom)
    current_bar: u32,
    current_beat: f64,
    total_bars: u32,
    track_duration_ms: f64,

    // Loop (snapshot from PlaybackEngine)
    loop_active: bool,
    loop_start_bar: u32,
    loop_end_bar: u32,

    // Practice mode (snapshot from PlaybackEngine)
    practice_mode: bool,
    practice_bpm: f64,          // Current practice BPM (ramping up)
    practice_target_bpm: f64,   // Target BPM to reach

    // Scoring (snapshot from ScoringEngine)
    score_full: ScoreAccumulator,
    score_8bar: ScoreAccumulator,
    score_16bar: ScoreAccumulator,
    score_32bar: ScoreAccumulator,

    // Visual feedback (recent events for rendering, last 2 seconds)
    recent_results: VecDeque<(f64, NoteResult)>,
    metronome_phase: f64,       // 0.0-1.0 for metronome animation

    // Visible notes for highway rendering (pre-sliced by PlaybackEngine)
    visible_notes: Vec<DrumNote>,   // Notes within look-ahead window

    // Audio state
    audio_mode: AudioMode,
    mute_metronome: bool,
    mute_backtrack: bool,
    mute_kit: bool,
    mute_all: bool,
    metronome_volume: f64,      // 0.0-1.0
    kit_volume: f64,
    backtrack_volume: f64,

    // Input mode
    input_mode: InputMode,
    console_input: String,      // Current console text
    console_cursor: usize,
    autocomplete_suggestions: Vec<String>,
    autocomplete_selected: Option<usize>,

    // UI
    theme: ThemeName,
    terminal_size: (u16, u16),  // (cols, rows)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioMode {
    VisualOnly,
    VisualAudio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Insert,
}
```

**State sharing mechanism:** The game thread writes a new `GameState` snapshot into
a double buffer (two `GameState` slots behind an `AtomicBool` swap flag). The render
thread reads the most recently published snapshot. No `Mutex` — the swap is lock-free.
Only the game thread writes; only the render thread reads. The `AtomicBool` indicates
which slot is "live."

```rust
struct SwapBuffer {
    slots: [UnsafeCell<GameState>; 2],
    /// Index of the slot the render thread should read (0 or 1).
    read_idx: AtomicUsize,
}

// Game thread (writer):
//   1. Write new snapshot into slots[1 - read_idx.load(Relaxed)]
//   2. read_idx.store(new_idx, Release)  ← Release ensures the write
//      to the slot is visible before the index update.
//
// Render thread (reader):
//   1. let idx = read_idx.load(Acquire)  ← Acquire ensures it sees
//      the completed slot write that preceded the Release store.
//   2. Read from slots[idx].
//
// The game thread writes at 120Hz; the render thread reads at 60Hz.
// The game thread may overwrite the inactive slot multiple times between
// render reads. This is intentional — the render thread always sees the
// *latest* snapshot, not every intermediate state. No frames are queued.
```

---

## 4. MIDI Engine

### 4.1 Parsing Pipeline

```
MIDI File (bytes)
    │
    │  midly::Smf::parse()
    v
Raw MIDI Tracks + Header
    │
    │  Step 1: Extract tempo map
    │  Step 2: Extract time signatures
    │  Step 3: Detect drum channel
    │  Step 4: Extract drum notes
    │  Step 5: Compute absolute times
    │  Step 6: Compute bar/beat positions
    │  Step 7: Apply difficulty filter
    v
DrumTrack (ready for playback)
```

### 4.2 Channel Detection Algorithm

```
fn detect_drum_channel(smf: &Smf) -> u8:
    1. Count notes per channel across all tracks
    2. If channel 9 (GM drum, 0-indexed) has notes → return 9
    3. Otherwise, for each channel with notes:
       - Count how many notes fall in GM drum range (35-81)
       - Pick channel with highest drum-range note count
    4. If no channel found → error: "No drum data found in MIDI file"
```

Override: If `meta.toml` specifies `midi_channel`, skip detection and use that value (convert 1-indexed user input to 0-indexed internal).

### 4.3 Tick-to-Time Conversion

MIDI timing is tick-based. Conversion to real-time requires the tempo map:

```
fn ticks_to_ms(tick: u64, tempo_map: &[TempoEvent], tpq: u16) -> f64:
    accumulated_ms = 0.0
    current_tick = 0
    current_tempo = 500_000  // Default: 120 BPM (500,000 µs/quarter)

    for each tempo_event in tempo_map:
        if tempo_event.tick > tick:
            break
        delta_ticks = tempo_event.tick - current_tick
        accumulated_ms += (delta_ticks as f64 / tpq as f64) * (current_tempo as f64 / 1000.0)
        current_tick = tempo_event.tick
        current_tempo = tempo_event.microseconds_per_quarter

    // Remaining ticks after last tempo change
    delta_ticks = tick - current_tick
    accumulated_ms += (delta_ticks as f64 / tpq as f64) * (current_tempo as f64 / 1000.0)

    return accumulated_ms
```

### 4.4 BPM Override

When user overrides BPM via `/bpm` or CLI flag:
1. Compute the ratio: `speed_factor = override_bpm / original_bpm`
2. Scale all `time_ms` values: `note.time_ms /= speed_factor`
3. Scale all `duration_ms` values: `note.duration_ms /= speed_factor`
4. Recalculate `duration_ms` of the entire track

The MIDI tick positions and bar/beat assignments remain unchanged - only real-time positions scale.

### 4.5 Bar/Beat Calculation

```
fn compute_bar_beat(tick: u64, time_sigs: &[TimeSignatureEvent], tpq: u16) -> (u32, f64):
    current_tick = 0
    current_bar = 0
    current_num = 4
    current_denom = 4

    for each ts_event in time_sigs:
        if ts_event.tick > tick:
            break
        // Count complete bars between current_tick and ts_event.tick
        ticks_per_bar = tpq as u64 * 4 * current_num as u64 / current_denom as u64
        delta_ticks = ts_event.tick - current_tick
        bars_in_section = delta_ticks / ticks_per_bar
        current_bar += bars_in_section as u32
        current_tick = ts_event.tick
        current_num = ts_event.numerator
        current_denom = ts_event.denominator

    // Remaining ticks
    ticks_per_bar = tpq as u64 * 4 * current_num as u64 / current_denom as u64
    ticks_per_beat = tpq as u64 * 4 / current_denom as u64
    delta_ticks = tick - current_tick
    bars_remaining = delta_ticks / ticks_per_bar
    tick_in_bar = delta_ticks % ticks_per_bar
    beat = tick_in_bar as f64 / ticks_per_beat as f64

    return (current_bar + bars_remaining as u32, beat)
```

---

## 5. Playback Engine

### 5.1 Position Tracking

The playback engine maintains a position in milliseconds, advanced by the game loop:

```rust
struct PlaybackEngine {
    position_ms: f64,
    speed_factor: f64,      // 1.0 = normal, 0.5 = half speed, etc.
    playing: bool,
    start_instant: Instant,
    pause_offset_ms: f64,   // Accumulated time before last pause

    // Loop state
    loop_active: bool,
    loop_start_ms: f64,
    loop_end_ms: f64,

    // Precomputed note windows
    track: DrumTrack,
    note_index: usize,      // Index of next upcoming note
}
```

Each game tick:
```
if playing:
    elapsed = start_instant.elapsed().as_secs_f64() * 1000.0
    position_ms = pause_offset_ms + (elapsed * speed_factor)

    if loop_active && position_ms >= loop_end_ms:
        // Before wrapping, finalize any notes within ok_ms of loop_end_ms.
        // Notes in the "grace zone" (loop_end_ms - ok_ms .. loop_end_ms) that
        // have not been judged are marked as Miss. This prevents them from
        // being abandoned mid-judgment at the wrap point.
        for note in unjudged_notes_in_range(loop_end_ms - timing_windows.ok_ms, loop_end_ms):
            emit NoteResult::Miss { note }
            mark note as judged

        position_ms = loop_start_ms + (position_ms - loop_end_ms)
        reset scoring for loop window
        reset note_index to loop start
        // Reset play_start_instant and pause_offset_ms to keep input time mapping valid.
        // pause_offset_ms is in "playback time" space (same unit as position_ms).
        // speed_factor is applied only to elapsed wall-clock time, not to the offset.
        pause_offset_ms = loop_start_ms
        start_instant = Instant::now()

    if position_ms >= track.duration_ms:
        playing = false
        trigger scoreboard display
```

### 5.2 Note Window

The render thread needs to know which notes are "visible" on the highway. The game thread maintains:

```rust
/// Look-ahead window: notes between (position_ms) and (position_ms + look_ahead_ms).
/// Default look_ahead_ms = 3000.0 (3 seconds of upcoming notes visible).
fn visible_notes(&self) -> &[DrumNote] {
    let start = self.position_ms;
    let end = self.position_ms + self.look_ahead_ms;
    // Binary search for start/end indices in the sorted notes vec
    &self.track.notes[start_idx..end_idx]
}

/// Notes in the "hit zone" window: within ok_ms of current position.
/// These are the notes the player can currently hit.
fn hittable_notes(&self) -> &[DrumNote] {
    let window = self.timing_windows.ok_ms;
    let start = self.position_ms - window;
    let end = self.position_ms + window;
    &self.track.notes[start_idx..end_idx]
}
```

### 5.3 Commands

| Command | Action |
|---------|--------|
| `/play` | If Stopped/Paused: set `playing = true`, record `start_instant = Instant::now()`. If already Playing: no-op. |
| `/pause` | If Playing: set `playing = false`, accumulate elapsed time into `pause_offset_ms`. If already Paused: no-op. |
| `/replay` | Reset `position_ms = 0`, `pause_offset_ms = 0`, `note_index = 0`, reset all scores, set `playing = true`. |
| `/loop N` | Calculate start/end ms for next N bars from current position. Set `loop_active = true`. |
| `/bpm N` | Recompute `speed_factor = N / original_bpm`. Scale all note times. Reset position. |

### 5.4 Practice Mode

When `/practice` is active:
1. `speed_factor` starts at 0.5 (50% BPM)
2. After each loop iteration, if `score_pct >= 90.0`:
   - Increment `speed_factor` by 0.05 (5% of original BPM)
   - Cap at 1.0 (100% BPM)
   - Flash "BPM UP!" feedback on screen
3. If score drops below 70% on a loop, decrease by 0.05 (floor at 0.3)
4. Practice mode requires an active loop (`/loop N` must be set)

---

## 6. Scoring Engine

### 6.1 Hit Detection Algorithm

Run every game tick (120Hz):

```
for each key_event in input_queue:
    piece = key_map.get(key_event.key)
    if piece is None: continue  // Unmapped key
    event_time_ms = key_event_to_playback_time(key_event.instant)

    // Step 1: Try to find the closest SAME-PIECE note within the ok_ms window.
    // This is the primary match — prevents false WrongPiece when multiple
    // instruments have notes near each other (e.g. kick+snare+hihat on the beat).
    same_piece_match: Option<(usize, f64)> = None  // (note_index, abs_delta)
    any_piece_match: Option<(usize, f64)> = None

    for (idx, note) in hittable_notes():
        if note.already_judged: continue
        delta = event_time_ms - note.time_ms
        abs_delta = delta.abs()
        if abs_delta <= timing_windows.ok_ms:
            // Track closest same-piece match
            if note.piece == piece:
                if same_piece_match.is_none() || abs_delta < same_piece_match.1:
                    same_piece_match = Some((idx, abs_delta))
            // Track closest any-piece match (fallback for WrongPiece detection)
            if any_piece_match.is_none() || abs_delta < any_piece_match.1:
                any_piece_match = Some((idx, abs_delta))

    // Step 2: Resolve the match.
    // Prefer same-piece match. Only fall back to any-piece (WrongPiece) if
    // there is NO same-piece note in the window at all.
    if let Some((idx, _)) = same_piece_match:
        note = &track.notes[idx]
        delta = event_time_ms - note.time_ms
        accuracy = classify_accuracy(delta.abs(), timing_windows)
        emit NoteResult::Hit { note, delta, accuracy }
        update_combo(accuracy)
        mark note as judged
    else if let Some((idx, _)) = any_piece_match:
        // There IS an expected note nearby, but the player hit the wrong drum.
        note = &track.notes[idx]
        delta = event_time_ms - note.time_ms
        emit NoteResult::WrongPiece { expected: note, actual: piece, delta }
        break_combo()
        mark note as judged
    else:
        emit NoteResult::Extra { piece, event_time_ms }
        // No combo break for extras

// Check for misses: notes whose time has passed beyond the ok_ms window
for note in unhit_notes_before(position_ms - timing_windows.ok_ms):
    emit NoteResult::Miss { note }
    break_combo()
    mark note as judged
```

### 6.2 Input Time Mapping

Key events are timestamped with `Instant`. To compare against MIDI note times:

```rust
fn key_event_to_playback_time(&self, event_instant: Instant) -> Option<f64> {
    // Use checked_duration_since to handle edge case where a key event
    // was queued before the most recent play/resume reset play_start_instant.
    // This can happen when a key event sits in the channel during a pause→play
    // transition. Stale events (timestamped before play_start_instant) are
    // discarded by returning None.
    let elapsed_since_start = event_instant.checked_duration_since(self.play_start_instant)?;
    let raw_ms = elapsed_since_start.as_secs_f64() * 1000.0;
    Some(self.pause_offset_ms + (raw_ms * self.speed_factor) + self.calibration_offset_ms)
}
```

The caller skips the event if `None` is returned (stale input from before resume).

**Grace period for first hit after /play:** When `/play` is executed, it resets
`play_start_instant = Instant::now()`. If a drum hit key event was pressed in the
same game tick (its `Instant` is microseconds before the new `play_start_instant`),
`checked_duration_since` returns `None` and the hit would be silently discarded.

To handle this: when processing `/play`, the game thread must process the command
**before** draining drum hit events from the input queue in that same tick. This
ensures `play_start_instant` is set before any hit timestamps are evaluated. As an
additional safeguard, if `checked_duration_since` returns `None` and the absolute
difference is less than 50ms (i.e., the event is very close to `play_start_instant`),
treat elapsed as 0ms rather than discarding:

```rust
let elapsed_since_start = match event_instant.checked_duration_since(self.play_start_instant) {
    Some(d) => d,
    None => {
        // Event is slightly before play_start_instant — grace period
        let reverse = self.play_start_instant.duration_since(event_instant);
        if reverse.as_millis() < 50 {
            Duration::ZERO  // Treat as "exactly at play start"
        } else {
            return None;  // Genuinely stale event
        }
    }
};
```

The `calibration_offset_ms` compensates for systematic input latency (set by `/calibrate`).

### 6.3 Rolling Window Scores

Maintain three rolling score windows for the last 8, 16, and 32 bars.

**Data structure:** Rolling windows use a **different structure** than the full-track
`ScoreAccumulator`. They maintain a deque of individual results so entries can be
removed when they fall outside the window. The summary counts are recomputed from
the deque (not decremented — avoids drift from rounding or bugs).

```rust
/// What happened to a note, from the rolling window's perspective.
/// Distinct from HitAccuracy because we need to track Miss and WrongPiece
/// separately (HitAccuracy::Miss exists but record_hit panics on it).
#[derive(Debug, Clone, Copy)]
enum NoteOutcome {
    Hit(HitAccuracy),   // Perfect, Great, Good, or Ok — never Miss
    Miss,
    WrongPiece,
}

struct RollingScoreWindow {
    window_bars: u32,                           // 8, 16, or 32
    entries: VecDeque<(u32, NoteOutcome)>,      // (bar_number, outcome)
    // Extras are not stored (they don't affect score).
}

impl RollingScoreWindow {
    fn add(&mut self, bar: u32, outcome: NoteOutcome) {
        self.entries.push_back((bar, outcome));
    }

    /// Remove entries that have fallen outside the window.
    fn prune(&mut self, current_bar: u32) {
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
    fn summarize(&self) -> ScoreAccumulator {
        let mut acc = ScoreAccumulator::default();
        for &(_, outcome) in &self.entries {
            match outcome {
                NoteOutcome::Hit(accuracy) => acc.record_hit(accuracy),
                NoteOutcome::Miss          => acc.record_miss(),
                NoteOutcome::WrongPiece    => acc.record_wrong_piece(),
            }
        }
        acc
    }
}
```

**Update flow each tick:**
```
On each note result:
    1. Add result to score_full (ScoreAccumulator, append-only, never pruned)
    2. Add result to rolling_8bar, rolling_16bar, rolling_32bar deques
    3. Prune all three rolling windows based on current_bar

When writing GameState snapshot for the render thread:
    score_8bar  = rolling_8bar.summarize()
    score_16bar = rolling_16bar.summarize()
    score_32bar = rolling_32bar.summarize()
```

The summarize() call is O(n) where n is the number of notes in the window. For 32
bars at typical note density (~16 notes/bar), this is ~512 entries — negligible.

### 6.4 Score Persistence

Scores are saved to SQLite when:
1. Track playback ends (full track score)
2. User explicitly runs `/scoreboard` (saves current running scores)
3. Practice mode loop completes at 100% BPM

Rolling 8/16/32 bar scores are saved as their best values during the session, not every intermediate state.

---

## 7. Audio Engine

### 7.1 Kira Setup

```rust
struct AudioEngine {
    manager: AudioManager,

    // Sub-tracks with independent volume
    metronome_track: TrackHandle,
    kit_track: TrackHandle,
    backtrack_track: TrackHandle,

    // Clock for timing
    clock: ClockHandle,

    // Loaded sound DATA (assets). StaticSoundData is the loaded-but-not-playing
    // asset. You get a StaticSoundHandle back when you call manager.play().
    metronome_hi: StaticSoundData,
    metronome_lo: StaticSoundData,
    kit_samples: HashMap<DrumPiece, StaticSoundData>,

    // Backtrack is a streaming sound (not preloaded into memory).
    // Handle exists only while the backtrack is actively playing.
    backtrack: Option<StreamingSoundHandle>,
}
```

### 7.2 Initialization

```
1. Create AudioManager with default backend settings
2. Create three sub-tracks:
   - metronome_track (volume from config)
   - kit_track (volume from config)
   - backtrack_track (volume from config)
3. Create clock with ClockSpeed::TicksPerMinute(bpm * subdivisions)
4. Load metronome samples as StaticSoundData
5. Load kit samples as StaticSoundData (one per DrumPiece)
6. If backtrack exists, prepare StreamingSoundData
```

### 7.3 Metronome Scheduling

The metronome uses kira's clock system for sample-accurate timing:

```
Given: time_sig = (numerator, denominator), bpm

beats_per_bar = numerator
clock ticks per beat = 1 (clock speed = bpm ticks/minute)

Schedule:
    For each beat 0..numerator:
        if beat == 0:
            play metronome_hi at clock_tick = bar * numerator + 0
        else:
            play metronome_lo at clock_tick = bar * numerator + beat
```

When BPM changes, update clock speed: `clock.set_speed(ClockSpeed::TicksPerMinute(new_bpm))`.

### 7.4 Kit Sample Triggering

When the game thread processes a hit (NoteResult::Hit):
```
if !mute_kit && !mute_all && audio_mode == VisualAudio:
    let sound = kit_samples[piece].clone()
        .with_modified_volume(velocity_to_volume(velocity))
        .on_track(&kit_track)
    manager.play(sound)
```

Volume mapping from MIDI velocity:
```rust
fn velocity_to_volume(vel: u8) -> f64 {
    // Quadratic curve for more natural dynamics
    let normalized = vel as f64 / 127.0;
    normalized * normalized  // 0.0 to 1.0
}
```

### 7.5 Backtrack Playback

```
On /play:
    if backtrack loaded && !mute_backtrack && !mute_all:
        Start streaming backtrack from current position
        Sync to kira clock

On /pause:
    Pause backtrack stream

On /replay:
    Seek backtrack to position 0

On /loop wrap:
    Seek backtrack to loop_start_ms
```

**Drift correction:**
Audio streaming and MIDI position tracking use independent clocks that can drift
over time. To prevent audible desync over long tracks:

1. Every 5 seconds, the game thread reads the backtrack's current playback position
   from kira (`sound.position()`) and compares it to the MIDI `position_ms`.
2. If drift exceeds **20ms**, issue a seek command to realign the backtrack to the
   MIDI position. Kira handles seeks smoothly without audible artifacts for small jumps.
3. If drift exceeds **100ms** (indicates a serious problem, e.g. audio buffer underrun),
   stop and restart the backtrack stream from the current MIDI position.
4. Log drift events for debugging but do not surface them to the user unless > 100ms.

### 7.6 Mute Controls

| Command | Effect |
|---------|--------|
| `/mute` | Toggle `mute_all`. Sets all three track volumes to 0 (or restores). |
| `/mute-metronome` | Toggle `mute_metronome`. Sets metronome_track volume to 0 or config value. |
| `/mute-backtrack` | Toggle `mute_backtrack`. Sets backtrack_track volume to 0 or config value. |
| `/mute-kit` | Toggle `mute_kit`. Sets kit_track volume to 0 or config value. |

Mute is implemented via track volume (not by skipping play commands) so that un-muting during playback resumes audio seamlessly.

---

## 8. Input System

### 8.1 Input Thread

```rust
fn input_thread(tx: Sender<TimestampedEvent>, shutdown: Arc<AtomicBool>) {
    // Use poll() with a short timeout instead of blocking read().
    // This allows the thread to check the shutdown flag periodically.
    let poll_timeout = Duration::from_millis(10);

    while !shutdown.load(Ordering::Relaxed) {
        // poll() returns Ok(true) if an event is available within the timeout.
        if crossterm::event::poll(poll_timeout).unwrap_or(false) {
            match crossterm::event::read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.kind == KeyEventKind::Press {
                        let ts = Instant::now();
                        // If send fails, the receiver was dropped — exit.
                        if tx.send(TimestampedEvent {
                            instant: ts,
                            event: InputEvent::Key(key_event),
                        }).is_err() {
                            break;
                        }
                    }
                }
                Ok(Event::Resize(cols, rows)) => {
                    let _ = tx.send(TimestampedEvent {
                        instant: Instant::now(),
                        event: InputEvent::Resize(cols, rows),
                    });
                }
                _ => {}
            }
        }
    }
    // Thread exits cleanly.
    //
    // Latency budget: The 10ms poll timeout adds up to 10ms before the event
    // reaches the game thread. Combined with the game tick interval (~8.3ms at
    // 120Hz), worst-case delivery delay is ~18ms. This does NOT affect timestamp
    // accuracy (timestamp is taken at read() time, not delivery time). It only
    // affects how quickly the game thread sees the event for scoring.
    //
    // Miss detection safety: At the tightest preset (Strict, ok_ms=40ms), the
    // 18ms worst-case delay is well within the window — a note won't be judged
    // as Miss before the delayed event arrives.
}

struct TimestampedEvent {
    instant: Instant,
    event: InputEvent,
}

enum InputEvent {
    Key(KeyEvent),
    Resize(u16, u16),
}
```

### 8.2 Vim Mode State Machine

```
Note: crossterm raw mode captures Ctrl+C — it does NOT send SIGINT.
Ctrl+C is context-dependent as described below.

NORMAL mode:
    Key press → check key_map:
        Mapped to DrumPiece → emit DrumHit event
        'i' → transition to INSERT mode, show console
        ':' → transition to INSERT mode with '/' pre-filled (vim-style)
        Ctrl+Q → quit confirmation (always available, never conflicts with drum keys)
        Ctrl+C → quit confirmation (same as Ctrl+Q in NORMAL mode)

INSERT mode:
    Key press:
        Esc → transition to NORMAL mode, clear console
        Ctrl+C → transition to NORMAL mode, clear console (cancels input, does NOT quit)
        Enter → execute console command, transition to NORMAL mode
        Tab → accept autocomplete suggestion
        Up/Down → navigate autocomplete suggestions
        Backspace → delete character
        Printable char → append to console_input, update autocomplete
```

### 8.3 Key Mapping

Two built-in presets plus custom override:

**Split-hand preset (default):**
```toml
[keys]
preset = "split"
kick = "Space"
snare = "a"
cross_stick = "s"
hihat_closed = "j"
hihat_open = "k"
hihat_pedal = "d"
ride = "l"
crash1 = "u"
crash2 = "i"
tom_high = "e"
tom_mid = "w"
tom_low = "q"
splash = "o"
china = "p"
```

**Compact preset:**
```toml
[keys]
preset = "compact"
kick = "Space"
snare = "f"
cross_stick = "d"
hihat_closed = "j"
hihat_open = "k"
hihat_pedal = "h"
ride = "l"
crash1 = "g"
crash2 = ";"
tom_high = "r"
tom_mid = "e"
tom_low = "w"
splash = "u"
china = "i"
```

### 8.4 Calibration

`/calibrate` transitions to `AppState::Calibrating`:

**State machine interaction:**
- If a session is active (Playing/Paused), playback is paused first, and the
  pre-calibration state is saved so it can be restored afterward.
- The Calibrating state has its own UI overlay (replaces the highway area).
- Input mode is forced to NORMAL (console not available during calibration).
- On completion or cancellation (Esc), the app returns to the previous state.

**Calibration flow:**
1. Show "TAP TO THE BEAT" UI with a visual metronome at 100 BPM, 4/4 time.
2. Play metronome clicks via the audio engine (uses metronome sub-track).
3. User presses any key in sync with the beat for 16 beats.
4. Discard the first 4 taps (warm-up). Use taps 5-16 (12 data points).
5. Compute median delta (not mean — median is robust to outlier taps).
6. Store result as `calibration_offset_ms` in config file.
7. Display result: "Offset: +12ms — Saved!" for 2 seconds.
8. Return to previous AppState.

**Cancellation:** Pressing Esc during calibration discards partial data and returns
to the previous state without modifying the offset.

---

## 9. Command System

### 9.1 Command Registry

```rust
struct Command {
    name: &'static str,             // e.g., "play"
    aliases: &'static [&'static str],  // e.g., &["p"]
    args: ArgSpec,
    description: &'static str,
    handler: fn(&str, &mut AppContext) -> Result<(), CommandError>,
}

enum ArgSpec {
    None,
    Optional(ArgType),
    Required(ArgType),
}

enum ArgType {
    Number,         // /bpm 120
    Text,           // /track "Basic Rock"
    Choice(&'static [&'static str]),  // /difficulty easy|medium|hard
}
```

### 9.2 Command Table

| Command | Args | Description |
|---------|------|-------------|
| `/play` | none | Start/resume playback |
| `/pause` | none | Pause playback |
| `/replay` | none | Restart from beginning |
| `/loop` | `<bars>` (opt, default: 2) | Loop next N bars |
| `/track` | `<name>` (req) | Select track by name (fuzzy) |
| `/cassette` | none | Open track browser |
| `/bpm` | `<number>` (req) | Set BPM override |
| `/kit` | `<name>` (req) | Select drum kit |
| `/difficulty` | `<easy\|medium\|hard>` (req) | Set difficulty |
| `/timing` | `<relaxed\|standard\|strict>` (req) | Set timing windows |
| `/practice` | none | Toggle practice mode (requires active loop) |
| `/scoreboard` | none | Show scores for current track |
| `/mute` | none | Toggle all audio mute |
| `/mute-metronome` | none | Toggle metronome mute |
| `/mute-backtrack` | none | Toggle backtrack mute |
| `/mute-kit` | none | Toggle kit sound mute |
| `/theme` | `<name>` (req) | Switch color theme |
| `/calibrate` | none | Run input calibration |
| `/channel` | `<number>` (req) | Override MIDI drum channel |
| `/help` | none | Show command reference |
| `/quit` | none | Exit application |

### 9.3 Autocomplete

Triggered on every keystroke in INSERT mode:

```
fn autocomplete(input: &str, commands: &[Command]) -> Vec<String>:
    if !input.starts_with('/'):
        return []

    let query = &input[1..]  // Strip leading /
    let matches: Vec<_> = commands
        .filter(|cmd| cmd.name.starts_with(query) || cmd.aliases.iter().any(|a| a.starts_with(query)))
        .map(|cmd| format!("/{}", cmd.name))
        .collect()

    // Sort: exact prefix match first, then alphabetical
    matches.sort()
    return matches (max 8 suggestions)
```

Rendering: Suggestions shown as a popup above the console input line, with the selected item highlighted.

---

## 10. UI & Rendering

### 10.1 Screen Layout

The screen is divided into regions using ratatui's `Layout`:

```
┌─────────────────────────────────────────────────┐
│ Header (2 rows)                                  │
├─────────────────────────────────────────────────┤
│                                                  │
│ Highway (remaining space - 7 rows)               │
│                                                  │
├─────────────────────────────────────────────────┤
│ Info Bar: Metronome + Score (3 rows)             │
├─────────────────────────────────────────────────┤
│ Console (2 rows)                                 │
└─────────────────────────────────────────────────┘
```

Minimum 24 rows: Header(2) + Highway(12) + Info(3) + Console(2) + borders(5) = 24.

At 40+ rows, Highway gets extra space.

### 10.2 Highway Widget

**Lane arrangement:**
Lanes are arranged horizontally to mimic a drum kit viewed from the drummer's perspective:

```
Full mode (120+ cols, up to 12 lanes):
┌──CR1──┬──HH──┬──OH──┬──TH──┬──SN──┬──KK──┬──TM──┬──TL──┬──RD──┬──RB──┬──CR2──┬──SP──┐

Compact mode (80 cols, up to 8 lanes — same core positions, outer lanes collapsed):
┌──HH──┬──TH──┬──SN──┬──KK──┬──TM──┬──TL──┬──RD──┬──CR──┐
```

Lane order (left to right, drummer's perspective — consistent across modes):

**Full mode (12 lanes):**
1. Crash 1
2. Closed Hi-Hat
3. Open Hi-Hat / Pedal Hi-Hat (shared lane, different symbols)
4. High Tom
5. Snare
6. Kick
7. Mid Tom
8. Low Tom
9. Ride
10. Ride Bell (shared lane with Ride)
11. Crash 2
12. Splash / China

**Compact mode (8 lanes):**
Core lanes (HH, TH, SN, KK, TM, TL, RD) keep the same relative positions.
Crash 1 + Crash 2 merge into one CR lane (rightmost). Open/Pedal HH merge into
the HH lane. Ride Bell merges into RD lane. Splash/China omitted (shown as CR if present).

This ensures muscle memory for SN/KK/HH positions is preserved across terminal widths.

Lanes that share a column use different note shapes to distinguish (e.g., `●` for closed HH, `◊` for open HH, `╳` for pedal HH).

**Only lanes with notes in the current track are displayed.** If the track uses only kick/snare/hi-hat, only 3-4 lanes appear (wider, centered).

**Note rendering:**
Each lane is a vertical column. Notes scroll from top to bottom. The Y position is:

```
y_offset = ((note.time_ms - position_ms) / look_ahead_ms) * highway_height
```

Where `y_offset = 0` is the top of the highway (furthest future) and `y_offset = highway_height` is the hit zone.

**Note glyphs by velocity:**
```
VelocityLevel::Ghost  → ░  (dim color)
VelocityLevel::Soft   → ▒  (medium color)
VelocityLevel::Normal → ▓  (bright color)
VelocityLevel::Accent → █  (full bright + bold)
```

**Sustained notes (duration > 0):**
Draw a tail below the note head:
```
▓  ← note head
│  ← tail (height proportional to duration)
│
╵  ← tail end
```

**Hit zone:**
A horizontal line at the bottom of the highway, rendered with `═══` characters. Highlighted with the theme's accent color.

**Hit feedback:**
When a note is hit, the hit zone cell for that lane briefly flashes:
- Perfect: bright green, text "PERFECT"
- Great: blue, text "GREAT"
- Good: yellow, text "GOOD"
- Ok: orange, text "OK"
- Miss: red, text "MISS" (when note passes unhit)
- Wrong Piece: orange, text "WRONG"

Feedback duration: 300ms, then fade.

### 10.3 Header Widget

```
 ██████ TERMINAL DRUMS ██████  ░░  Track: Basic Rock  ░░  BPM: 120  4/4
 Kit: Acoustic  ░░  [▶ PLAYING]  ░░  Bar 12/32  ░░  ◆ NORMAL ◆
```

- Track name, BPM, time signature on line 1
- Kit name, playback state, bar position, input mode on line 2
- Playback state icons: `▶ PLAYING`, `⏸ PAUSED`, `⏹ STOPPED`
- Input mode: `◆ NORMAL ◆` or `✎ INSERT ✎`
- Mute indicators: `🔇` when any mute is active

### 10.4 Metronome Widget

A visual pendulum that swings left-right in sync with the beat:

```
Frame 1 (beat 1):    Frame 2 (beat 2):    Frame 3 (beat 3):
  ╲                       │                        ╱
   ╲                      │                       ╱
    ●                     ●                      ●
```

Implementation:
- `metronome_phase` ranges from 0.0 to 1.0 per beat
- Pendulum position: `x = sin(metronome_phase * PI) * amplitude`
- Downbeat (beat 1): flash the whole widget with accent color
- Width: 12 columns
- Height: 3 rows

### 10.5 Score Widget

```
┌─ Score ──────────────────────────┐
│ 94.2%   Combo: 23x   ★ Best: 96 │
│ P:47  G:12  OK:3  M:1  W:0      │
│ ████████████████████░░░░ 78%     │ <- progress bar (track position)
└──────────────────────────────────┘
```

- Score percentage (large, prominent)
- Current combo with multiplier
- Best score for this track (from DB)
- Hit breakdown: Perfect/Great/Good+Ok/Miss/Wrong
- Track progress bar

### 10.6 Console Widget

```
NORMAL mode:
│                                                │ (empty, invisible)

INSERT mode (empty):
│ > /█                                           │

INSERT mode (typing):
│ > /pla█                                        │
│   /play  /pause  /practice                     │ ← autocomplete popup
```

- `> ` prompt prefix
- Blinking cursor (`█`)
- Autocomplete popup: rendered as a line above or below the input, with matching commands
- Selected suggestion highlighted with inverted colors

### 10.7 Scoreboard Overlay

Full-screen overlay shown when track ends or via `/scoreboard`:

```
╔══════════════════════════════════════════════════════╗
║              ★ SCOREBOARD ★                          ║
║  Track: Basic Rock  |  Difficulty: Hard              ║
║  Timing: Standard   |  BPM: 120                     ║
╠══════════════════════════════════════════════════════╣
║                                                      ║
║  THIS SESSION                                        ║
║  Full Track:  94.2%  (P:47 G:12 OK:3 M:1)  x23     ║
║  Last 8 bars: 97.1%  (P:15 G:2  OK:0 M:0)  x17     ║
║  Last 16 bars: 96.3%  (P:31 G:5  OK:1 M:0)  x22    ║
║  Last 32 bars: 95.0%  (P:44 G:10 OK:2 M:1)  x23    ║
║                                                      ║
║  PERSONAL BEST                                       ║
║  #1  96.8%  2026-04-15  x28                          ║
║  #2  95.1%  2026-04-12  x25                          ║
║  #3  94.2%  2026-04-18  x23  ← NEW                  ║
║                                                      ║
║  Press ESC to continue, R to replay                  ║
╚══════════════════════════════════════════════════════╝
```

### 10.8 Cassette Browser Overlay

Full-screen overlay for track selection:

```
╔═══════════════════════════════════════════════════╗
║  ┌─────────────────────────────────────────────┐  ║
║  │  ┌──────┐                    ┌──────┐       │  ║
║  │  │ ◉──◉ │   TERMINAL DRUMS  │ ◉──◉ │       │  ║
║  │  │ │╲╱│ │     SIDE A        │ │╲╱│ │       │  ║
║  │  └──────┘                    └──────┘       │  ║
║  │  ═══════════════════════════════════════     │  ║
║  └─────────────────────────────────────────────┘  ║
║                                                   ║
║  ▸ Basic Rock        ★★☆☆  120 BPM  Best: 94%   ║
║    Blues Shuffle      ★★★☆  90  BPM  Best: --    ║
║    Funk Groove        ★★★☆  100 BPM  Best: 78%   ║
║    Latin Beat         ★★★☆  110 BPM  Best: --    ║
║    Progressive        ★★★★  140 BPM  Best: 62%   ║
║                                                   ║
║  j/k: navigate  Enter: select  /: search  q: back ║
╚═══════════════════════════════════════════════════╝
```

The cassette reels animate (spinning `◉──◉` → `◉╲╱◉` → `◉──◉` cycle) when a track is highlighted.

### 10.9 Welcome Screen

Shown on first launch only:

```
╔═══════════════════════════════════════════════════╗
║                                                   ║
║  ████████╗██████╗ ██████╗ ██╗   ██╗███╗   ███╗   ║
║  ╚══██╔══╝██╔══██╗██╔══██╗██║   ██║████╗ ████║   ║
║     ██║   ██║  ██║██████╔╝██║   ██║██╔████╔██║   ║
║     ██║   ██║  ██║██╔══██╗██║   ██║██║╚██╔╝██║   ║
║     ██║   ██████╔╝██║  ██║╚██████╔╝██║ ╚═╝ ██║   ║
║     ╚═╝   ╚═════╝ ╚═╝  ╚═╝ ╚═════╝╚═╝     ╚═╝   ║
║              TERMINAL  DRUMS  v0.1                 ║
║                                                   ║
║     What's your name, drummer?                     ║
║     > █                                            ║
║                                                   ║
║     Press ENTER to continue                        ║
╚═══════════════════════════════════════════════════╝
```

### 10.10 Rendering Pipeline

Each frame (60 FPS):

```
1. Read current GameState snapshot from SwapBuffer (lock-free atomic read)
2. Build ratatui layout from snapshot
3. Render each widget from snapshot data:
   a. Header widget
   b. Highway widget (most expensive - iterates visible notes)
   c. Hit feedback overlay (on top of highway)
   d. Metronome widget
   e. Score widget
   f. Console widget (if INSERT mode)
   g. Autocomplete popup (if suggestions exist)
   h. Overlay (Scoreboard or Cassette, if active)
4. terminal.draw(frame)
```

### 10.11 Responsive Layout

```rust
fn build_layout(area: Rect, has_console: bool) -> Result<AppLayout, AppError> {
    let terminal_rows = area.height;
    let terminal_cols = area.width;

    // Check minimum terminal size. Return error instead of panicking —
    // the caller renders a "terminal too small" screen.
    if terminal_rows < 24 || terminal_cols < 80 {
        return Err(AppError::TerminalTooSmall {
            need_cols: 80, need_rows: 24,
            have_cols: terminal_cols, have_rows: terminal_rows,
        });
    }

    // Header always 2 rows
    let header_height = 2;

    // Console: 2 rows if INSERT mode, 1 row (status line) if NORMAL
    let console_height = if has_console { 2 } else { 1 };

    // Info bar (metronome + score): 3 rows if height >= 30, else 0 (embedded in header)
    let info_height = if terminal_rows >= 30 { 3 } else { 0 };

    // Highway gets remaining space
    let highway_height = terminal_rows - header_height - console_height - info_height - 2; // -2 for borders

    Ok(AppLayout { header_height, highway_height, info_height, console_height })
}
```

---

## 11. Theme System

### 11.1 Theme Trait

```rust
struct Theme {
    name: &'static str,

    // Base colors
    bg: Color,
    fg: Color,
    fg_dim: Color,
    accent: Color,

    // UI chrome
    border: Color,
    header_bg: Color,
    header_fg: Color,
    status_bg: Color,
    status_fg: Color,

    // Drum piece colors (used in highway lanes)
    kick: Color,
    snare: Color,
    cross_stick: Color,
    hihat: Color,           // All hi-hat variants
    crash: Color,           // All crash variants
    ride: Color,            // Ride + ride bell
    tom_high: Color,
    tom_mid: Color,
    tom_low: Color,
    splash: Color,
    china: Color,

    // Hit accuracy feedback
    perfect: Color,
    great: Color,
    good: Color,
    ok: Color,
    miss: Color,
    wrong_piece: Color,
    extra: Color,

    // Metronome
    metronome_fg: Color,
    metronome_accent: Color,  // Downbeat flash

    // Console
    console_bg: Color,
    console_fg: Color,
    console_prompt: Color,
    autocomplete_bg: Color,
    autocomplete_fg: Color,
    autocomplete_selected_bg: Color,

    // Score
    score_fg: Color,
    combo_fg: Color,
    progress_bar_fg: Color,
    progress_bar_bg: Color,
}
```

### 11.2 Bundled Themes

**gruvbox (default):**
Warm retro palette based on the Gruvbox vim color scheme.
- bg: #282828, fg: #ebdbb2, accent: #fe8019
- Drum colors: kick=#cc241d, snare=#d79921, hihat=#689d6a, ride=#458588, toms=#b16286, crash=#fabd2f
- Perfect=#b8bb26, Great=#83a598, Good=#d79921, Ok=#fe8019, Miss=#cc241d

**desert:**
Sandy warm tones.
- bg: #333333, fg: #ffa0a0, accent: #f0e68c

**evening:**
Dark with muted blue/purple accents.
- bg: #00002a, fg: #c0c0c0, accent: #7070ff

**slate:**
Gray-blue professional.
- bg: #262626, fg: #c6c8d1, accent: #6b7089

**blue:**
Cool blue dominant palette.
- bg: #1a1a2e, fg: #e0e0ff, accent: #4fc3f7

**pablo:**
High contrast, bold colors.
- bg: #000000, fg: #ffffff, accent: #ff6600

**quiet:**
Minimal, muted, low distraction.
- bg: #1c1c1c, fg: #808080, accent: #505050

**shine:**
Bright, vibrant neon.
- bg: #0a0a0a, fg: #ffffff, accent: #00ff88

**run:**
Cyberpunk/synthwave neon.
- bg: #0d0221, fg: #ff00ff, accent: #00ffff

---

## 12. Data Persistence

### 12.1 SQLite Schema

Database file: `~/.local/share/terminal-drums/scores.db`

```sql
CREATE TABLE IF NOT EXISTS profile (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,          -- Unix timestamp
    github_id TEXT,                        -- Future: GitHub user ID
    github_username TEXT                   -- Future: GitHub username
);

CREATE TABLE IF NOT EXISTS scores (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES profile(id),
    track_name TEXT NOT NULL,
    track_hash TEXT NOT NULL,             -- SHA256 of MIDI file (to detect changes)
    difficulty TEXT NOT NULL,              -- "easy", "medium", "hard"
    timing_preset TEXT NOT NULL,           -- "relaxed", "standard", "strict"
    bpm REAL NOT NULL,
    scope TEXT NOT NULL,                   -- "8bars", "16bars", "32bars", "full"
    score_pct REAL NOT NULL,
    perfect_count INTEGER NOT NULL,
    great_count INTEGER NOT NULL,
    good_count INTEGER NOT NULL,
    ok_count INTEGER NOT NULL,
    miss_count INTEGER NOT NULL,
    wrong_piece_count INTEGER NOT NULL,
    max_combo INTEGER NOT NULL,
    played_at INTEGER NOT NULL,            -- Unix timestamp
    UNIQUE(profile_id, track_name, track_hash, difficulty, timing_preset, bpm, scope, played_at)
);

CREATE INDEX idx_scores_track ON scores(track_name, scope, score_pct DESC);
CREATE INDEX idx_scores_profile ON scores(profile_id, played_at DESC);

CREATE TABLE IF NOT EXISTS preferences (
    profile_id INTEGER PRIMARY KEY REFERENCES profile(id),
    last_track TEXT,
    last_kit TEXT,
    last_theme TEXT,
    last_bpm REAL,
    last_difficulty TEXT,
    last_timing TEXT
);
```

### 12.2 Queries

**Top 10 scores for a track:**
```sql
SELECT * FROM scores
WHERE track_name = ? AND scope = ? AND difficulty = ? AND timing_preset = ?
ORDER BY score_pct DESC, max_combo DESC
LIMIT 10;
```

**Progress over time (last 30 sessions):**
```sql
SELECT score_pct, played_at FROM scores
WHERE track_name = ? AND scope = 'full' AND difficulty = ?
ORDER BY played_at DESC
LIMIT 30;
```

**Save last session preferences:**
```sql
INSERT OR REPLACE INTO preferences (profile_id, last_track, last_kit, last_theme, last_bpm, last_difficulty, last_timing)
VALUES (?, ?, ?, ?, ?, ?, ?);
```

---

## 13. File Formats

### 13.1 Track Bundle

Directory structure:
```
track-name/
├── meta.toml           # Required: track metadata
├── track.mid           # Required: MIDI file
├── backtrack.ogg       # Optional: backing audio
└── cover.txt           # Optional: ASCII art (max 40x20 chars)
```

**meta.toml:**
```toml
[track]
name = "Basic Rock"
artist = "Terminal Drums"
description = "Simple 4/4 rock beat with 8th note hi-hat"
difficulty_stars = 2           # 1-5 star rating
default_bpm = 120              # Can be overridden by user
genre = "Rock"

[midi]
channel = 10                    # Optional: override auto-detection (1-16)
# time_signature = "4/4"       # Optional: override MIDI metadata

[backtrack]
offset_ms = 0                  # Optional: offset if backtrack doesn't start at tick 0
```

### 13.2 Kit Bundle

Directory structure:
```
kit-name/
├── kit.toml            # Required: kit metadata + file mappings
├── kick.wav            # Sample files (WAV, OGG, or FLAC)
├── snare.wav
├── cross_stick.wav
├── hihat_closed.wav
├── hihat_open.wav
├── hihat_pedal.wav
├── crash1.wav
├── crash2.wav
├── ride.wav
├── ride_bell.wav
├── tom_high.wav
├── tom_mid.wav
├── tom_low.wav
├── splash.wav
└── china.wav
```

**kit.toml:**
```toml
[kit]
name = "My Acoustic Kit"
author = "Your Name"
description = "DW Collector's Series recorded at home studio"

[samples]
kick = "kick.wav"
snare = "snare.wav"
cross_stick = "cross_stick.wav"
hihat_closed = "hihat_closed.wav"
hihat_open = "hihat_open.wav"
hihat_pedal = "hihat_pedal.wav"
crash1 = "crash1.wav"
crash2 = "crash2.wav"
ride = "ride.wav"
ride_bell = "ride_bell.wav"
tom_high = "tom_high.wav"
tom_mid = "tom_mid.wav"
tom_low = "tom_low.wav"
splash = "splash.wav"
china = "china.wav"
```

Missing samples are non-fatal: the app simply won't play audio for that piece.

### 13.3 Config File

See [Section 14](#14-configuration).

---

## 14. Configuration

**Config file path:** `~/.config/terminal-drums/config.toml`

Created with defaults on first run. User edits manually (or preferences saved via app).

```toml
# Note: user name is stored in SQLite profile table (single source of truth),
# NOT in config.toml. This avoids dual source of truth.

[display]
theme = "gruvbox"               # Theme name
fps = 60                        # Render FPS (30, 60, or 120)
look_ahead_ms = 3000            # How far ahead notes are visible (ms)
show_velocity = true            # Show velocity-based note intensity
show_note_tails = true          # Show duration tails on sustained notes

[audio]
mode = "visual_audio"           # "visual_only" | "visual_audio"
metronome_volume = 0.7          # 0.0 - 1.0
kit_volume = 1.0                # 0.0 - 1.0
backtrack_volume = 0.8          # 0.0 - 1.0
input_offset_ms = 0             # Calibration offset (set by /calibrate)

[playback]
default_bpm = 0                 # 0 = use MIDI file's BPM
default_kit = "acoustic"        # Kit name
default_difficulty = "hard"     # "easy" | "medium" | "hard"
default_timing = "standard"     # "relaxed" | "standard" | "strict"

[keys]
preset = "split"                # "split" | "compact" | "custom"
# Individual overrides (only needed if preset = "custom"):
# kick = "Space"
# snare = "a"
# ...

[paths]
tracks_dir = "~/.config/terminal-drums/tracks"
kits_dir = "~/.config/terminal-drums/kits"
data_dir = "~/.local/share/terminal-drums"
```

### 14.1 Directory Resolution

On startup, the app checks/creates these directories:
```
~/.config/terminal-drums/           # Config root
~/.config/terminal-drums/tracks/    # User track bundles
~/.config/terminal-drums/kits/      # User kit bundles
~/.local/share/terminal-drums/      # Data (scores.db)
```

Track discovery order:
1. CLI argument path (if provided)
2. `tracks_dir` from config
3. `./assets/tracks/` (development fallback)

Kit discovery order:
1. `kits_dir` from config
2. `./assets/kits/` (development fallback)

---

## 15. Application Lifecycle

### 15.1 Startup Sequence

```
1. Parse CLI arguments (clap)
2. Load config file (create defaults if missing)
3. Initialize directories
4. Open SQLite database (create schema if missing)
5. Check if profile exists:
   - No profile → show Welcome screen → get name → create profile
   - Profile exists → load preferences
6. Initialize audio engine (kira)
7. Enable terminal raw mode (crossterm)
8. Create terminal (ratatui)
9. Spawn input thread
10. Load last session state or show track selection
11. Enter main game loop
```

### 15.2 Shutdown Sequence

```
1. Stop playback
2. Set input thread shutdown flag (AtomicBool → true)
3. Join input thread (will exit within 10ms poll timeout)
4. Save current preferences to SQLite
5. Drop audio engine (kira cleanup)
6. Disable terminal raw mode
7. Restore terminal state (show cursor, leave alternate screen)
8. Exit process
```

### 15.3 Panic Handler

Register a custom panic handler that:
1. Disables raw mode
2. Restores terminal
3. Prints the panic message to stderr
4. Exits with code 1

This prevents leaving the terminal in a broken state on crash.

---

## 16. Error Handling

### 16.1 Error Categories

```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("MIDI parsing failed: {0}")]
    MidiParse(String),

    #[error("No drum data found in MIDI file")]
    NoDrumData,

    #[error("Audio engine error: {0}")]
    Audio(String),

    #[error("Kit sample not found: {path}")]
    SampleNotFound { path: PathBuf },

    #[error("Track bundle missing required file: {file} in {bundle}")]
    BundleIncomplete { bundle: String, file: String },

    #[error("Config error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Terminal error: {0}")]
    Terminal(#[from] std::io::Error),

    #[error("Terminal too small: need {need_cols}x{need_rows}, got {have_cols}x{have_rows}")]
    TerminalTooSmall {
        need_cols: u16, need_rows: u16,
        have_cols: u16, have_rows: u16,
    },
}
```

### 16.2 Error Display Strategy

- **Fatal errors** (terminal init, database open): Print to stderr before raw mode, exit.
- **Recoverable errors** (bad MIDI file, missing sample): Show in console area as red text, continue running. User can fix and retry.
- **Warnings** (missing optional files like backtrack, cover art): Show briefly in status bar, continue.

---

## 17. Performance Requirements

| Metric | Target | Measurement |
|--------|--------|-------------|
| Input-to-game-state latency | < 2ms | Time from `event::read()` return to hit detection completion |
| Game tick rate | 120 Hz ± 1% | Measured via `spin_sleep` accuracy |
| Render frame rate | 60 FPS sustained | No frame drops during normal playback |
| MIDI parse time | < 50ms for 1MB file | Wall clock time from file read to DrumTrack ready |
| Audio trigger latency | < 10ms | Time from hit detection to audible sound |
| Memory usage | < 50MB | RSS during normal playback (excluding audio buffers) |
| Startup time | < 500ms | Time from process start to first frame rendered |
| Binary size | < 10MB | Release build, stripped |

### 17.1 Optimization Notes

- **Highway rendering:** Only iterate notes within the visible window, not the entire track. Binary search for window boundaries.
- **Score calculation:** Incremental update, not recalculated from scratch each tick.
- **Audio samples:** Pre-loaded into memory at session start. No disk I/O during playback.
- **State sharing:** Lock-free `SwapBuffer` with atomic index swap. Zero contention between game and render threads. No mutex, no blocking.
- **String allocation:** Pre-allocate format strings for score display. Avoid per-frame allocations.

---

## 18. Testing Specification

### 18.1 Unit Tests

**MIDI parsing (`tests/midi_parsing.rs`):**
- Parse a known MIDI file and verify note count, piece types, velocities
- Verify tempo extraction (single tempo, tempo changes)
- Verify time signature extraction (4/4, 3/4, 6/8, changes)
- Verify tick-to-ms conversion with known tempo
- Verify bar/beat calculation
- Verify drum channel auto-detection (Ch10, non-standard channel)
- Test Format 0 and Format 1 files
- Test empty/invalid MIDI files (graceful error)

**Scoring (`tests/scoring.rs`):**
- Perfect hit at exact time → HitAccuracy::Perfect
- Hit at +14ms (Standard) → Perfect, at +16ms → Great
- Hit at +79ms → Ok, at +81ms → Miss
- Wrong piece detection
- Extra hit (no expected note)
- Miss detection (note passes window)
- Combo building and breaking
- Score percentage calculation
- Rolling window accumulator (8/16/32 bars)
- All three timing presets

**Difficulty filtering (`tests/difficulty.rs`):**
- Easy mode filters ghost notes and non-core pieces
- Medium mode filters only ghost notes (vel < 40)
- Hard mode shows everything
- Downbeat detection accuracy

**Key mapping (`tests/key_map.rs`):**
- Split preset maps correctly
- Compact preset maps correctly
- Custom overrides work
- Unmapped keys return None

**Command parsing (`tests/command.rs`):**
- All commands parse correctly
- Autocomplete returns correct suggestions
- Partial match works
- Invalid command returns error
- Arguments validated (BPM must be positive number, etc.)

### 18.2 Snapshot Tests

Using `ratatui::TestBackend` + `insta`:

**Highway widget snapshots:**
- Empty highway (no notes)
- Single note at various Y positions
- Multiple lanes with notes
- Hit feedback display (each accuracy level)
- Velocity levels (ghost through accent)
- Sustained note with tail

**Layout snapshots:**
- Minimum size (80x24)
- Recommended size (120x40)
- Wide terminal (200x50)
- Console visible vs hidden

**Overlay snapshots:**
- Scoreboard with data
- Scoreboard empty (no prior scores)
- Cassette browser with tracks
- Welcome screen

**Theme snapshots:**
- Each bundled theme renders correctly (colors may not show in text snapshots, but layout is verified)

### 18.3 Integration Tests

**Full session simulation (`tests/integration.rs`):**
- Load MIDI → play → simulate key presses at correct times → verify scores
- Load MIDI → play → simulate misses → verify miss detection
- Practice mode: verify BPM increment after 90% score
- Loop mode: verify position wraps correctly
- BPM override: verify notes shift correctly

### 18.4 Test MIDI Files

Include in `tests/fixtures/`:
- `basic_4_4.mid` - Simple 4/4 rock beat, 4 bars
- `time_sig_change.mid` - Starts 4/4, changes to 3/4 at bar 5
- `tempo_change.mid` - Starts 120 BPM, changes to 140 at bar 3
- `all_pieces.mid` - Uses every DrumPiece at least once
- `ghost_notes.mid` - Contains ghost notes (vel < 40)
- `format_0.mid` - Single-track format
- `format_1.mid` - Multi-track format
- `non_standard_channel.mid` - Drums on channel 1 instead of 10

These will be created programmatically using midly's write capabilities or a simple MIDI file generator.

---

## 19. Project Structure

```
terminal-drums/
├── Cargo.toml
├── Cargo.lock
├── PLAN.md
├── SPEC.md
├── README.md
├── .gitignore
├── assets/
│   ├── kits/
│   │   └── placeholder/        # Synthesized test samples
│   │       ├── kit.toml
│   │       ├── kick.wav
│   │       ├── snare.wav
│   │       └── ...
│   ├── tracks/
│   │   └── demo/               # Demo track (programmatically generated MIDI)
│   │       ├── meta.toml
│   │       └── track.mid
│   └── metronome/
│       ├── click_hi.wav        # Generated sine pip
│       └── click_lo.wav        # Generated sine pip
├── src/
│   ├── main.rs                 # Entry point, CLI, panic handler
│   ├── app.rs                  # App struct, state machine, main loop
│   ├── config.rs               # Config loading/saving (TOML)
│   ├── error.rs                # AppError enum
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── playback.rs         # PlaybackEngine
│   │   ├── timing.rs           # Tick-to-time, bar/beat math
│   │   ├── scoring.rs          # Hit detection, ScoreAccumulator
│   │   └── practice.rs         # Practice mode logic
│   ├── midi/
│   │   ├── mod.rs
│   │   ├── parser.rs           # MIDI file → DrumTrack
│   │   ├── drum_map.rs         # GM note → DrumPiece
│   │   └── types.rs            # DrumNote, DrumTrack, etc.
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── engine.rs           # AudioEngine (kira wrapper)
│   │   ├── kit.rs              # Kit loading
│   │   ├── backtrack.rs        # Backtrack streaming
│   │   └── metronome.rs        # Metronome scheduling
│   ├── input/
│   │   ├── mod.rs
│   │   ├── thread.rs           # Input thread + TimestampedEvent
│   │   ├── vim_mode.rs         # Normal/Insert mode state machine
│   │   ├── key_map.rs          # Key → DrumPiece mapping
│   │   └── command.rs          # Command parsing + autocomplete
│   ├── ui/
│   │   ├── mod.rs              # render() entry point
│   │   ├── layout.rs           # Screen layout calculation
│   │   ├── widgets/
│   │   │   ├── mod.rs
│   │   │   ├── header.rs
│   │   │   ├── highway.rs      # Note highway (scrolling lanes)
│   │   │   ├── hit_feedback.rs # Hit/miss flash overlays
│   │   │   ├── metronome.rs    # Pendulum animation
│   │   │   ├── score.rs        # Score display
│   │   │   ├── console.rs      # Command console + autocomplete
│   │   │   ├── scoreboard.rs   # Full-screen scoreboard overlay
│   │   │   ├── cassette.rs     # Track browser overlay
│   │   │   └── welcome.rs      # First-run welcome screen
│   │   └── themes/
│   │       ├── mod.rs          # Theme struct + registry
│   │       ├── gruvbox.rs
│   │       ├── desert.rs
│   │       ├── evening.rs
│   │       ├── slate.rs
│   │       ├── blue.rs
│   │       ├── pablo.rs
│   │       ├── quiet.rs
│   │       ├── shine.rs
│   │       └── run.rs
│   └── data/
│       ├── mod.rs
│       ├── db.rs               # SQLite operations
│       ├── track_bundle.rs     # Track discovery + loading
│       ├── kit_bundle.rs       # Kit discovery + loading
│       └── profile.rs          # User profile CRUD
└── tests/
    ├── fixtures/               # Test MIDI files, sample configs
    │   ├── basic_4_4.mid
    │   ├── time_sig_change.mid
    │   └── ...
    ├── midi_parsing.rs
    ├── scoring.rs
    ├── difficulty.rs
    ├── key_map.rs
    ├── command.rs
    ├── timing.rs
    └── snapshots/              # insta snapshot files (auto-generated)
```

---

## 20. Dependency Manifest

```toml
[package]
name = "terminal-drums"
version = "0.1.0"
edition = "2021"
description = "Terminal-based drum training app with MIDI visualization and vim keybindings"
license = "MIT"
rust-version = "1.75"

[[bin]]
name = "tdrums"
path = "src/main.rs"

[dependencies]
# TUI
ratatui = "0.30"
crossterm = "0.28"

# Audio
kira = "0.9"

# MIDI
midly = "0.5"

# Timing
spin_sleep = "1"

# Concurrency
crossbeam-channel = "0.5"

# Serialization
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Paths
dirs = "5"

# Error handling
thiserror = "2"
anyhow = "1"

# Hashing (for track fingerprinting)
sha2 = "0.10"

# Future: networking
# tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
# reqwest = { version = "0.12", features = ["json"] }

[dev-dependencies]
insta = "1"
# criterion = "0.5"  # For benchmarks

[profile.release]
opt-level = 3
lto = true
strip = true
```

---

## 21. Future: Network Architecture

Not implemented in Phase 1-4, but the architecture should not preclude these features.

### 21.1 GitHub OAuth

- Device flow (CLI-friendly, no browser redirect needed)
- Endpoint: `https://github.com/login/device/code`
- Stores OAuth token in system keychain or encrypted config
- Associates local profile with GitHub user ID

### 21.2 Global Leaderboard API

```
POST /api/v1/scores       # Submit score
GET  /api/v1/scores?track=<name>&scope=<scope>&limit=10  # Top scores
GET  /api/v1/me/scores    # User's scores (auth required)
```

Scores include a replay hash to prevent tampering (hash of note results sequence).

### 21.3 Track Catalog API

```
GET  /api/v1/catalog                 # List available track bundles
GET  /api/v1/catalog/<id>/download   # Download track bundle (zip)
```

Track bundles downloaded to `tracks_dir`.

### 21.4 Architecture Preparation

- All score records include `profile_id` (local) that can be linked to GitHub identity
- `track_hash` (SHA256) ensures scores reference specific track versions
- Network module is isolated behind a trait/interface so offline mode works identically
- SQLite schema includes GitHub columns (nullable) from the start

---

## Appendix A: GM Drum Map Quick Reference

```
35 Acoustic Bass Drum    49 Crash Cymbal 1
36 Bass Drum 1           50 High Tom
37 Side Stick            51 Ride Cymbal 1
38 Acoustic Snare        52 Chinese Cymbal
39 Hand Clap             53 Ride Bell
40 Electric Snare        54 Tambourine
41 Low Floor Tom         55 Splash Cymbal
42 Closed Hi-Hat         56 Cowbell
43 High Floor Tom        57 Crash Cymbal 2
44 Pedal Hi-Hat          58 Vibraslap
45 Low Tom               59 Ride Cymbal 2
46 Open Hi-Hat
47 Low-Mid Tom
48 Hi-Mid Tom
```

## Appendix B: Timing Window Reference

| Preset | Perfect | Great | Good | Ok | Miss |
|--------|---------|-------|------|-----|------|
| Relaxed | ≤25ms | ≤50ms | ≤80ms | ≤120ms | >120ms |
| Standard | ≤15ms | ≤30ms | ≤50ms | ≤80ms | >80ms |
| Strict | ≤8ms | ≤15ms | ≤25ms | ≤40ms | >40ms |

## Appendix C: Velocity Level Reference

| Level | MIDI Velocity | Glyph | Visual |
|-------|-------------|-------|--------|
| Ghost | 1-39 | ░ | Dim color |
| Soft | 40-69 | ▒ | Medium color |
| Normal | 70-104 | ▓ | Bright color |
| Accent | 105-127 | █ | Full bright + bold |
