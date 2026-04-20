use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::GameState;
use crate::ui::themes::Theme;

/// Width of a single beat box: 5 chars (e.g. "+===+")
const BOX_WIDTH: u16 = 5;
/// Gap between boxes
const BOX_GAP: u16 = 2;
/// Rows per beat box (top border + content + bottom border)
const BOX_ROWS: u16 = 3;

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    if area.height < 3 || area.width < 5 {
        return;
    }

    let (ts_num, _ts_denom) = state.time_sig;
    let num_beats = ts_num as u16;
    if num_beats == 0 {
        return;
    }

    let active_beat = state.current_beat.floor() as u16 % num_beats;

    // How many beats fit per row?
    let beats_per_row = ((area.width + BOX_GAP) / (BOX_WIDTH + BOX_GAP)).max(1);
    let num_rows = (num_beats + beats_per_row - 1) / beats_per_row;
    // Each row needs BOX_ROWS (3) + 1 for intensity bar
    let row_height = if area.height > num_rows * BOX_ROWS { BOX_ROWS + 1 } else { BOX_ROWS };
    let total_height = num_rows * row_height;

    // Vertical start: center if space allows
    let start_y = if total_height < area.height {
        area.y + (area.height - total_height) / 2
    } else {
        area.y
    };

    let buf = frame.buffer_mut();

    for beat_idx in 0..num_beats {
        let row = beat_idx / beats_per_row;
        let col = beat_idx % beats_per_row;

        // How many beats on this row?
        let beats_this_row = if row < num_rows - 1 {
            beats_per_row
        } else {
            num_beats - row * beats_per_row
        };
        let row_width = beats_this_row * BOX_WIDTH + beats_this_row.saturating_sub(1) * BOX_GAP;
        let row_start_x = if row_width < area.width {
            area.x + (area.width - row_width) / 2
        } else {
            area.x
        };

        let box_x = row_start_x + col * (BOX_WIDTH + BOX_GAP);
        let box_y = start_y + row * row_height;

        if box_x + BOX_WIDTH > area.x + area.width {
            continue;
        }
        if box_y + 2 >= area.y + area.height {
            continue;
        }

        let is_active = beat_idx == active_beat;
        let is_downbeat = beat_idx == 0;

        let (style, top_line, mid_left, mid_right, bot_line) = if is_active {
            let color = if is_downbeat {
                theme.metronome_accent
            } else {
                theme.accent
            };
            let s = Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD);
            (
                s,
                "\u{2554}\u{2550}\u{2550}\u{2550}\u{2557}", // ╔═══╗
                "\u{2551}",                                   // ║
                "\u{2551}",                                   // ║
                "\u{255A}\u{2550}\u{2550}\u{2550}\u{255D}", // ╚═══╝
            )
        } else {
            let s = Style::default().fg(theme.fg_dim);
            (
                s,
                "\u{250C}\u{2500}\u{2500}\u{2500}\u{2510}", // ┌───┐
                "\u{2502}",                                   // │
                "\u{2502}",                                   // │
                "\u{2514}\u{2500}\u{2500}\u{2500}\u{2518}", // └───┘
            )
        };

        let beat_label = format!("{}", beat_idx + 1);
        let label_style = if is_active {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        };

        render_str_at(buf, box_x, box_y, top_line, style);
        render_str_at(buf, box_x, box_y + 1, mid_left, style);
        let inner_x = box_x + 1 + (3 - beat_label.len() as u16) / 2;
        render_str_at(buf, inner_x, box_y + 1, &beat_label, label_style);
        render_str_at(buf, box_x + 4, box_y + 1, mid_right, style);
        render_str_at(buf, box_x, box_y + 2, bot_line, style);

        // Intensity bar below box: fills progressively during the active beat
        if box_y + 3 < area.y + area.height {
            let bar_width: usize = 5;
            let beat_color = if is_downbeat {
                theme.metronome_accent
            } else {
                theme.accent
            };

            if is_active {
                // Fill based on phase (0.0 = empty, 1.0 = full)
                let phase = state.metronome_phase;
                let filled = (phase * bar_width as f64).ceil() as usize;
                for c in 0..bar_width {
                    let ch = if c < filled { '\u{2593}' } else { '\u{2591}' }; // ▓ vs ░
                    let s = if c < filled {
                        Style::default().fg(beat_color)
                    } else {
                        Style::default().fg(theme.fg_dim)
                    };
                    if let Some(cell) = buf.cell_mut((box_x + c as u16, box_y + 3)) {
                        cell.set_char(ch);
                        cell.set_style(s);
                    }
                }
            } else {
                // Inactive: show empty bar
                let s = Style::default().fg(theme.fg_dim);
                for c in 0..bar_width {
                    if let Some(cell) = buf.cell_mut((box_x + c as u16, box_y + 3)) {
                        cell.set_char('\u{2591}'); // ░
                        cell.set_style(s);
                    }
                }
            }
        }
    }
}

/// Compute the height needed for the metronome at a given width and time signature.
pub fn metronome_height(ts_num: u8, width: u16) -> u16 {
    let num_beats = ts_num as u16;
    if num_beats == 0 || width < BOX_WIDTH {
        return 0;
    }
    let beats_per_row = ((width + BOX_GAP) / (BOX_WIDTH + BOX_GAP)).max(1);
    let num_rows = (num_beats + beats_per_row - 1) / beats_per_row;
    // Each row: 3 for box + 1 for intensity bar
    num_rows * 4
}

fn render_str_at(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    let mut cx = x;
    for ch in s.chars() {
        if let Some(cell) = buf.cell_mut((cx, y)) {
            cell.set_char(ch);
            cell.set_style(style);
        }
        cx = cx.saturating_add(1);
    }
}
