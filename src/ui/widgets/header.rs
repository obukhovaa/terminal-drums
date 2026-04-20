use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{AppState, GameState, SessionState};
use crate::ui::themes::Theme;

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    let track_name = state.track_name.as_deref().unwrap_or("--");
    let bpm = state.effective_bpm;
    let (ts_num, ts_denom) = state.time_sig;

    let accent = theme.accent;
    let dim = theme.fg_dim;
    let fg = theme.header_fg;
    let header_bg = theme.header_bg;

    let base = Style::default().fg(fg).bg(header_bg);
    let sep_style = Style::default().fg(dim).bg(header_bg);

    // Build all header segments as (text, style) pairs
    let mut segments: Vec<(&str, String, Style)> = Vec::new();

    // Logo
    segments.push((
        "logo",
        " TERMINAL DRUMS".into(),
        Style::default().fg(accent).bg(header_bg).add_modifier(Modifier::BOLD),
    ));

    // Playback state
    let (playback_str, pb_color) = match &state.app_state {
        AppState::Session(session_state) => match session_state {
            SessionState::Playing => ("\u{25B6} PLAYING", Color::Green),
            SessionState::Paused => ("\u{23F8} PAUSED", Color::Yellow),
            SessionState::Ready => ("\u{23F9} STOPPED", dim),
        },
        AppState::Calibrating => ("CALIBRATING", dim),
        _ => ("\u{23F9} STOPPED", dim),
    };
    segments.push((
        "playback",
        playback_str.into(),
        Style::default().fg(pb_color).bg(header_bg).add_modifier(Modifier::BOLD),
    ));

    // Track name
    let practice_str = if state.practice_mode {
        format!(
            " [PRACTICE {:.0}%]",
            (state.practice_bpm / state.practice_target_bpm.max(1.0)) * 100.0
        )
    } else {
        String::new()
    };
    segments.push(("track", format!("{}{}", track_name, practice_str), base));

    // BPM + time sig
    segments.push(("bpm", format!("{:.0} BPM {}/{}", bpm, ts_num, ts_denom), base));

    // Bar position
    segments.push((
        "bar",
        format!("Bar {}/{}", state.current_bar + 1, state.total_bars.max(1)),
        base,
    ));

    // Loop indicator (only if active)
    if state.loop_active {
        segments.push((
            "loop",
            format!("LOOP {}-{}", state.loop_start_bar + 1, state.loop_end_bar + 1),
            Style::default().fg(accent).bg(header_bg),
        ));
    }

    // Score + combo
    let score = &state.score_full;
    segments.push((
        "score",
        format!("{:.1}%", score.percentage()),
        Style::default().fg(theme.score_fg).bg(header_bg).add_modifier(Modifier::BOLD),
    ));
    segments.push((
        "combo",
        format!("{}x", score.current_combo),
        Style::default().fg(theme.combo_fg).bg(header_bg).add_modifier(Modifier::BOLD),
    ));

    // Autoplay indicator
    if state.autoplay {
        segments.push((
            "autoplay",
            "AUTOPLAY ON".into(),
            Style::default().fg(Color::Cyan).bg(header_bg).add_modifier(Modifier::BOLD),
        ));
    }

    // Mute indicator
    if state.mute_all || state.mute_metronome || state.mute_backtrack || state.mute_kit {
        segments.push((
            "mute",
            "\u{1F507}".into(),
            Style::default().fg(Color::Yellow).bg(header_bg),
        ));
    }

    // Convert segments to spans with separators and split into lines
    let sep_str = " \u{2502} ";
    let sep_width = 3; // " │ "
    let width = area.width as usize;

    let mut lines: Vec<Line> = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut current_width: usize = 0;

    for (_tag, text, style) in segments.iter() {
        let seg_width = text.chars().count();
        let need = if current_spans.is_empty() {
            seg_width
        } else {
            sep_width + seg_width
        };

        if !current_spans.is_empty() && current_width + need > width {
            // Wrap: finish current line and start a new one
            lines.push(Line::from(current_spans));
            current_spans = Vec::new();
            current_width = 0;
        }

        if !current_spans.is_empty() {
            current_spans.push(Span::styled(sep_str, sep_style));
            current_width += sep_width;
        }

        current_spans.push(Span::styled(text.clone(), *style));
        current_width += seg_width;
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    let para = Paragraph::new(lines)
        .style(Style::default().bg(header_bg).fg(fg));

    frame.render_widget(para, area);
}

/// Compute how many lines the header needs at the given width.
pub fn header_height(state: &GameState, width: u16) -> u16 {
    let w = width as usize;
    let sep_width = 3;

    // Approximate segment widths without allocating full strings
    let track_name = state.track_name.as_deref().unwrap_or("--");
    let practice_extra = if state.practice_mode { 16 } else { 0 };

    let widths: Vec<usize> = vec![
        15, // " TERMINAL DRUMS"
        9,  // "▶ PLAYING" (longest playback state)
        track_name.len() + practice_extra,
        12, // "120 BPM 4/4"
        10, // "Bar 99/99"
        if state.loop_active { 11 } else { 0 }, // "LOOP 1-4"
        5,  // "99.9%"
        4,  // "99x"
        if state.autoplay { 11 } else { 0 }, // "AUTOPLAY ON"
        if state.mute_all || state.mute_metronome || state.mute_backtrack || state.mute_kit { 2 } else { 0 },
    ];

    let mut lines = 1u16;
    let mut current_w = 0usize;
    for seg_w in widths {
        if seg_w == 0 {
            continue;
        }
        let need = if current_w == 0 { seg_w } else { sep_width + seg_w };
        if current_w > 0 && current_w + need > w {
            lines += 1;
            current_w = seg_w;
        } else {
            current_w += need;
        }
    }

    lines
}
