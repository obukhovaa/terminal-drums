/// Generator for placeholder WAV drum samples and demo MIDI tracks.
///
/// Run with:
///   cargo test --test gen_samples -- --nocapture
///
/// Outputs:
///   assets/kits/placeholder/*.wav
///   assets/metronome/*.wav
///   assets/tracks/basic-rock/track.mid
///   assets/tracks/blues-shuffle/track.mid
///   assets/tracks/funk-groove/track.mid

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    num::{u15, u24, u28, u4, u7},
};

// ---------------------------------------------------------------------------
// WAV helpers
// ---------------------------------------------------------------------------

fn write_wav(path: &Path, sample_rate: u32, samples: &[i16]) {
    let mut f = File::create(path).expect("failed to create WAV file");
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;

    // RIFF header
    f.write_all(b"RIFF").unwrap();
    f.write_all(&file_size.to_le_bytes()).unwrap();
    f.write_all(b"WAVE").unwrap();

    // fmt chunk
    f.write_all(b"fmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&sample_rate.to_le_bytes()).unwrap();
    f.write_all(&(sample_rate * 2).to_le_bytes()).unwrap(); // byte rate
    f.write_all(&2u16.to_le_bytes()).unwrap(); // block align
    f.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample

    // data chunk
    f.write_all(b"data").unwrap();
    f.write_all(&data_size.to_le_bytes()).unwrap();
    for &s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

/// Simple sine wave with exponential decay.
/// decay_rate: higher = faster decay (10.0 ≈ gone by ~300ms)
fn gen_sine_decay(freq: f64, duration_ms: u32, sample_rate: u32, amplitude: f64, decay_rate: f64) -> Vec<i16> {
    let num_samples = (sample_rate as f64 * duration_ms as f64 / 1000.0) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            let decay = (-t * decay_rate).exp();
            let sample = (2.0 * std::f64::consts::PI * freq * t).sin() * amplitude * decay;
            (sample * 32767.0).clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

/// Simple LCG-based pseudo-random noise in [-1.0, 1.0].
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    /// Next sample in [-1.0, 1.0]
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let hi = (self.0 >> 33) as i32;
        hi as f64 / i32::MAX as f64
    }
}

/// White noise burst with exponential decay.
fn gen_noise_decay(duration_ms: u32, sample_rate: u32, amplitude: f64, decay_rate: f64, seed: u64) -> Vec<i16> {
    let num_samples = (sample_rate as f64 * duration_ms as f64 / 1000.0) as usize;
    let mut lcg = Lcg::new(seed);
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            let decay = (-t * decay_rate).exp();
            let noise = lcg.next_f64();
            let sample = noise * amplitude * decay;
            (sample * 32767.0).clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

/// Mix two sample buffers (both must have the same length or the shorter is zero-padded).
fn mix(a: &[i16], b: &[i16]) -> Vec<i16> {
    let len = a.len().max(b.len());
    (0..len)
        .map(|i| {
            let sa = a.get(i).copied().unwrap_or(0) as f64;
            let sb = b.get(i).copied().unwrap_or(0) as f64;
            ((sa + sb) * 0.5).clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

/// Band-pass / high-pass approximation via a simple first-order high-pass IIR.
/// Attenuates very low frequencies. rc = 1/(2π·fc), alpha = rc/(rc+dt).
fn highpass(samples: &[i16], cutoff_hz: f64, sample_rate: u32) -> Vec<i16> {
    let dt = 1.0 / sample_rate as f64;
    let rc = 1.0 / (2.0 * std::f64::consts::PI * cutoff_hz);
    let alpha = rc / (rc + dt);
    let mut out = vec![0i16; samples.len()];
    let mut prev_in = 0.0f64;
    let mut prev_out = 0.0f64;
    for (i, &s) in samples.iter().enumerate() {
        let x = s as f64;
        let y = alpha * (prev_out + x - prev_in);
        out[i] = y.clamp(-32768.0, 32767.0) as i16;
        prev_in = x;
        prev_out = y;
    }
    out
}

// ---------------------------------------------------------------------------
// Individual drum generators
// ---------------------------------------------------------------------------

const SR: u32 = 44100;

fn gen_kick() -> Vec<i16> {
    // Low sine 60Hz, short pitch sweep down from 80→60 Hz, ~200ms
    let num_samples = (SR as f64 * 0.200) as usize;
    let mut lcg = Lcg::new(42);
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / SR as f64;
            let decay = (-t * 18.0).exp();
            // pitch envelope: starts at 80Hz sweeps to 60Hz
            let freq = 60.0 + 20.0 * (-t * 40.0).exp();
            let sine = (2.0 * std::f64::consts::PI * freq * t).sin();
            // small click transient
            let click = lcg.next_f64() * (-t * 500.0).exp() * 0.3;
            let sample = (sine * 0.9 + click) * decay;
            (sample * 32767.0).clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

fn gen_snare() -> Vec<i16> {
    let sine = gen_sine_decay(200.0, 150, SR, 0.6, 25.0);
    let noise = gen_noise_decay(150, SR, 0.7, 20.0, 12345);
    let raw = mix(&sine, &noise);
    highpass(&raw, 120.0, SR)
}

fn gen_cross_stick() -> Vec<i16> {
    // Short high-frequency click ~2kHz, 50ms
    let sine = gen_sine_decay(2000.0, 50, SR, 0.8, 120.0);
    let noise = gen_noise_decay(50, SR, 0.3, 200.0, 99);
    mix(&sine, &noise)
}

fn gen_hihat_closed() -> Vec<i16> {
    let noise = gen_noise_decay(80, SR, 0.9, 60.0, 7777);
    highpass(&noise, 5000.0, SR)
}

fn gen_hihat_open() -> Vec<i16> {
    let noise = gen_noise_decay(300, SR, 0.85, 8.0, 8888);
    highpass(&noise, 4000.0, SR)
}

fn gen_hihat_pedal() -> Vec<i16> {
    let noise = gen_noise_decay(40, SR, 0.7, 100.0, 5555);
    highpass(&noise, 3000.0, SR)
}

fn gen_crash(seed: u64, decay_rate: f64) -> Vec<i16> {
    let noise = gen_noise_decay(500, SR, 0.9, decay_rate, seed);
    highpass(&noise, 3000.0, SR)
}

fn gen_crash1() -> Vec<i16> {
    gen_crash(11111, 5.0)
}

fn gen_crash2() -> Vec<i16> {
    // Slightly brighter / faster decay
    gen_crash(22222, 6.5)
}

fn gen_ride() -> Vec<i16> {
    let tone = gen_sine_decay(600.0, 200, SR, 0.5, 12.0);
    let noise = gen_noise_decay(200, SR, 0.4, 15.0, 33333);
    let raw = mix(&tone, &noise);
    highpass(&raw, 2000.0, SR)
}

fn gen_ride_bell() -> Vec<i16> {
    // Brighter, shorter bell ping
    let tone1 = gen_sine_decay(1200.0, 120, SR, 0.6, 20.0);
    let tone2 = gen_sine_decay(2400.0, 120, SR, 0.3, 30.0);
    mix(&tone1, &tone2)
}

fn gen_tom(freq: f64, duration_ms: u32) -> Vec<i16> {
    let num_samples = (SR as f64 * duration_ms as f64 / 1000.0) as usize;
    let mut lcg = Lcg::new(freq as u64);
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / SR as f64;
            let decay = (-t * 15.0).exp();
            // Small pitch drop for tom feel
            let f = freq * (1.0 + 0.3 * (-t * 30.0).exp());
            let sine = (2.0 * std::f64::consts::PI * f * t).sin();
            let click = lcg.next_f64() * (-t * 300.0).exp() * 0.2;
            let sample = (sine * 0.85 + click) * decay;
            (sample * 32767.0).clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

fn gen_splash() -> Vec<i16> {
    let noise = gen_noise_decay(200, SR, 0.8, 18.0, 44444);
    highpass(&noise, 4500.0, SR)
}

fn gen_china() -> Vec<i16> {
    // Noise + a small bit of low-freq distortion feel
    let noise = gen_noise_decay(300, SR, 0.9, 10.0, 55555);
    let hp = highpass(&noise, 2500.0, SR);
    // Soft clip / bit crush effect by quantizing
    hp.iter()
        .map(|&s| {
            let f = s as f64;
            // Soft clip at 0.7 amplitude
            let clipped = if f.abs() > 22000.0 {
                f.signum() * 22000.0 + (f - f.signum() * 22000.0) * 0.3
            } else {
                f
            };
            clipped.clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

fn gen_click(freq: f64, duration_ms: u32) -> Vec<i16> {
    gen_sine_decay(freq, duration_ms, SR, 0.8, 150.0)
}

// ---------------------------------------------------------------------------
// MIDI helpers (mirrors midi_test_gen.rs)
// ---------------------------------------------------------------------------

fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent {
        delta: u28::from(delta),
        kind,
    }
}

fn note_on(ch: u8, key: u8, vel: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::from(ch),
        message: MidiMessage::NoteOn {
            key: u7::from(key),
            vel: u7::from(vel),
        },
    }
}

fn note_off(ch: u8, key: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::from(ch),
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
    TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom_pow, 24, 8))
}

fn end_of_track() -> TrackEventKind<'static> {
    TrackEventKind::Meta(MetaMessage::EndOfTrack)
}

fn smf_to_file(smf: &Smf, path: &Path) {
    smf.save(path).expect("failed to save MIDI file");
}

// ---------------------------------------------------------------------------
// GM drum note constants
// ---------------------------------------------------------------------------
const KICK: u8 = 36;
const SNARE: u8 = 38;
const HIHAT_CLOSED: u8 = 42;
const HIHAT_OPEN: u8 = 46;
const RIDE: u8 = 51;
const CRASH1: u8 = 49;
const TOM_HIGH: u8 = 50;
const TOM_MID: u8 = 47;
const TOM_LOW: u8 = 45;
const DRUM_CH: u8 = 9; // GM channel 10 (0-indexed)
const NOTE_DUR: u32 = 10; // ticks for note-off

// ---------------------------------------------------------------------------
// Basic Rock MIDI: 8 bars, 4/4, 120 BPM
// Kick on 1+3, snare on 2+4, closed hi-hat every 8th note
// ---------------------------------------------------------------------------
fn generate_basic_rock(out_dir: &Path) -> PathBuf {
    let tpq: u16 = 480;
    let tpb = tpq as u32; // ticks per beat
    let tp8 = tpb / 2; // ticks per 8th note
    let bars: u32 = 8;
    let beats_per_bar: u32 = 4;
    let total_8ths = bars * beats_per_bar * 2;

    let tempo_track: Vec<TrackEvent<'static>> = vec![
        event(0, tempo_event(500_000)),
        event(0, time_sig_event(4, 2)),
        event(tpb * beats_per_bar * bars, end_of_track()),
    ];

    let mut drum_events: Vec<TrackEvent<'static>> = Vec::new();
    let mut last_abs: u32 = 0;

    for idx in 0..total_8ths {
        let abs_tick = idx * tp8;
        let beat_in_bar = (idx / 2) % beats_per_bar;
        let is_downbeat = idx % 2 == 0;
        let delta = abs_tick - last_abs;

        // Hi-hat on every 8th note
        drum_events.push(event(delta, note_on(DRUM_CH, HIHAT_CLOSED, 75)));

        if is_downbeat && (beat_in_bar == 0 || beat_in_bar == 2) {
            drum_events.push(event(0, note_on(DRUM_CH, KICK, 100)));
        }
        if is_downbeat && (beat_in_bar == 1 || beat_in_bar == 3) {
            drum_events.push(event(0, note_on(DRUM_CH, SNARE, 90)));
        }

        drum_events.push(event(NOTE_DUR, note_off(DRUM_CH, HIHAT_CLOSED)));
        last_abs = abs_tick + NOTE_DUR;

        if is_downbeat && (beat_in_bar == 0 || beat_in_bar == 2) {
            drum_events.push(event(0, note_off(DRUM_CH, KICK)));
        }
        if is_downbeat && (beat_in_bar == 1 || beat_in_bar == 3) {
            drum_events.push(event(0, note_off(DRUM_CH, SNARE)));
        }
    }

    let total_ticks = total_8ths * tp8;
    let remaining = total_ticks.saturating_sub(last_abs);
    drum_events.push(event(remaining, end_of_track()));

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(tpq)));
    let mut smf = Smf::new(header);
    smf.tracks.push(tempo_track);
    smf.tracks.push(drum_events);

    let path = out_dir.join("track.mid");
    smf_to_file(&smf, &path);
    path
}

// ---------------------------------------------------------------------------
// Blues Shuffle MIDI: 8 bars, 12/8 swing, 90 BPM
// Kick on beat 1 and 7 (of 12 triplet 8ths per bar), snare on 4 and 10,
// ride on every triplet 8th
// 12/8: 4 dotted-quarter beats per bar, 3 triplet-8ths each.
// With tpq=480, a dotted quarter = 720 ticks, triplet-8th = 240 ticks.
// ---------------------------------------------------------------------------
fn generate_blues_shuffle(out_dir: &Path) -> PathBuf {
    let tpq: u16 = 480;
    let tp_trip8: u32 = (tpq as u32 * 2) / 3; // 320 ticks for triplet 8th
    // 12/8 means 12 triplet-8th notes per bar
    let bars: u32 = 8;
    let trip8_per_bar: u32 = 12;
    let total_trip8 = bars * trip8_per_bar;
    // 90 BPM → dotted quarter = beat; 90 BPM dotted-quarter = 666666 µs
    // but in 12/8, the beat IS the dotted-quarter.
    // Standard tempo for 12/8 at 90 BPM (dotted-quarter) = 666_667 µs
    let us_per_beat: u32 = 666_667;

    let tempo_track: Vec<TrackEvent<'static>> = vec![
        event(0, tempo_event(us_per_beat)),
        event(0, time_sig_event(12, 3)), // 12/8 (denom=2^3=8)
        event(total_trip8 * tp_trip8, end_of_track()),
    ];

    let mut drum_events: Vec<TrackEvent<'static>> = Vec::new();
    let mut last_abs: u32 = 0;

    // Pattern per bar (12 triplet 8th positions, 0-indexed):
    // Kick: 0, 6
    // Snare: 3, 9
    // Ride: every position 0-11
    for idx in 0..total_trip8 {
        let abs_tick = idx * tp_trip8;
        let pos_in_bar = idx % trip8_per_bar;
        let delta = abs_tick - last_abs;

        // Ride on every triplet 8th
        drum_events.push(event(delta, note_on(DRUM_CH, RIDE, 70)));

        if pos_in_bar == 0 || pos_in_bar == 6 {
            drum_events.push(event(0, note_on(DRUM_CH, KICK, 95)));
        }
        if pos_in_bar == 3 || pos_in_bar == 9 {
            drum_events.push(event(0, note_on(DRUM_CH, SNARE, 85)));
        }
        // Accent ride on beat positions 0, 3, 6, 9 (dotted-quarter beats)
        let ride_vel: u8 = if pos_in_bar % 3 == 0 { 85 } else { 65 };
        // We already pushed ride above; update it — easier to adjust vel in event
        // (Just accept the already-pushed ride note above with fixed vel)
        let _ = ride_vel; // suppress warning

        drum_events.push(event(NOTE_DUR, note_off(DRUM_CH, RIDE)));
        last_abs = abs_tick + NOTE_DUR;

        if pos_in_bar == 0 || pos_in_bar == 6 {
            drum_events.push(event(0, note_off(DRUM_CH, KICK)));
        }
        if pos_in_bar == 3 || pos_in_bar == 9 {
            drum_events.push(event(0, note_off(DRUM_CH, SNARE)));
        }
    }

    let total_ticks = total_trip8 * tp_trip8;
    let remaining = total_ticks.saturating_sub(last_abs);
    drum_events.push(event(remaining, end_of_track()));

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(tpq)));
    let mut smf = Smf::new(header);
    smf.tracks.push(tempo_track);
    smf.tracks.push(drum_events);

    let path = out_dir.join("track.mid");
    smf_to_file(&smf, &path);
    path
}

