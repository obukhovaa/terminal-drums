use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::GameState;
use crate::ui::themes::Theme;

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    let box_w = area.width.min(45);
    let list_h = state.kit_list.len().max(1) as u16 + 4; // title + padding + hints
    let box_h = area.height.min(list_h.max(8));
    let x = area.x + area.width.saturating_sub(box_w) / 2;
    let y = area.y + area.height.saturating_sub(box_h) / 2;
    let overlay_area = Rect {
        x,
        y,
        width: box_w,
        height: box_h,
    };

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        " SELECT KIT",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if state.kit_list.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (No kits found)",
            Style::default().fg(theme.fg_dim),
        )));
    } else {
        let visible_start = if state.kit_selected > 5 {
            state.kit_selected - 5
        } else {
            0
        };
        let visible_end = (visible_start + 10).min(state.kit_list.len());
        for i in visible_start..visible_end {
            let name = &state.kit_list[i];
            let marker = if i == state.kit_selected {
                "\u{25B8}"
            } else {
                " "
            };
            let style = if i == state.kit_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg)
            };
            lines.push(Line::from(Span::styled(
                format!("  {} {}", marker, name),
                style,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k:move  Enter:select  Esc:back",
        Style::default().fg(theme.fg_dim),
    )));

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg)),
    );

    frame.render_widget(para, overlay_area);
}
