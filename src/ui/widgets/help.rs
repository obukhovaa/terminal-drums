use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::GameState;
use crate::ui::themes::Theme;

/// Render a full-screen help overlay listing all commands.
pub fn render(frame: &mut Frame, area: Rect, _state: &GameState, theme: &Theme) {
    if area.height < 6 || area.width < 30 {
        return;
    }

    // Center overlay, leaving a margin
    let margin_h = if area.width > 80 {
        (area.width - 72) / 2
    } else {
        2
    };
    let margin_v = if area.height > 30 { 3 } else { 1 };

    let overlay = Rect {
        x: area.x + margin_h,
        y: area.y + margin_v,
        width: area.width.saturating_sub(margin_h * 2),
        height: area.height.saturating_sub(margin_v * 2),
    };

    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.console_prompt))
        .title(" Command Reference ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.console_bg).fg(theme.console_fg));

    let inner = block.inner(overlay);
    frame.render_widget(block, overlay);

    let cmd_style = Style::default()
        .fg(theme.console_prompt)
        .add_modifier(Modifier::BOLD);
    let alias_style = Style::default().fg(theme.autocomplete_fg);
    let desc_style = Style::default().fg(theme.console_fg);
    let header_style = Style::default()
        .fg(theme.console_prompt)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let commands: Vec<(&str, &str, &str, &str)> = vec![
        ("/play", "/p", "", "Start/resume playback"),
        ("/pause", "", "", "Pause playback"),
        ("/replay", "/r", "", "Restart from beginning"),
        ("/loop", "", "[N]", "Loop next N bars (default: 2)"),
        ("/bpm", "", "<number>", "Set BPM override"),
        ("/difficulty", "", "<easy|medium|hard>", "Set difficulty"),
        ("/timing", "", "<relaxed|standard|strict>", "Set timing windows"),
        ("/theme", "", "<name>", "Switch color theme"),
        ("/practice", "", "", "Toggle practice mode"),
        ("/scoreboard", "", "", "Show scores"),
        ("/cassette", "", "", "Track browser"),
        ("/mute", "", "", "Toggle all audio mute"),
        ("/mute-metronome", "", "", "Toggle metronome mute"),
        ("/mute-backtrack", "", "", "Toggle backtrack mute"),
        ("/mute-kit", "", "", "Toggle kit sound mute"),
        ("/track", "", "<name>", "Select track by name"),
        ("/kit", "", "<name>", "Select drum kit"),
        ("/channel", "", "<number>", "Override MIDI drum channel"),
        ("/calibrate", "", "", "Input calibration"),
        ("/help", "", "", "Show this reference"),
        ("/quit", "/q", "", "Exit application"),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("Commands", header_style)));
    lines.push(Line::from(""));

    for (name, alias, args, desc) in &commands {
        let mut spans = vec![Span::styled(format!("{:<18}", name), cmd_style)];
        if !alias.is_empty() {
            spans.push(Span::styled(format!(" ({}) ", alias), alias_style));
        } else {
            spans.push(Span::raw("      "));
        }
        if !args.is_empty() {
            spans.push(Span::styled(format!("{:<24}", args), alias_style));
        } else {
            spans.push(Span::raw("                        "));
        }
        spans.push(Span::styled(desc.to_string(), desc_style));
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Keys", header_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("i         ", cmd_style),
        Span::styled("Enter command mode (empty console)", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(":         ", cmd_style),
        Span::styled("Enter command mode with / prefix", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Esc       ", cmd_style),
        Span::styled("Return to normal mode", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Tab       ", cmd_style),
        Span::styled("Accept autocomplete suggestion", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+Q    ", cmd_style),
        Span::styled("Quit application", desc_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Esc or q to close",
        alias_style,
    )));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}
