/// Helper module for generating test MIDI fixture files using midly.
///
/// Run with `cargo test --test midi_test_gen` to regenerate fixtures.
/// The generated files are committed to `tests/fixtures/` so the integration
/// tests do not need to regenerate them on every run.

use std::path::PathBuf;

use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    num::{u15, u24, u28, u4, u7},
};

/// Returns the path to the fixtures directory, creating it if necessary.
pub fn fixtures_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    std::fs::create_dir_all(&path).expect("failed to create fixtures directory");
    path
}

/// Build a `TrackEvent` helper (delta, kind).
fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent {
        delta: u28::from(delta),
        kind,
    }
}

fn note_on(channel: u8, key: u8, vel: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::from(channel),
        message: MidiMessage::NoteOn {
            key: u7::from(key),
            vel: u7::from(vel),
        },
    }
}

fn note_off(channel: u8, key: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::from(channel),
        message: MidiMessage::NoteOff {
            key: u7::from(key),
            vel: u7::from(0),
        },
    }
}

fn tempo_event(us_per_beat: u32) -> TrackEventKind<'static> {
    TrackEventKind::Meta(MetaMessage::Tempo(u24::from(us_per_beat)))
}

fn time_sig_event(num: u8, denom_pow: u8) -> TrackEventKind<'static> {
    // clocks_per_click=24, thirty_seconds_per_quarter=8 are standard defaults
    TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom_pow, 24, 8))
}

fn end_of_track() -> TrackEventKind<'static> {
    TrackEventKind::Meta(MetaMessage::EndOfTrack)
}

/// Generate `tests/fixtures/basic_4_4.mid`:
///
/// - 4/4 time, 120 BPM (500,000 µs/beat)
/// - tpq = 480
/// - 4 bars (16 beats)
/// - Channel 9 (0-indexed, GM drums)
/// - Kick (GM 36) on beats 1 and 3 of each bar
/// - Snare (GM 38) on beats 2 and 4 of each bar
/// - Closed hi-hat (GM 42) on every 8th note (every 240 ticks)
pub fn generate_basic_4_4() -> PathBuf {
    let tpq: u16 = 480;
    let ticks_per_beat: u32 = tpq as u32;
    let ticks_per_8th: u32 = tpq as u32 / 2;
    let note_duration: u32 = 10; // short note, 10 ticks
    let ch: u8 = 9; // GM drum channel (0-indexed)

    // Tempo track (track 0)
    let tempo_track: Vec<TrackEvent<'static>> = vec![
        event(0, tempo_event(500_000)),                // 120 BPM
        event(0, time_sig_event(4, 2)),                // 4/4 (denom=2^2=4)
        event(ticks_per_beat * 16, end_of_track()),    // 4 bars = 16 beats
    ];

    // Drum track (track 1)
    let mut drum_events: Vec<TrackEvent<'static>> = Vec::new();

    // 4 bars × 4 beats/bar = 16 beats; 8th notes are every 240 ticks
    // In each beat:
    //   - Hi-hat on the downbeat (offset 0)
    //   - Hi-hat on the upbeat (offset 240)
    //   - Kick on beats 1 and 3 (beat index 0 and 2)
    //   - Snare on beats 2 and 4 (beat index 1 and 3)
    let total_8ths: u32 = 4 * 4 * 2; // 4 bars × 4 beats × 2 8ths = 32 8th notes
    let mut last_event_abs_tick: u32 = 0;

    for eighth_idx in 0..total_8ths {
        let abs_tick = eighth_idx * ticks_per_8th;
        let beat_in_bar = (eighth_idx / 2) % 4; // 0-3 within bar
        let is_downbeat = eighth_idx % 2 == 0;  // every even 8th is a beat downbeat

        // Delta from last event
        let delta = abs_tick - last_event_abs_tick;

        // Hi-hat on every 8th note
        drum_events.push(event(delta, note_on(ch, 42, 80)));

        // Kick on beats 1 and 3 (0-indexed: beat_in_bar == 0 or 2), downbeat only
        if is_downbeat && (beat_in_bar == 0 || beat_in_bar == 2) {
            drum_events.push(event(0, note_on(ch, 36, 100)));
        }

        // Snare on beats 2 and 4 (0-indexed: beat_in_bar == 1 or 3), downbeat only
        if is_downbeat && (beat_in_bar == 1 || beat_in_bar == 3) {
            drum_events.push(event(0, note_on(ch, 38, 90)));
        }

        // Note offs (at abs_tick + note_duration)
        let off_delta = note_duration;
        drum_events.push(event(off_delta, note_off(ch, 42)));
        last_event_abs_tick = abs_tick + note_duration;

        if is_downbeat && (beat_in_bar == 0 || beat_in_bar == 2) {
            drum_events.push(event(0, note_off(ch, 36)));
        }
        if is_downbeat && (beat_in_bar == 1 || beat_in_bar == 3) {
            drum_events.push(event(0, note_off(ch, 38)));
        }

    }

    // End of track — add remaining ticks to reach 4 full bars
    let total_track_ticks = total_8ths * ticks_per_8th;
    let remaining = if total_track_ticks > last_event_abs_tick {
        total_track_ticks - last_event_abs_tick
    } else {
        0
    };
    drum_events.push(event(remaining, end_of_track()));

    let header = Header::new(
        Format::Parallel,
        Timing::Metrical(u15::from(tpq)),
    );
    let mut smf = Smf::new(header);
    smf.tracks.push(tempo_track);
    smf.tracks.push(drum_events);

    let path = fixtures_dir().join("basic_4_4.mid");
    smf.save(&path).expect("failed to save basic_4_4.mid");
    path
}

#[test]
fn generate_fixtures() {
    let path = generate_basic_4_4();
    assert!(path.exists(), "basic_4_4.mid should exist after generation");
    println!("Generated: {}", path.display());
}