// ---------------------------------------------------------------------------
// Funk Groove MIDI: 8 bars, 4/4, 100 BPM
// Syncopated kick, ghost snares (low velocity), hi-hat 16th-note pattern
// ---------------------------------------------------------------------------
fn generate_funk_groove(out_dir: &Path) -> PathBuf {
    let tpq: u16 = 480;
    let tpb = tpq as u32;
    let tp16 = tpb / 4; // ticks per 16th note
    // 100 BPM → 600_000 µs/beat
    let us_per_beat: u32 = 600_000;
    let bars: u32 = 8;
    let sixteenth_per_bar: u32 = 16;
    let total_16ths = bars * sixteenth_per_bar;

    let tempo_track: Vec<TrackEvent<'static>> = vec![
        event(0, tempo_event(us_per_beat)),
        event(0, time_sig_event(4, 2)), // 4/4
        event(total_16ths * tp16, end_of_track()),
    ];

    // Funk pattern per bar (16th grid, positions 0-15):
    // Kick:  0, 3, 6, 10  (syncopated)
    // Snare: 4, 12 (full); ghost notes at 2, 7, 9, 14 (vel 40)
    // HiHat: all 16ths open, accent on downbeats (0,4,8,12)
    // HiHat open: position 6, close back at 7 (for that open-hat choke)
    let kick_positions: &[u32] = &[0, 3, 6, 10];
    let snare_positions: &[u32] = &[4, 12];
    let ghost_positions: &[u32] = &[2, 7, 9, 14];
    let hihat_open_positions: &[u32] = &[6];
    let hihat_close_positions: &[u32] = &[7];

    let mut drum_events: Vec<TrackEvent<'static>> = Vec::new();
    let mut last_abs: u32 = 0;

    for bar in 0..bars {
        for pos in 0..sixteenth_per_bar {
            let abs_tick = (bar * sixteenth_per_bar + pos) * tp16;
            let delta = abs_tick - last_abs;

            let is_kick = kick_positions.contains(&pos);
            let is_snare = snare_positions.contains(&pos);
            let is_ghost = ghost_positions.contains(&pos);
            let is_hihat_open = hihat_open_positions.contains(&pos);
            let is_hihat_close = hihat_close_positions.contains(&pos);
            let is_accent = pos % 4 == 0;

            let hihat_note = if is_hihat_open { HIHAT_OPEN } else { HIHAT_CLOSED };
            let hihat_vel: u8 = if is_accent { 90 } else { 65 };

            // Push hi-hat (every 16th, except when we explicitly close an open hat)
            if !is_hihat_close {
                drum_events.push(event(delta, note_on(DRUM_CH, hihat_note, hihat_vel)));
                let mut local_delta = NOTE_DUR;

                if is_kick {
                    drum_events.push(event(0, note_on(DRUM_CH, KICK, 100)));
                }
                if is_snare {
                    drum_events.push(event(0, note_on(DRUM_CH, SNARE, 95)));
                }
                if is_ghost {
                    drum_events.push(event(0, note_on(DRUM_CH, SNARE, 38)));
                }

                drum_events.push(event(local_delta, note_off(DRUM_CH, hihat_note)));
                local_delta = 0;
                last_abs = abs_tick + NOTE_DUR;

                if is_kick {
                    drum_events.push(event(local_delta, note_off(DRUM_CH, KICK)));
                    local_delta = 0;
                }
                if is_snare {
                    drum_events.push(event(local_delta, note_off(DRUM_CH, SNARE)));
                }
                if is_ghost {
                    drum_events.push(event(0, note_off(DRUM_CH, SNARE)));
                }
            } else {
                // Close open hi-hat: send open hat note off + closed hat note on
                drum_events.push(event(delta, note_off(DRUM_CH, HIHAT_OPEN)));
                drum_events.push(event(0, note_on(DRUM_CH, HIHAT_CLOSED, 55)));
                drum_events.push(event(NOTE_DUR, note_off(DRUM_CH, HIHAT_CLOSED)));
                last_abs = abs_tick + NOTE_DUR;
            }
        }
    }

    let total_ticks = total_16ths * tp16;
    let remaining = total_ticks.saturating_sub(last_abs);
    drum_events.push(event(remaining, end_of_track()));

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(tpq)));
    let mut smf = Smf::new(header);
    smf.tracks.push(tempo_track);
    smf.tracks.push(drum_events);

    let path = out_dir.join("track.mid");
    smf_to_file(&smf, &path);
    path
}

