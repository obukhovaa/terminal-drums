use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::GameState;
use crate::ui::themes::Theme;

/// Welcome screen shown on first launch for profile name entry.
pub struct WelcomeWidget;

/// ASCII art logo for "TDRUMS"
static LOGO: &[&str] = &[
    " \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2557}   \u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2588}\u{2557}   \u{2588}\u{2588}\u{2588}\u{2557}",
    " \u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{255D}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2551}",
    "    \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255D}\u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2554}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2554}\u{2588}\u{2588}\u{2551}",
    "    \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}\u{255A}\u{2588}\u{2588}\u{2554}\u{255D}\u{2588}\u{2588}\u{2551}",
    "    \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255D}\u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{255A}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255D}\u{2588}\u{2588}\u{2551} \u{255A}\u{2550}\u{255D} \u{2588}\u{2588}\u{2551}",
    "    \u{255A}\u{2550}\u{255D}   \u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D} \u{255A}\u{2550}\u{255D}  \u{255A}\u{2550}\u{255D} \u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D} \u{255A}\u{2550}\u{255D}     \u{255A}\u{2550}\u{255D}",
];

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    let accent_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let fg_style = Style::default().fg(theme.fg);
    let dim_style = Style::default().fg(theme.fg_dim);
    let prompt_style = Style::default()
        .fg(theme.console_prompt)
        .add_modifier(Modifier::BOLD);
    let cursor_style = Style::default()
        .fg(theme.bg)
        .bg(theme.fg)
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled("", Style::default())));

    // Logo
    for logo_line in LOGO {
        lines.push(Line::from(Span::styled(*logo_line, accent_style)));
    }

    lines.push(Line::from(vec![
        Span::styled("              TERMINAL  DRUMS  ", dim_style),
        Span::styled("v0.1", Style::default().fg(theme.fg_dim)),
    ]));

    lines.push(Line::from(Span::styled("", Style::default())));
    lines.push(Line::from(Span::styled(
        "     What's your name, drummer?",
        fg_style,
    )));
    lines.push(Line::from(Span::styled("", Style::default())));

    // Name input line — uses welcome_name (not console_input)
    let name_input = &state.welcome_name;
    let _cursor = name_input.len(); // Cursor is always at the end for welcome input
    let before = name_input.as_str();
    let cursor_char = " ".to_string();
    let after_rest = "";

    lines.push(Line::from(vec![
        Span::styled("     ", Style::default()),
        Span::styled("> ", prompt_style),
        Span::styled(before.to_string(), fg_style),
        Span::styled(cursor_char, cursor_style),
        Span::styled(after_rest.to_string(), fg_style),
    ]));

    lines.push(Line::from(Span::styled("", Style::default())));
    lines.push(Line::from(Span::styled(
        "     Press ENTER to continue",
        dim_style,
    )));
    lines.push(Line::from(Span::styled("", Style::default())));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .border_type(ratatui::widgets::BorderType::Double)
        .style(Style::default().bg(theme.bg));

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(para, area);
}
