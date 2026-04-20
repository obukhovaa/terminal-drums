# AGENTS.md — UI Module

- Widgets are functions `fn render(frame, area, state, theme)`, not ratatui `Widget` trait impls. This was a deliberate choice for simplicity.
- `hit_feedback.rs` still exists on disk but is unreferenced — feedback rendering was merged into `highway.rs` to share lane position data. Safe to delete the file.
- Highway note Y-position math: `y_frac = time_until_ms / LOOK_AHEAD_MS` where 0 = at hit zone (bottom), 1 = far future (top). The inversion `highway_height - note_row` maps this to screen coordinates.
- Notes within 80ms of hit zone render as inverted diamond `◆` (bg=piece_color, fg=bg). Notes within 200ms get bold+highlight bg. This is in `highway.rs`, not a separate widget.
- The info bar (metronome + score) only appears when terminal height >= 30 rows. Below that, it's omitted entirely (see `layout.rs`).
- Console widget handles 3 distinct concerns: vim mode indicator (NORMAL/INSERT), command input with autocomplete, and status messages. These share the same 1-2 row area.
