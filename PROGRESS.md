# Terminal Drums - Progress

> **Spec:** [spec/init-project/SPEC.md](./spec/init-project/SPEC.md)

## Phase 1: Foundation (Core Loop)

- [x] Project scaffold (Cargo.toml, module structure, error types)
- [x] MIDI parser: load file, extract drum notes, time signature, tempo
- [x] Drum map: GM MIDI note → DrumPiece mapping
- [x] Basic TUI layout with ratatui (header, highway area, status bar)
- [x] Note highway widget: render scrolling notes with velocity glyphs
- [x] Input thread: poll-based key reading with timestamps + shutdown flag
- [x] Vim mode state machine (NORMAL/INSERT)
- [x] Key → DrumPiece mapping (split-hand preset)
- [x] Game loop: advance playback position via PlaybackEngine
- [x] Hit detection: same-piece-first algorithm (SPEC §6.1)
- [x] Visual feedback: color flash on hit/miss/wrong piece/extra

## Phase 2: Audio & Scoring

- [x] Kira AudioEngine setup with 3 sub-tracks
- [x] Drum kit sample loading from directory
- [x] Trigger drum samples on key press (velocity-scaled volume)
- [x] Metronome: visual pendulum animation
- [x] Metronome: audio clicks via kira Clock scheduling
- [x] Scoring engine: ScoreAccumulator + RollingScoreWindow
- [x] SQLite database setup with schema (SPEC §12.1)
- [x] Score persistence on track end
- [x] Scoreboard widget
- [x] Combo system

## Phase 3: Command Console & UX Polish

- [x] Command console widget with autocomplete
- [x] Slash command parser with registry
- [x] All slash commands implemented
- [x] Theme system with 9 bundled themes
- [x] Cassette track browser overlay
- [x] Track bundle discovery
- [x] First-startup flow (welcome screen, profile creation)
- [x] Config file persistence
- [x] BPM/time signature override
- [x] Loop mode with grace-zone handling
- [x] Practice mode with auto-BPM

## Phase 4: Backtrack & Kit System

- [ ] Backtrack loading and streaming
- [ ] Backtrack sync + drift correction
- [ ] Multiple kit support
- [x] Input calibration wizard
- [x] Volume/mute controls
- [x] Demo track bundles

## Phase 5: Network & Social (Future)

- [ ] GitHub OAuth device flow
- [ ] Global leaderboard
- [ ] Track catalog + downloads
