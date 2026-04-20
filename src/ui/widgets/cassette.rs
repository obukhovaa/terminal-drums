use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::GameState;
use crate::ui::themes::Theme;

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    // Use most of the screen height, reasonable width
    let box_w = area.width.min(50).max(30);
    let box_h = area.height.saturating_sub(4).max(10);
    let x = area.x + area.width.saturating_sub(box_w) / 2;
    let y = area.y + area.height.saturating_sub(box_h) / 2;
    let overlay = Rect { x, y, width: box_w, height: box_h };

    frame.render_widget(Clear, overlay);

    // Inner area (inside borders): 2 less width, 2 less height
    let inner_w = box_w.saturating_sub(2) as usize;
    // Rows: title(1) + blank(1) + search?(1) + list + blank(1) + hints(1)
    let has_search = state.track_search_active || !state.track_search.is_empty();
    let chrome_rows: u16 = 2 + 1 + 1 + if has_search { 1 } else { 0 }; // title+blank, blank, hints, search
    let list_rows = box_h.saturating_sub(2).saturating_sub(chrome_rows) as usize; // -2 for borders

    let filtered = &state.track_filtered;
    let total = filtered.len();
    let selected = state.track_selected.min(total.saturating_sub(1));

    // Compute visible window centered on selection
    let visible_start = if total <= list_rows {
        0
    } else if selected < list_rows / 2 {
        0
    } else if selected + list_rows / 2 >= total {
        total.saturating_sub(list_rows)
    } else {
        selected - list_rows / 2
    };
    let visible_end = (visible_start + list_rows).min(total);

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        " SELECT TRACK",
        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
    )));

    // Search bar
    if has_search {
        let search_text = format!(" /{}", state.track_search);
        let cursor = if state.track_search_active { "_" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(
                search_text,
                Style::default().fg(theme.console_fg).bg(theme.console_bg),
            ),
            Span::styled(
                cursor.to_string(),
                Style::default().fg(theme.console_bg).bg(theme.console_fg),
            ),
            Span::styled(
                format!("  {} matches", total),
                Style::default().fg(theme.fg_dim).bg(theme.console_bg),
            ),
        ]));
    } else {
        lines.push(Line::from(""));
    }

    // Track list
    if total == 0 {
        lines.push(Line::from(Span::styled(
            if state.track_list.is_empty() {
                "  (No tracks found)"
            } else {
                "  (No matches)"
            },
            Style::default().fg(theme.fg_dim),
        )));
    } else {
        for vi in visible_start..visible_end {
            let real_idx = filtered[vi];
            let name = &state.track_list[real_idx];
            let is_selected = vi == selected;
            let marker = if is_selected { "\u{25B8}" } else { " " };

            // Truncate name to fit with marker + scrollbar
            let max_name = inner_w.saturating_sub(5); // "  ▸ " + scrollbar
            let display_name = if name.len() > max_name {
                &name[..max_name]
            } else {
                name
            };

            // Scrollbar character on the right
            let scrollbar_ch = if total > list_rows {
                let bar_height = list_rows.max(1);
                let thumb_at = (selected as f64 / total.max(1) as f64 * bar_height as f64) as usize;
                if vi - visible_start == thumb_at {
                    "\u{2588}" // █ thumb
                } else {
                    "\u{2502}" // │ track
                }
            } else {
                " "
            };

            let style = if is_selected {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg)
            };
            let scrollbar_style = Style::default().fg(theme.fg_dim);

            let pad = inner_w.saturating_sub(display_name.len() + 4 + 1);
            lines.push(Line::from(vec![
                Span::styled(format!("  {} {}", marker, display_name), style),
                Span::styled(" ".repeat(pad), Style::default()),
                Span::styled(scrollbar_ch, scrollbar_style),
            ]));
        }
    }

    // Pad remaining rows
    let used = lines.len();
    let target = (box_h as usize).saturating_sub(2 + 2); // borders + hints + blank
    for _ in used..target {
        lines.push(Line::from(""));
    }

    // Bottom hints
    lines.push(Line::from(""));
    let hint = if state.track_search_active {
        " type to filter  Enter:done  Esc:clear"
    } else {
        " j/k:move  Enter:select  /:search  Esc:back"
    };
    lines.push(Line::from(Span::styled(hint, Style::default().fg(theme.fg_dim))));

    // Count indicator
    if total > 0 && !state.track_search_active {
        let count = format!(" {}/{}", selected + 1, total);
        // Replace last line to include count on the right
        let last = lines.last_mut().unwrap();
        let hint_len = hint.len();
        let count_len = count.len();
        let gap = inner_w.saturating_sub(hint_len + count_len);
        *last = Line::from(vec![
            Span::styled(hint, Style::default().fg(theme.fg_dim)),
            Span::styled(" ".repeat(gap), Style::default()),
            Span::styled(count, Style::default().fg(theme.fg_dim)),
        ]);
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, overlay);
}
