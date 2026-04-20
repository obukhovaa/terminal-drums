use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{GameState, InputMode};
use crate::ui::themes::Theme;

/// Command console widget with vim mode indicator, input line, and autocomplete popup.
pub struct ConsoleWidget;

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // Check for status message to show (auto-fades after 3 seconds)
    // Skip if user is typing — input takes priority
    if state.input_mode != InputMode::Insert {
        if let Some((ref msg, shown_at)) = state.status_message {
            if shown_at.elapsed().as_secs() < 3 {
                render_status_message(frame, area, msg, theme);
                render_status_indicators(frame, area, state, theme);
                return;
            }
        }
    }

    match state.input_mode {
        InputMode::Normal => {
            // Show: diamond NORMAL diamond  -- i or : to enter commands --
            let line = Line::from(vec![
                Span::styled(
                    " \u{25C6} NORMAL ",
                    Style::default()
                        .fg(theme.header_fg)
                        .bg(theme.status_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "  -- i or : to enter commands --",
                    Style::default().fg(theme.fg_dim).bg(theme.status_bg),
                ),
            ]);
            let para = Paragraph::new(line).style(Style::default().bg(theme.status_bg));
            frame.render_widget(para, area);
        }
        InputMode::Insert => {
            render_insert_mode(frame, area, state, theme);
        }
    }

    // Right-aligned status indicators on the console line
    render_status_indicators(frame, area, state, theme);
}

