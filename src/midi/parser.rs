use std::collections::{HashMap, HashSet};
use std::path::Path;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use crate::engine::timing::{compute_bar_beat, ticks_to_ms};
use crate::error::AppError;
use crate::midi::drum_map::midi_note_to_drum_piece;
use crate::midi::types::{DrumNote, DrumTrack, TempoEvent, TimeSignatureEvent};

/// Parse a MIDI file and extract drum data as a `DrumTrack`.
///
/// Pipeline (spec §4.1):
/// 1. Parse raw bytes via `midly::Smf::parse`.
/// 2. Extract `ticks_per_quarter` from header.
/// 3. Iterate all tracks to collect tempo events, time signature events, and raw note data.
/// 4. Detect the drum channel (spec §4.2).
/// 5. Convert raw note data to `DrumNote` using timing helpers.
/// 6. Sort notes by `time_ms`, compute totals, build `pieces_used`.
pub fn parse_midi_file(path: &Path) -> Result<DrumTrack, AppError> {
    // Step 1 – read and parse raw bytes
    let raw = std::fs::read(path)
        .map_err(|e| AppError::MidiParse(format!("cannot read file: {e}")))?;
    let smf = Smf::parse(&raw)
        .map_err(|e| AppError::MidiParse(format!("midly parse error: {e}")))?;

    // Step 2 – extract ticks_per_quarter
    let tpq: u16 = match smf.header.timing {
        Timing::Metrical(tpq) => tpq.as_int(),
        Timing::Timecode(_, _) => {
            return Err(AppError::MidiParse(
                "SMPTE timecode MIDI files are not supported".into(),
            ));
        }
    };

    // Step 3 – one-pass: collect tempo events, time sigs, and per-channel note counts
    // We need a two-pass approach: first collect metadata + channel counts, then extract notes.

    // Pass A: extract tempo, time signatures, and count notes per channel
    let mut tempo_map: Vec<TempoEvent> = Vec::new();
    let mut time_signatures: Vec<TimeSignatureEvent> = Vec::new();
    // channel → count of drum-range notes (35–81)
    let mut channel_drum_counts: HashMap<u8, u32> = HashMap::new();
    // channel → total note count (any note)
    let mut channel_note_counts: HashMap<u8, u32> = HashMap::new();

    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += event.delta.as_int() as u64;
            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(us)) => {
                    tempo_map.push(TempoEvent {
                        tick: abs_tick,
                        microseconds_per_quarter: us.as_int(),
                    });
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom_pow, _, _)) => {
                    let denominator = 1u8 << denom_pow; // power-of-2 → actual value
                    time_signatures.push(TimeSignatureEvent {
                        tick: abs_tick,
                        numerator: num,
                        denominator,
                    });
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            *channel_note_counts.entry(ch).or_insert(0) += 1;
                            if key.as_int() >= 35 && key.as_int() <= 81 {
                                *channel_drum_counts.entry(ch).or_insert(0) += 1;
                            }
                        }
                        // NoteOff or NoteOn with vel=0 — count them for note tracking later
                        // but we don't count them here for channel detection
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // Ensure events are sorted by tick (spec says they must be)
    tempo_map.sort_by_key(|e| e.tick);
    time_signatures.sort_by_key(|e| e.tick);

    // Step 4 – detect drum channel (spec §4.2)
    let drum_channel = detect_drum_channel(&channel_note_counts, &channel_drum_counts)?;

    // Pass B: extract NoteOn/NoteOff events for the drum channel, build DrumNotes
    // We need to track NoteOff times to compute duration_ms.
    // Key: MIDI note number → (abs_tick_of_noteon, note_vec_index)
    let mut pending_notes: HashMap<u8, (u64, usize)> = HashMap::new();
    let mut notes: Vec<DrumNote> = Vec::new();

    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += event.delta.as_int() as u64;
            if let TrackEventKind::Midi { channel, message } = event.kind {
                if channel.as_int() != drum_channel {
                    continue;
                }
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        let key_u8 = key.as_int();
                        let vel_u8 = vel.as_int();

                        if vel_u8 == 0 {
                            // NoteOn with vel=0 is a NoteOff (spec §4.1 step 3)
                            finish_note(&mut pending_notes, &mut notes, key_u8, abs_tick, tpq, &tempo_map);
                        } else {
                            // Real NoteOn — skip notes we can't map to a DrumPiece
                            if let Some(piece) = midi_note_to_drum_piece(key_u8) {
                                let time_ms = ticks_to_ms(abs_tick, &tempo_map, tpq);
                                let (bar, beat) =
                                    compute_bar_beat(abs_tick, &time_signatures, tpq);
                                let idx = notes.len();
                                notes.push(DrumNote {
                                    piece,
                                    tick: abs_tick,
                                    time_ms,
                                    velocity: vel_u8,
                                    duration_ms: 0.0, // filled in when NoteOff arrives
                                    bar,
                                    beat,
                                });
                                pending_notes.insert(key_u8, (abs_tick, idx));
                            }
                        }
                    }
                    MidiMessage::NoteOff { key, vel: _ } => {
                        let key_u8 = key.as_int();
                        finish_note(&mut pending_notes, &mut notes, key_u8, abs_tick, tpq, &tempo_map);
                    }
                    _ => {}
                }
            }
        }
    }

    if notes.is_empty() {
        return Err(AppError::NoDrumData);
    }

    // Step 5 – sort by time_ms ascending
    notes.sort_by(|a, b| a.time_ms.partial_cmp(&b.time_ms).unwrap());

    // Step 6 – compute totals
    let duration_ms = notes
        .last()
        .map(|n| n.time_ms + n.duration_ms.max(0.0))
        .unwrap_or(0.0);

    let pieces_used: HashSet<_> = notes.iter().map(|n| n.piece).collect();

    // Compute total_bars from the last note's bar position (+1 for the bar it's in)
    let total_bars = notes
        .last()
        .map(|n| n.bar + 1)
        .unwrap_or(1);

    // Use the file stem as the track name
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(DrumTrack {
        name,
        notes,
        tempo_map,
        time_signatures,
        ticks_per_quarter: tpq,
        duration_ms,
        total_bars,
        pieces_used,
    })
}

