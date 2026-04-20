use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::GameState;
use crate::ui::themes::Theme;

/// Render the calibration overlay, centered on screen.
pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    // Compute a centered popup area (42 wide, 18 tall)
    let popup_width = 44u16;
    let popup_height = 18u16;

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let accent_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let fg_style = Style::default().fg(theme.fg);
    let dim_style = Style::default().fg(theme.fg_dim);
    let ok_style = Style::default()
        .fg(theme.perfect)
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        "       CALIBRATE INPUT",
        accent_style,
    )));
    lines.push(Line::from(Span::raw("")));

    // Instruction or result
    if let Some(offset_ms) = state.calibration_result {
        // Show saved result
        let sign = if offset_ms >= 0.0 { "+" } else { "" };
        lines.push(Line::from(Span::styled(
            format!("   Offset: {}{:.0}ms  Saved!", sign, offset_ms),
            ok_style,
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  TAP ANY KEY IN SYNC WITH THE BEAT",
            fg_style,
        )));
    }
    lines.push(Line::from(Span::raw("")));

    // Beat indicator boxes (4 boxes showing beat 1-4)
    let beat_in_bar = state.calibration_beat % 4;
    let beats_done = state.calibration_beat;
    let mut beat_spans: Vec<Span> = Vec::new();
    beat_spans.push(Span::raw("   "));
    for b in 0u32..4 {
        let is_active = beat_in_bar == b && beats_done < state.calibration_total_beats;
        let label = format!(" {} ", b + 1);
        if is_active {
            beat_spans.push(Span::styled(
                format!("\u{2554}{}\u{2557}", "\u{2550}".repeat(label.len())),
                accent_style,
            ));
            beat_spans.push(Span::raw("  "));
        } else {
            beat_spans.push(Span::styled(
                format!("\u{250c}{}\u{2510}", "\u{2500}".repeat(label.len())),
                dim_style,
            ));
            beat_spans.push(Span::raw("  "));
        }
    }
    lines.push(Line::from(beat_spans));

    // Beat number row
    let mut num_spans: Vec<Span> = Vec::new();
    num_spans.push(Span::raw("   "));
    for b in 0u32..4 {
        let is_active = beat_in_bar == b && beats_done < state.calibration_total_beats;
        let label = format!(" {} ", b + 1);
        if is_active {
            num_spans.push(Span::styled(
                format!("\u{2551}{}\u{2551}", label),
                accent_style,
            ));
            num_spans.push(Span::raw("  "));
        } else {
            num_spans.push(Span::styled(
                format!("\u{2502}{}\u{2502}", label),
                dim_style,
            ));
            num_spans.push(Span::raw("  "));
        }
    }
    lines.push(Line::from(num_spans));

    // Bottom of boxes
    let mut bot_spans: Vec<Span> = Vec::new();
    bot_spans.push(Span::raw("   "));
    for b in 0u32..4 {
        let is_active = beat_in_bar == b && beats_done < state.calibration_total_beats;
        let label = format!(" {} ", b + 1);
        if is_active {
            bot_spans.push(Span::styled(
                format!("\u{255a}{}\u{255d}", "\u{2550}".repeat(label.len())),
                accent_style,
            ));
            bot_spans.push(Span::raw("  "));
        } else {
            bot_spans.push(Span::styled(
                format!("\u{2514}{}\u{2518}", "\u{2500}".repeat(label.len())),
                dim_style,
            ));
            bot_spans.push(Span::raw("  "));
        }
    }
    lines.push(Line::from(bot_spans));

    lines.push(Line::from(Span::raw("")));

    // Tap counter and BPM
    lines.push(Line::from(vec![
        Span::styled("   Tap: ", dim_style),
        Span::styled(
            format!("{}/{}", state.calibration_beat, state.calibration_total_beats),
            fg_style,
        ),
        Span::styled("          BPM: ", dim_style),
        Span::styled("100", fg_style),
    ]));

    lines.push(Line::from(Span::raw("")));

    // Phase progress bar (20 chars wide)
    let bar_width = 20usize;
    let filled = (state.calibration_phase * bar_width as f64).round() as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;
    let bar = format!(
        "   [{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty)
    );
    lines.push(Line::from(Span::styled(bar, accent_style)));

    lines.push(Line::from(Span::raw("")));

    // Cancel hint
    lines.push(Line::from(Span::styled(
        "   Press ESC to cancel",
        dim_style,
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg));

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(para, popup_area);
}

/// Compute a centered rect of the given width and height within `r`.
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    let x = r.x + (r.width.saturating_sub(w)) / 2;
    let y = r.y + (r.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