/// Render right-aligned mute/metronome indicators on the status bar.
fn render_status_indicators(
    frame: &mut Frame,
    area: Rect,
    state: &GameState,
    theme: &Theme,
) {
    let mut indicators: Vec<(String, Style)> = Vec::new();

    if state.mute_all {
        indicators.push((
            "\u{1F507} MUTE".into(),
            Style::default().fg(ratatui::style::Color::Yellow).bg(theme.status_bg),
        ));
    } else if state.mute_kit {
        indicators.push((
            "\u{1F507} KIT".into(),
            Style::default().fg(ratatui::style::Color::Yellow).bg(theme.status_bg),
        ));
    }

    if !state.mute_metronome && !state.mute_all {
        indicators.push((
            "MET:ON".into(),
            Style::default().fg(theme.fg_dim).bg(theme.status_bg),
        ));
    } else {
        indicators.push((
            "MET:OFF".into(),
            Style::default().fg(ratatui::style::Color::Yellow).bg(theme.status_bg),
        ));
    }

    if indicators.is_empty() {
        return;
    }

    // Build the right-aligned string
    let text: String = indicators
        .iter()
        .map(|(t, _)| t.as_str())
        .collect::<Vec<_>>()
        .join("  ");
    let text_len = text.chars().count() as u16 + 1; // +1 trailing space
    let row_y = area.y + area.height.saturating_sub(1);
    let start_x = area.x + area.width.saturating_sub(text_len + 1);

    let buf = frame.buffer_mut();
    let mut cx = start_x;
    let mut ind_idx = 0;
    let mut char_in_ind = 0;

    for ch in text.chars() {
        // Find which indicator this character belongs to
        while ind_idx < indicators.len() {
            let ind_text = &indicators[ind_idx].0;
            if char_in_ind < ind_text.chars().count() {
                break;
            }
            char_in_ind = 0;
            ind_idx += 1;
            // Skip separator chars
        }
        let style = if ind_idx < indicators.len() {
            indicators[ind_idx].1
        } else {
            Style::default().fg(theme.fg_dim).bg(theme.status_bg)
        };

        if cx < area.x + area.width {
            if let Some(cell) = buf.cell_mut((cx, row_y)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
        }
        cx = cx.saturating_add(1);
        char_in_ind += 1;
    }
}

fn render_insert_mode(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    let input = &state.console_input;
    let cursor = state.console_cursor;

    // Build the prompt + input + cursor line
    let before_cursor = &input[..cursor.min(input.len())];
    let after_cursor = if cursor < input.len() {
        &input[cursor..]
    } else {
        ""
    };

    let text_style = Style::default().fg(theme.console_fg).bg(theme.console_bg);
    let cursor_style = Style::default()
        .fg(theme.console_bg)
        .bg(theme.console_fg)
        .add_modifier(Modifier::BOLD);

    // Cursor character
    let cursor_char = if after_cursor.is_empty() {
        " ".to_string()
    } else {
        after_cursor.chars().next().unwrap_or(' ').to_string()
    };
    let after_cursor_rest = if after_cursor.len() > 1 {
        &after_cursor[cursor_char.len()..]
    } else {
        ""
    };

    // Mode indicator prefix: pencil INSERT pencil  > /text
    let mode_prefix = Span::styled(
        " \u{270E} INSERT ",
        Style::default()
            .fg(theme.accent)
            .bg(theme.console_bg)
            .add_modifier(Modifier::BOLD),
    );

    let prompt = Span::styled(
        "> ",
        Style::default()
            .fg(theme.console_prompt)
            .bg(theme.console_bg)
            .add_modifier(Modifier::BOLD),
    );

    let mut input_spans = vec![
        mode_prefix,
        prompt,
        Span::styled(before_cursor.to_string(), text_style),
        Span::styled(cursor_char, cursor_style),
        Span::styled(after_cursor_rest.to_string(), text_style),
    ];

    // Show placeholder hint after cursor when:
    // - input ends with a space (command typed, waiting for arg), OR
    // - placeholder is set by autocomplete or typed command detection
    if after_cursor.is_empty() && !state.console_placeholder.is_empty() {
        let placeholder_style = Style::default().fg(theme.status_fg).bg(theme.console_bg);
        input_spans.push(Span::styled(
            state.console_placeholder.clone(),
            placeholder_style,
        ));
    }

    let input_line = Line::from(input_spans);

    // Input line always renders in the given area (1 line)
    let input_area = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };
    let para = Paragraph::new(input_line).style(Style::default().bg(theme.console_bg));
    frame.render_widget(para, input_area);

    // Autocomplete popup floats above the input line
    if !state.autocomplete_suggestions.is_empty() && input_area.y > 0 {
        let suggestions = &state.autocomplete_suggestions;
        let selected = state.autocomplete_selected;

        let mut spans: Vec<Span> = vec![Span::styled(
            "  ",
            Style::default().fg(theme.autocomplete_fg),
        )];
        for (i, suggestion) in suggestions.iter().enumerate() {
            let style = if selected == Some(i) {
                Style::default()
                    .fg(theme.autocomplete_fg)
                    .bg(theme.autocomplete_selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.autocomplete_fg)
                    .bg(theme.autocomplete_bg)
            };
            spans.push(Span::styled(format!(" {} ", suggestion), style));
            spans.push(Span::styled(
                "  ",
                Style::default().fg(theme.autocomplete_fg),
            ));
        }

        if state.autocomplete_total > suggestions.len() {
            let more = state.autocomplete_total - suggestions.len();
            spans.push(Span::styled(
                format!("...+{}", more),
                Style::default().fg(theme.fg_dim).bg(theme.autocomplete_bg),
            ));
        }

        let autocomplete_line = Line::from(spans);
        let ac_area = Rect {
            y: input_area.y - 1,
            height: 1,
            ..area
        };
        let ac_para =
            Paragraph::new(autocomplete_line).style(Style::default().bg(theme.autocomplete_bg));
        frame.render_widget(ac_para, ac_area);
    }
}

/// Render a status message line (e.g. error feedback).
fn render_status_message(frame: &mut Frame, area: Rect, msg: &str, theme: &Theme) {
    let style = if msg.starts_with("Error:") {
        Style::default().fg(theme.miss).bg(theme.status_bg)
    } else {
        Style::default()
            .fg(theme.console_prompt)
            .bg(theme.status_bg)
    };

    let line = Line::from(Span::styled(format!(" {} ", msg), style));
    let msg_area = Rect {
        y: area.y,
        height: 1,
        ..area
    };
    let para = Paragraph::new(line).style(Style::default().bg(theme.status_bg));
    frame.render_widget(para, msg_area);
}
