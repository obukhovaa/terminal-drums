# Terminal Drums

A TUI drum trainer for programmers. Load any MIDI file, see notes scroll down a highway, hit the right keys in time, and get scored on accuracy. Built with Rust, ratatui, kira, and midly.

Vim-style modal input: you play drums in Normal mode and type commands in Insert mode.

## Install

Requires Rust 1.75+.

```
cargo install --path .
```

The binary is called `tdrums`.

## Quick Start

```
tdrums assets/tracks/basic-rock/track.mid
```

Or launch without a track and use the track browser:

```
tdrums
```

Then press `:` and type `/cassette` to browse available tracks.

### CLI Flags

```
tdrums [MIDI_PATH] [OPTIONS]

Options:
  --bpm <NUMBER>      Override track BPM
  --kit <NAME>        Select drum kit by name
  --theme <NAME>      Set color theme
  --visual-only       Run without audio (silent mode)
  --config <PATH>     Custom config file path
```

## Input Modes

Terminal Drums uses vim-style modal input.

**Normal mode** — drum keys are active. This is the default mode.

| Key        | Action              |
|------------|---------------------|
| `i`        | Enter Insert mode   |
| `:`        | Enter Insert mode with `/` pre-filled |
| `Ctrl+Q`   | Quit                |
| `Ctrl+C`   | Quit                |

**Insert mode** — type slash commands in the console.

| Key        | Action              |
|------------|---------------------|
| `Esc`      | Return to Normal    |
| `Ctrl+C`   | Return to Normal    |
| `Enter`    | Execute command     |
| `Tab`      | Accept autocomplete / cycle suggestions |
| `Shift+Tab`| Cycle suggestions backward |
| `Up` / `Down` | Navigate autocomplete |

## Drum Key Mappings

### Split Preset (default)

Designed for two-hand play across the keyboard.

```
Left hand                  Right hand
 q  w  e                    u  i  o  p
 TL TM TH                  CR1 CR2 SP CH

  a  s  d                    j  k  l
  SN XS PH                  HH OH RD

        [Space] = Kick
```

| Key   | Piece          | Abbr |
|-------|----------------|------|
| Space | Kick           | KK   |
| a     | Snare          | SN   |
| s     | Cross Stick    | XS   |
| d     | Pedal Hi-Hat   | PH   |
| j     | Closed Hi-Hat  | HH   |
| k     | Open Hi-Hat    | OH   |
| l     | Ride Cymbal    | RD   |
| u     | Crash 1        | CR1  |
| i     | Crash 2        | CR2  |
| e     | High Tom       | TH   |
| w     | Mid Tom        | TM   |
| q     | Low Tom        | TL   |
| o     | Splash         | SP   |
| p     | China          | CH   |

### Compact Preset

Home-row centered layout for smaller keyboards.

| Key   | Piece          |
|-------|----------------|
| Space | Kick           |
| f     | Snare          |
| d     | Cross Stick    |
| h     | Pedal Hi-Hat   |
| j     | Closed Hi-Hat  |
| k     | Open Hi-Hat    |
| l     | Ride Cymbal    |
| g     | Crash 1        |
| ;     | Crash 2        |
| r     | High Tom       |
| e     | Mid Tom        |
| w     | Low Tom        |
| u     | Splash         |
| i     | China          |

Set preset in config or override individual keys (see Configuration below).

## Commands

Press `:` or `i` then type a command. Commands support prefix matching and Tab autocomplete.

### Playback

| Command             | Alias | Args                       | Description                    |
|---------------------|-------|----------------------------|--------------------------------|
| `/play`             | `/p`  |                            | Start/resume playback          |
| `/pause`            |       |                            | Pause playback                 |
| `/replay`           | `/r`  |                            | Restart from beginning         |
| `/bpm <number>`     |       | BPM value                  | Override BPM                   |
| `/loop [N]`         |       | Bar count (default: 2)     | Loop next N bars from current position |

### Track & Kit

| Command             | Args        | Description                          |
|---------------------|-------------|--------------------------------------|
| `/track <name>`     | Track name  | Load track by name (fuzzy match)     |
| `/cassette`         |             | Open track browser                   |
| `/kit <name>`       | Kit name    | Select drum kit                      |
| `/channel <number>` | 1-16        | Override MIDI drum channel           |

