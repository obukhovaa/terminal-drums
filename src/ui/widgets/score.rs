use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::GameState;
use crate::ui::themes::Theme;

/// Color for a score milestone level.
fn milestone_color(milestone: u8) -> Color {
    match milestone {
        80..=100 => Color::Green,
        60..=79 => Color::Cyan,
        40..=59 => Color::Yellow,
        20..=39 => Color::Magenta,
        _ => Color::White,
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    if area.height < 4 || area.width < 20 {
        return;
    }

    let score = &state.score_full;
    let pct = score.percentage();

    // Milestone flash: active for 1.5 seconds, blinks every 200ms
    let milestone_active = state
        .score_milestone_time
        .map(|t| t.elapsed().as_millis() < 1500)
        .unwrap_or(false);
    let milestone_visible = milestone_active
        && state
            .score_milestone_time
            .map(|t| (t.elapsed().as_millis() / 200) % 2 == 0)
            .unwrap_or(false);
    let m_color = milestone_color(state.score_milestone);

    // Progress bar
    let progress = if state.track_duration_ms > 0.0 {
        (state.position_ms / state.track_duration_ms).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let inner_width = area.width.saturating_sub(2) as usize; // subtract borders

    // Build progress bar with optional loop region markers
    let progress_line = {
        let mut bar_chars: Vec<(char, ratatui::style::Color)> = Vec::with_capacity(inner_width);
        let filled = (progress * inner_width as f64) as usize;

        // Compute loop region positions in the progress bar
        let loop_start_frac = if state.loop_active && state.track_duration_ms > 0.0 {
            // Approximate loop start from bar fraction
            let total = state.total_bars.max(1) as f64;
            state.loop_start_bar as f64 / total
        } else {
            -1.0
        };
        let loop_end_frac = if state.loop_active && state.track_duration_ms > 0.0 {
            let total = state.total_bars.max(1) as f64;
            state.loop_end_bar as f64 / total
        } else {
            -1.0
        };

        for i in 0..inner_width {
            let frac = i as f64 / inner_width as f64;
            let in_loop = state.loop_active
                && frac >= loop_start_frac
                && frac < loop_end_frac;

            if i < filled {
                if in_loop {
                    bar_chars.push(('\u{2588}', theme.accent)); // █ in accent
                } else {
                    bar_chars.push(('\u{2588}', theme.progress_bar_fg)); // █
                }
            } else if in_loop {
                bar_chars.push(('\u{2592}', theme.accent)); // ▒ loop region
            } else {
                bar_chars.push(('\u{2591}', theme.progress_bar_bg)); // ░
            }
        }

        let mut spans: Vec<Span> = vec![Span::raw(" ")];
        for (ch, color) in &bar_chars {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(*color),
            ));
        }
        let loop_label = if state.loop_active {
            format!(" {:.0}% [L]", progress * 100.0)
        } else {
            format!(" {:.0}%", progress * 100.0)
        };
        spans.push(Span::styled(loop_label, Style::default().fg(theme.fg_dim)));
        Line::from(spans)
    };

    let score_fg = if milestone_visible { m_color } else { theme.score_fg };

    let best_str = match state.personal_best {
        Some(best) => format!("{:.1}%", best),
        None => "--".into(),
    };

    let line1 = Line::from(vec![
        Span::styled(
            format!(" {:.1}%", pct),
            Style::default()
                .fg(score_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   Combo: ", Style::default().fg(theme.fg_dim)),
        Span::styled(
            format!("{}x", score.current_combo),
            Style::default().fg(theme.combo_fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   \u{2605} Best: ", Style::default().fg(theme.fg_dim)),
        Span::styled(best_str, Style::default().fg(theme.score_fg)),
    ]);

    let line2 = Line::from(vec![Span::styled(
        format!(
            " P:{} G:{} OK:{} M:{} W:{}",
            score.perfect_count,
            score.great_count,
            score.good_count + score.ok_count,
            score.miss_count,
            score.wrong_piece_count,
        ),
        Style::default().fg(theme.fg_dim),
    )]);

    let line3 = progress_line;

    let border_fg = if milestone_visible { m_color } else { theme.border };
    let title_str = if milestone_active {
        format!(" Score \u{2605} {}% ", state.score_milestone)
    } else {
        " Score ".into()
    };
    let title_fg = if milestone_visible { m_color } else { theme.accent };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_fg))
        .title(Span::styled(
            title_str,
            Style::default().fg(title_fg).add_modifier(Modifier::BOLD),
        ));

    let para = Paragraph::new(vec![line1, line2, line3]).block(block);
    frame.render_widget(para, area);
}
