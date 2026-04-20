# AGENTS.md — Terminal Drums

TUI drum trainer for programmers. Rust + ratatui + kira + midly.

## Session Start

At the start of each session, read `PROGRESS.md` to understand current progress. When completing a task, update the checkbox (`- [ ]` → `- [x]`).

## Build & Test
- `cargo run -- assets/tracks/basic-rock/track.mid --visual-only` for quick visual testing without audio init
- Binary target is `tdrums` (not `terminal-drums`); defined in `[[bin]]` section of Cargo.toml
- Rust toolchain 1.95+ required — transitive deps (coreaudio-sys, clap_lex) need edition2024
- `tests/gen_samples.rs` generates all WAV/MIDI assets; run `cargo test --test gen_samples` to regenerate

## Architecture
- Single-threaded 60fps loop in Phase 1 (not the spec's 4-thread design). SwapBuffer exists but isn't used yet — game state is mutated directly.
- `src/app.rs` is the god module: GameState, SessionContext, run loop, all input handlers, command dispatch. When modifying behavior, this is almost always the file.
- `src/lib.rs` exists solely for integration tests to import `terminal_drums::*`; the binary uses `mod` declarations in `main.rs`.

## Gotchas
- Worktree agents share the git object store. Uncommitted changes in one worktree are visible from all others. Agents don't always commit — check for dirty files with `git status` from the worktree path.
- The vim `:` key pre-fills `/` in console_input. Users typing `/play` after `:` produce `//play`. Parser uses `trim_start_matches('/')` not `strip_prefix` to handle this.
- `InputMode` is defined in `src/input/vim_mode.rs` but re-exported from `src/app.rs` via `pub use`. UI modules import it from `crate::app::InputMode`.
- `kira` v0.9 API differs significantly from docs/examples online. When troubleshooting audio, read actual source in `~/.cargo/registry/src/` — method signatures for `AudioManager`, `TrackBuilder`, `StaticSoundData` are not what you'd expect.

## Files That Must Change Together
- Adding a new widget: `src/ui/widgets/<name>.rs` + `src/ui/widgets/mod.rs` + `src/ui/mod.rs` (render dispatch)
- Adding a new slash command: `src/input/command.rs` (registry) + `src/app.rs` (execute_command match arm)
- Adding GameState fields: `src/app.rs` (struct + Default impl) + any widget that reads the field
- Adding a new AppState variant: `src/app.rs` (enum + handle_input routing) + `src/ui/mod.rs` (render routing)

## Implementation Rules
- Follow SPEC.md for all data structures, algorithms, and architecture decisions
- Prioritize low latency — this is a rhythm game where sub-millisecond timing matters
- Terminal compatibility: must work in tmux + iTerm2 on macOS
- Lock-free SwapBuffer for game→render state sharing (no Mutex)
- Input thread uses poll(10ms) + AtomicBool shutdown flag (not blocking read)
- Profile name stored in SQLite only, not in config.toml
- Quit via Ctrl+Q/Ctrl+C, not bare `q` (mapped to drum)