### Difficulty & Scoring

| Command                        | Args                          | Description                   |
|--------------------------------|-------------------------------|-------------------------------|
| `/difficulty easy\|medium\|hard` | Difficulty level              | Set note filter               |
| `/timing relaxed\|standard\|strict` | Timing preset             | Set hit accuracy windows      |
| `/practice`                    |                               | Toggle practice mode          |
| `/scoreboard`                  |                               | View scores                   |

### Audio

| Command            | Description                    |
|--------------------|--------------------------------|
| `/mute`            | Toggle all audio               |
| `/mute-metronome`  | Toggle metronome               |
| `/mute-backtrack`  | Toggle backing track           |
| `/mute-kit`        | Toggle drum kit sounds         |

### Other

| Command      | Alias | Description                |
|--------------|-------|----------------------------|
| `/theme <name>` |    | Switch color theme         |
| `/calibrate` |       | Run input latency calibration |
| `/help`      |       | Show in-app command reference |
| `/quit`      | `/q`  | Exit                       |

## Configuration

Config file location: `~/.config/terminal-drums/config.toml`

Created automatically on first run with defaults. Full example:

```toml
[display]
theme = "gruvbox"
fps = 60
look_ahead_ms = 3000
show_velocity = true
show_note_tails = true

[audio]
mode = "visual_audio"
metronome_volume = 0.7
kit_volume = 1.0
backtrack_volume = 0.8
input_offset_ms = 0          # set by /calibrate

[playback]
default_bpm = 0              # 0 = use track BPM
default_kit = "acoustic"
default_difficulty = "hard"  # easy | medium | hard
default_timing = "standard"  # relaxed | standard | strict

[keys]
preset = "split"             # split | compact
# Individual overrides (uncomment to remap):
# kick = "x"
# snare = "a"
# hihat_closed = "j"

[paths]
tracks_dir = "~/.config/terminal-drums/tracks"
kits_dir = "~/.config/terminal-drums/kits"
data_dir = "~/.local/share/terminal-drums"
```

## Loading MIDI Tracks

### From command line

```
tdrums path/to/track.mid
```

### Track bundles

Place track bundles in `~/.config/terminal-drums/tracks/`. Each track is a directory:

```
~/.config/terminal-drums/tracks/
  my-song/
    track.mid           # required — MIDI file
    meta.toml           # required — track metadata
    backtrack.mp3       # optional — instrumental audio
    cover.jpg           # optional — album art
```

#### meta.toml

```toml
[track]
name = "My Song"
artist = "Artist Name"
description = "Description of the beat"
difficulty_stars = 3        # 1-5 star rating
default_bpm = 120
genre = "Rock"

[midi]
# channel = 10             # override auto-detection (1-16, default: 10)

[backtrack]
offset_ms = 0               # sync adjustment if backtrack is offset from MIDI
```

The app also scans bundled tracks in `assets/tracks/` alongside the binary.

### MIDI requirements

Standard MIDI files with drum data on channel 10 (General MIDI). Supported drum notes:

| MIDI Note | Piece          |
|-----------|----------------|
| 35, 36    | Kick           |
| 37        | Cross Stick    |
| 38, 40    | Snare          |
| 42        | Closed Hi-Hat  |
| 44        | Pedal Hi-Hat   |
| 46        | Open Hi-Hat    |
| 41, 43, 45| Low Tom        |
| 47        | Mid Tom        |
| 48, 50    | High Tom       |
| 49        | Crash 1        |
| 51        | Ride Cymbal    |
| 52        | China          |
| 53        | Ride Bell      |
| 55        | Splash         |
| 57        | Crash 2        |

## Loading Drum Kits

Place kits in `~/.config/terminal-drums/kits/`. Each kit is a directory with WAV samples and a manifest:

```
~/.config/terminal-drums/kits/
  my-kit/
    kit.toml
    kick.wav
    snare.wav
    hihat_closed.wav
    ...
```

#### kit.toml