/// Fill in the `duration_ms` for a pending NoteOn when we encounter its matching NoteOff.
fn finish_note(
    pending_notes: &mut HashMap<u8, (u64, usize)>,
    notes: &mut Vec<DrumNote>,
    key: u8,
    off_tick: u64,
    tpq: u16,
    tempo_map: &[TempoEvent],
) {
    if let Some((on_tick, idx)) = pending_notes.remove(&key) {
        let on_ms = ticks_to_ms(on_tick, tempo_map, tpq);
        let off_ms = ticks_to_ms(off_tick, tempo_map, tpq);
        notes[idx].duration_ms = (off_ms - on_ms).max(0.0);
    }
}

/// Detect the MIDI channel that carries drum data (spec §4.2).
///
/// Preference order:
/// 1. Channel 9 (0-indexed GM drum channel) if it has any notes.
/// 2. Otherwise, the channel with the highest count of notes in GM drum range (35–81).
///
/// Returns `Err(AppError::NoDrumData)` if no suitable channel is found.
fn detect_drum_channel(
    channel_note_counts: &HashMap<u8, u32>,
    channel_drum_counts: &HashMap<u8, u32>,
) -> Result<u8, AppError> {
    // Prefer channel 9 (0-indexed) if it has notes
    if channel_note_counts.get(&9).copied().unwrap_or(0) > 0 {
        return Ok(9);
    }

    // Otherwise pick the channel with the most drum-range notes
    let best = channel_drum_counts
        .iter()
        .filter(|(_, &count)| count > 0)
        .max_by_key(|(_, &count)| count)
        .map(|(&ch, _)| ch);

    best.ok_or(AppError::NoDrumData)
}