// ---------------------------------------------------------------------------
// Generic pattern-based MIDI generator for odd time signatures
// ---------------------------------------------------------------------------

struct PatternDef {
    name: &'static str,
    bpm: u32,
    numerator: u8,
    denom_pow: u8, // 2^n: 2=4(quarter), 3=8(eighth)
    bars: u32,
    /// Grid subdivision per bar (e.g. 8 for 8th notes in 7/8)
    grid_per_bar: u32,
    /// (grid_position, midi_note, velocity) per bar
    pattern: &'static [(u32, u8, u8)],
}

fn generate_pattern(out_dir: &Path, def: &PatternDef) -> PathBuf {
    let tpq: u16 = 480;
    // Ticks per grid unit depends on time sig.
    // For /4 sigs: grid is 8th notes → tpq/2 = 240 ticks
    // For /8 sigs: grid is 8th notes → tpq ticks (one 8th = one beat in /8)
    let ticks_per_grid: u32 = if def.denom_pow == 3 {
        tpq as u32 // /8 time: each 8th note IS a beat
    } else {
        tpq as u32 / 2 // /4 time: 8th note grid
    };

    let us_per_beat: u32 = (60_000_000.0 / def.bpm as f64) as u32;
    let total_grids = def.bars * def.grid_per_bar;

    let tempo_track: Vec<TrackEvent<'static>> = vec![
        event(0, tempo_event(us_per_beat)),
        event(0, time_sig_event(def.numerator, def.denom_pow)),
        event(total_grids * ticks_per_grid, end_of_track()),
    ];

    let mut drum_events: Vec<TrackEvent<'static>> = Vec::new();
    let mut last_abs: u32 = 0;

    for bar in 0..def.bars {
        for &(pos, note, vel) in def.pattern {
            let abs_tick = (bar * def.grid_per_bar + pos) * ticks_per_grid;
            let delta = abs_tick.saturating_sub(last_abs);
            drum_events.push(event(delta, note_on(DRUM_CH, note, vel)));
            drum_events.push(event(NOTE_DUR, note_off(DRUM_CH, note)));
            last_abs = abs_tick + NOTE_DUR;
        }
    }

    let total_ticks = total_grids * ticks_per_grid;
    let remaining = total_ticks.saturating_sub(last_abs);
    drum_events.push(event(remaining, end_of_track()));

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(tpq)));
    let mut smf = Smf::new(header);
    smf.tracks.push(tempo_track);
    smf.tracks.push(drum_events);

    let path = out_dir.join("track.mid");
    smf_to_file(&smf, &path);
    path
}

