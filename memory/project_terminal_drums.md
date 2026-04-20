---
name: Terminal Drums Project
description: TUI drum trainer app for programmers - Rust/ratatui/kira/midly stack with vim keybindings, 80s cassette aesthetic
type: project
---

Terminal Drums is a TUI-based drum training application targeting programmers who work in terminal environments (zsh/tmux/iTerm2 on macOS).

**Core concept:** Visualize MIDI drum tracks as a scrolling note highway, user presses vim-style keys to hit drums at the right time, app scores accuracy.

**Tech stack decided:** Rust with ratatui (TUI), kira (audio), midly (MIDI parsing), crossterm (input), spin_sleep (game loop timing).

**Why:** User wants to practice drums during terminal downtime (waiting for builds/deploys). 80s cassette aesthetic with vim color themes.

**Key files:**
- `spec/init-project/SPEC.md` — Single source of truth for all implementation details
- `PROGRESS.md` — Phase tracking with checkboxes, updated as work completes

**How to apply:** All development decisions should prioritize low latency (sub-millisecond timing matters for rhythm game accuracy), terminal compatibility (tmux/iTerm2), and the vim-centric UX paradigm (NORMAL mode for playing, INSERT mode for commands). Read PROGRESS.md at session start to know where we left off. Read SPEC.md for implementation guidance.
