use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::GameState;
use crate::engine::scoring::ScoreAccumulator;
use crate::ui::themes::Theme;

/// Full-screen scoreboard overlay showing session results and personal bests.
pub struct ScoreboardWidget;

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    // Overlay: center a box in the terminal
    let box_w = area.width.min(60);
    let box_h = area.height.min(22);
    let x = area.x + area.width.saturating_sub(box_w) / 2;
    let y = area.y + area.height.saturating_sub(box_h) / 2;
    let overlay_area = Rect {
        x,
        y,
        width: box_w,
        height: box_h,
    };

    // Clear behind the overlay
    frame.render_widget(Clear, overlay_area);

    let track_name = state.track_name.as_deref().unwrap_or("--");
    let bpm = state.effective_bpm;

    let title_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.fg_dim);
    let value_style = Style::default().fg(theme.score_fg).add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme.fg_dim);
    let hint_style = Style::default().fg(theme.fg_dim);

    let lines = vec![
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            format!("  \u{2605} SCOREBOARD \u{2605}"),
            title_style,
        )),
        Line::from(Span::styled("", Style::default())),
        Line::from(vec![
            Span::styled("  Track: ", label_style),
            Span::styled(track_name, value_style),
            Span::styled("   BPM: ", label_style),
            Span::styled(format!("{:.0}", bpm), value_style),
        ]),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            "  THIS SESSION",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        score_line("  Full Track: ", &state.score_full, value_style, dim_style),
        score_line("  Last 8 bars: ", &state.score_8bar, value_style, dim_style),
        score_line("  Last 16 bars: ", &state.score_16bar, value_style, dim_style),
        score_line("  Last 32 bars: ", &state.score_32bar, value_style, dim_style),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            "  PERSONAL BEST",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        match state.personal_best {
            Some(best) => Line::from(vec![
                Span::styled("  Best: ", dim_style),
                Span::styled(
                    format!("{:.1}%", best),
                    value_style,
                ),
            ]),
            None => Line::from(Span::styled(
                "  (No records yet)",
                dim_style,
            )),
        },
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            "  Press ESC to continue, R to replay",
            hint_style,
        )),
        Line::from(Span::styled("", Style::default())),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .border_type(ratatui::widgets::BorderType::Double)
        .style(Style::default().bg(theme.bg));

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(para, overlay_area);
}

fn score_line<'a>(
    label: &'a str,
    acc: &ScoreAccumulator,
    value_style: Style,
    dim_style: Style,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, dim_style),
        Span::styled(format!("{:.1}%", acc.percentage()), value_style),
        Span::styled(
            format!(
                "  (P:{} G:{} OK:{} M:{})  x{}",
                acc.perfect_count,
                acc.great_count,
                acc.good_count + acc.ok_count,
                acc.miss_count,
                acc.max_combo
            ),
            dim_style,
        ),
    ])
}