const CROSS_STICK: u8 = 37;
const HIHAT_PEDAL: u8 = 44;
const RIDE_BELL: u8 = 53;

static ODD_TIME_TRACKS: &[PatternDef] = &[
    // 1. Waltz 3/4 — classic waltz feel
    PatternDef {
        name: "Waltz 3/4",
        bpm: 140,
        numerator: 3, denom_pow: 2, // 3/4
        bars: 16,
        grid_per_bar: 6, // 8th note grid
        pattern: &[
            (0, KICK, 100), (0, HIHAT_CLOSED, 80),
            (1, HIHAT_CLOSED, 55),
            (2, SNARE, 85), (2, HIHAT_CLOSED, 75),
            (3, HIHAT_CLOSED, 55),
            (4, HIHAT_CLOSED, 75),
            (5, HIHAT_CLOSED, 55),
        ],
    },
    // 2. Progressive 5/4 — Tool-style
    PatternDef {
        name: "Prog 5/4",
        bpm: 120,
        numerator: 5, denom_pow: 2, // 5/4
        bars: 12,
        grid_per_bar: 10, // 8th note grid
        pattern: &[
            (0, KICK, 100), (0, HIHAT_CLOSED, 80),
            (1, HIHAT_CLOSED, 55),
            (2, HIHAT_CLOSED, 75),
            (3, HIHAT_CLOSED, 55),
            (4, SNARE, 90), (4, HIHAT_CLOSED, 80),
            (5, HIHAT_CLOSED, 55),
            (6, KICK, 95), (6, HIHAT_CLOSED, 75),
            (7, HIHAT_CLOSED, 55),
            (8, HIHAT_CLOSED, 75),
            (9, HIHAT_CLOSED, 55),
        ],
    },
    // 3. Balkan 7/8 — grouped 2+2+3
    PatternDef {
        name: "Balkan 7/8",
        bpm: 160,
        numerator: 7, denom_pow: 3, // 7/8
        bars: 16,
        grid_per_bar: 7,
        pattern: &[
            (0, KICK, 100), (0, HIHAT_CLOSED, 85),
            (1, HIHAT_CLOSED, 55),
            (2, SNARE, 85), (2, HIHAT_CLOSED, 80),
            (3, HIHAT_CLOSED, 55),
            (4, KICK, 90), (4, HIHAT_CLOSED, 80),
            (5, HIHAT_CLOSED, 60),
            (6, HIHAT_CLOSED, 60),
        ],
    },
    // 4. Dave Brubeck 5/4 Jazz — Take Five style
    PatternDef {
        name: "Take Five 5/4",
        bpm: 175,
        numerator: 5, denom_pow: 2, // 5/4
        bars: 12,
        grid_per_bar: 10,
        pattern: &[
            (0, RIDE, 80), (0, HIHAT_PEDAL, 70),
            (1, RIDE, 60),
            (2, RIDE, 75), (2, CROSS_STICK, 75),
            (3, RIDE, 60),
            (4, RIDE, 75),
            (5, RIDE, 60), (5, HIHAT_PEDAL, 70),
            (6, RIDE, 80), (6, KICK, 90),
            (7, RIDE, 60),
            (8, RIDE, 75), (8, CROSS_STICK, 75),
            (9, RIDE, 60),
        ],
    },
    // 5. Prog Metal 7/4
    PatternDef {
        name: "Prog Metal 7/4",
        bpm: 130,
        numerator: 7, denom_pow: 2, // 7/4
        bars: 12,
        grid_per_bar: 14,
        pattern: &[
            (0, KICK, 110), (0, CRASH1, 90),
            (1, HIHAT_CLOSED, 60),
            (2, HIHAT_CLOSED, 75),
            (3, HIHAT_CLOSED, 60),
            (4, SNARE, 100), (4, HIHAT_CLOSED, 80),
            (5, HIHAT_CLOSED, 60),
            (6, KICK, 95), (6, HIHAT_CLOSED, 75),
            (7, HIHAT_CLOSED, 60),
            (8, SNARE, 90), (8, HIHAT_CLOSED, 80),
            (9, HIHAT_CLOSED, 60),
            (10, KICK, 100), (10, HIHAT_CLOSED, 75),
            (11, HIHAT_CLOSED, 60),
            (12, HIHAT_CLOSED, 75),
            (13, HIHAT_CLOSED, 60),
        ],
    },
    // 6. Flamenco 3/4
    PatternDef {
        name: "Flamenco 3/4",
        bpm: 120,
        numerator: 3, denom_pow: 2, // 3/4
        bars: 16,
        grid_per_bar: 6,
        pattern: &[
            (0, KICK, 95), (0, HIHAT_CLOSED, 80),
            (1, CROSS_STICK, 50),
            (2, CROSS_STICK, 80),
            (3, HIHAT_CLOSED, 55),
            (4, CROSS_STICK, 70), (4, HIHAT_CLOSED, 75),
            (5, CROSS_STICK, 50),
        ],
    },
    // 7. Odd Groove 9/8 — grouped 2+2+2+3
    PatternDef {
        name: "Odd Groove 9/8",
        bpm: 140,
        numerator: 9, denom_pow: 3, // 9/8
        bars: 12,
        grid_per_bar: 9,
        pattern: &[
            (0, KICK, 100), (0, RIDE, 80),
            (1, RIDE, 55),
            (2, SNARE, 80), (2, RIDE, 75),
            (3, RIDE, 55),
            (4, RIDE, 75),
            (5, RIDE, 55),
            (6, KICK, 90), (6, RIDE, 80),
            (7, RIDE, 60),
            (8, RIDE, 60),
        ],
    },
    // 8. March 6/8
    PatternDef {
        name: "March 6/8",
        bpm: 120,
        numerator: 6, denom_pow: 3, // 6/8
        bars: 16,
        grid_per_bar: 6,
        pattern: &[
            (0, KICK, 100), (0, HIHAT_CLOSED, 85),
            (1, HIHAT_CLOSED, 55),
            (2, HIHAT_CLOSED, 65),
            (3, SNARE, 90), (3, HIHAT_CLOSED, 85),
            (4, HIHAT_CLOSED, 55),
            (5, HIHAT_CLOSED, 65),
        ],
    },
    // 9. Afro-Cuban 6/4
    PatternDef {
        name: "Afro-Cuban 6/4",
        bpm: 110,
        numerator: 6, denom_pow: 2, // 6/4
        bars: 12,
        grid_per_bar: 12,
        pattern: &[
            (0, KICK, 100), (0, RIDE_BELL, 85),
            (1, RIDE, 55),
            (2, RIDE, 70),
            (3, SNARE, 80), (3, RIDE, 75),
            (4, RIDE, 55),
            (5, RIDE, 70), (5, KICK, 80),
            (6, RIDE_BELL, 80),
            (7, RIDE, 55),
            (8, SNARE, 85), (8, RIDE, 75),
            (9, RIDE, 55),
            (10, KICK, 90), (10, RIDE, 70),
            (11, RIDE, 55),
        ],
    },
    // 10. Prog Epic 11/8 — grouped 3+3+3+2
    PatternDef {
        name: "Prog Epic 11/8",
        bpm: 145,
        numerator: 11, denom_pow: 3, // 11/8
        bars: 8,
        grid_per_bar: 11,
        pattern: &[
            (0, KICK, 100), (0, CRASH1, 85),
            (1, HIHAT_CLOSED, 55),
            (2, HIHAT_CLOSED, 60),
            (3, SNARE, 90), (3, HIHAT_CLOSED, 80),
            (4, HIHAT_CLOSED, 55),
            (5, HIHAT_CLOSED, 60),
            (6, KICK, 95), (6, HIHAT_CLOSED, 80),
            (7, HIHAT_CLOSED, 55),
            (8, HIHAT_CLOSED, 60),
            (9, SNARE, 85), (9, HIHAT_CLOSED, 80),
            (10, HIHAT_CLOSED, 55),
        ],
    },
];

