# AGENTS.md — MIDI Module

- `midly::MetaMessage::TimeSignature` denominator is a power-of-2 encoding (2 means 4, 3 means 8). The parser converts: `actual_denom = 2u8.pow(midi_denom)`.
- NoteOn with velocity 0 is a NoteOff per MIDI convention. Parser filters these. If you see missing notes, check velocity handling.
- Channel detection prefers channel 9 (0-indexed GM drum standard). Fallback: channel with most notes in 35-81 range. Override via `meta.toml` `midi_channel` field.
- `tests/midi_test_gen.rs` creates `tests/fixtures/basic_4_4.mid` at test time. The fixture doesn't exist in git — it's generated on first `cargo test` run.