```toml
[kit]
name = "My Kit"
author = "Your Name"
description = "Optional description"

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

Missing samples are non-fatal — that piece just won't produce sound. Select a kit with `/kit my-kit` or set `default_kit` in config.

## Calibration

Input calibration measures your system's audio latency so hit detection is accurate.

1. Run `/calibrate`
2. A metronome plays 16 beats at 100 BPM
3. Tap any drum key in sync with each beat
4. The app computes the median offset from your taps
5. Result is saved as `input_offset_ms` in your config

Press `Esc` to cancel calibration at any time.

## Difficulty

Controls which notes appear on the highway.

| Level    | Notes shown                                    |
|----------|------------------------------------------------|
| `easy`   | Downbeats only (beat 1 of each bar)            |
| `medium` | All main hits, no ghost notes (velocity < 40)  |
| `hard`   | Everything including ghost notes               |

Change with `/difficulty easy|medium|hard` or set `default_difficulty` in config.

## Timing Windows

Controls how precisely you need to hit notes.

| Preset     | Perfect | Great | Good  | OK    |
|------------|---------|-------|-------|-------|
| `relaxed`  | ±25ms   | ±50ms | ±80ms | ±120ms|
| `standard` | ±15ms   | ±30ms | ±50ms | ±80ms |
| `strict`   | ±8ms    | ±15ms | ±25ms | ±40ms |

Change with `/timing relaxed|standard|strict` or set `default_timing` in config.

## Loop Mode

Loop a section for focused practice:

1. Play to the bar you want to start from
2. Type `/loop` to loop the next 2 bars, or `/loop 4` for 4 bars
3. Playback wraps back to the loop start when it reaches the end

The header shows `[LOOP 3-4]` when a loop is active. Run `/loop` again to disable.

## Practice Mode

Adaptive speed training. Requires an active loop.

1. Set a loop with `/loop`
2. Type `/practice` to enable
3. Playback starts at 50% speed
4. Speed adjusts automatically each loop iteration:
   - Score >= 90%: speed increases by 5%
   - Score < 70%: speed decreases by 5%
   - 70-90%: no change
5. Caps at 100% of original BPM, floor at 30%

The header shows `[PRACTICE 65%]` with current speed.

## Scoring

Every note hit is graded:

| Grade   | Points | Window         |
|---------|--------|----------------|
| Perfect | 100    | Within timing  |
| Great   | 80     | 1.5x window    |
| Good    | 50     | 2.5x window    |
| OK      | 20     | 3x window      |
| Miss    | 0      | Outside window |

Score percentage = total points earned / (total notes x 100).

A combo counter tracks consecutive hits — resets on miss or wrong piece.

Scores are tracked over four windows: full track, last 8 bars, last 16 bars, and last 32 bars. View with `/scoreboard`.

### Persistence

Scores are saved to SQLite at `~/.local/share/terminal-drums/scores.db`. The scoreboard shows personal bests per track, difficulty, and timing preset.

## Themes

Nine built-in themes. Switch with `/theme <name>`:

- `gruvbox` (default) — warm retro
- `desert` — sandy, warm
- `evening` — cool twilight
- `slate` — dark, clean
- `blue` — cool blue
- `pablo` — vibrant
- `quiet` — muted, minimal
- `shine` — bright, high-contrast
- `run` — dynamic

## Muting

Four independent audio channels, each toggled separately:

| Command           | What it mutes              |
|-------------------|----------------------------|
| `/mute`           | All audio at once          |
| `/mute-metronome` | Metronome clicks only      |
| `/mute-backtrack` | Backing instrumental track |
| `/mute-kit`       | Drum kit sample playback   |

A mute icon appears in the header when any channel is muted.

## Visual-Only Mode

Run without initializing audio hardware:

```
tdrums track.mid --visual-only
```

The highway, scoring, and all visual feedback work normally — just no sound. Useful for silent practice or testing.

## File Locations

| Path | Contents |
|------|----------|
| `~/.config/terminal-drums/config.toml` | Configuration |
| `~/.config/terminal-drums/tracks/` | User track bundles |
| `~/.config/terminal-drums/kits/` | User drum kits |
| `~/.local/share/terminal-drums/scores.db` | Score database |

## License

MIT