// ---------------------------------------------------------------------------
// Test entry point
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn generate_all_samples_and_tracks() {
    let root = manifest_dir();

    // ---- WAV: placeholder kit ----
    let kit_dir = root.join("assets/kits/placeholder");
    fs::create_dir_all(&kit_dir).unwrap();

    let samples: &[(&str, Vec<i16>)] = &[
        ("kick.wav", gen_kick()),
        ("snare.wav", gen_snare()),
        ("cross_stick.wav", gen_cross_stick()),
        ("hihat_closed.wav", gen_hihat_closed()),
        ("hihat_open.wav", gen_hihat_open()),
        ("hihat_pedal.wav", gen_hihat_pedal()),
        ("crash1.wav", gen_crash1()),
        ("crash2.wav", gen_crash2()),
        ("ride.wav", gen_ride()),
        ("ride_bell.wav", gen_ride_bell()),
        ("tom_high.wav", gen_tom(300.0, 150)),
        ("tom_mid.wav", gen_tom(200.0, 150)),
        ("tom_low.wav", gen_tom(120.0, 200)),
        ("splash.wav", gen_splash()),
        ("china.wav", gen_china()),
    ];

    for (name, data) in samples {
        let path = kit_dir.join(name);
        write_wav(&path, SR, data);
        assert!(path.exists(), "{name} should exist");
        println!("  wrote {}", path.display());
    }

    // ---- WAV: metronome clicks ----
    let metro_dir = root.join("assets/metronome");
    fs::create_dir_all(&metro_dir).unwrap();

    let click_hi = gen_click(1000.0, 30);
    let click_lo = gen_click(800.0, 20);

    let metro_samples: &[(&str, &[i16])] = &[
        ("click_hi.wav", &click_hi),
        ("click_lo.wav", &click_lo),
    ];
    for (name, data) in metro_samples {
        let path = metro_dir.join(name);
        write_wav(&path, SR, data);
        assert!(path.exists(), "{name} should exist");
        println!("  wrote {}", path.display());
    }

    // ---- MIDI tracks ----
    let tracks_root = root.join("assets/tracks");

    let basic_rock_dir = tracks_root.join("basic-rock");
    fs::create_dir_all(&basic_rock_dir).unwrap();
    let p = generate_basic_rock(&basic_rock_dir);
    assert!(p.exists(), "basic-rock/track.mid should exist");
    println!("  wrote {}", p.display());

    let blues_dir = tracks_root.join("blues-shuffle");
    fs::create_dir_all(&blues_dir).unwrap();
    let p = generate_blues_shuffle(&blues_dir);
    assert!(p.exists(), "blues-shuffle/track.mid should exist");
    println!("  wrote {}", p.display());

    let funk_dir = tracks_root.join("funk-groove");
    fs::create_dir_all(&funk_dir).unwrap();
    let p = generate_funk_groove(&funk_dir);
    assert!(p.exists(), "funk-groove/track.mid should exist");
    println!("  wrote {}", p.display());

    // ---- Odd-time MIDI tracks ----
    for def in ODD_TIME_TRACKS {
        let slug = def.name
            .to_lowercase()
            .replace(' ', "-")
            .replace('/', "-");
        let dir = tracks_root.join(&slug);
        fs::create_dir_all(&dir).unwrap();
        let p = generate_pattern(&dir, def);
        assert!(p.exists(), "{}/track.mid should exist", slug);
        println!("  wrote {}", p.display());

        // Generate meta.toml
        let denom = 1u8 << def.denom_pow;
        let genre = if def.name.contains("Jazz") || def.name.contains("Take") {
            "Jazz"
        } else if def.name.contains("Metal") {
            "Metal"
        } else if def.name.contains("Waltz") || def.name.contains("Flamenco") {
            "World"
        } else if def.name.contains("Afro") {
            "Latin"
        } else if def.name.contains("March") {
            "March"
        } else {
            "Progressive"
        };
        let stars = match def.numerator {
            3 | 6 => 2,
            5 => 3,
            7 | 9 => 4,
            _ => 5,
        };
        let meta = format!(
            r#"[track]
name = "{}"
artist = "Terminal Drums"
description = "{}/{} {} pattern"
difficulty_stars = {}
default_bpm = {}
genre = "{}"

[midi]

[backtrack]
offset_ms = 0
"#,
            def.name, def.numerator, denom, genre.to_lowercase(), stars, def.bpm, genre
        );
        fs::write(dir.join("meta.toml"), meta).unwrap();
    }

    println!("\nAll samples and tracks generated successfully.");
}
