use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::GameState;
use crate::engine::scoring::{HitAccuracy, NoteResult};
use crate::ui::themes::Theme;

/// Hit feedback overlay showing accuracy flashes on the hit zone.
pub struct HitFeedbackWidget;

/// Show recent hit results within the last 300ms overlaid on the highway hit zone.
pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // The hit zone is the last row of the highway area
    let hit_zone_y = area.y + area.height.saturating_sub(1);

    // Show each recent result (within last 300ms)
    // We show text centered in a lane-sized chunk if possible
    // Since we don't know lane positions here, we spread them out evenly
    // For each result we render at a column offset
    let mut col_offset: u16 = 0;
    let col_step: u16 = 9; // ~9 chars per feedback label

    for (time_ms, result) in state.recent_results.iter().rev() {
        let age_ms = state.position_ms - time_ms;
        if age_ms > 300.0 {
            continue;
        }

        let (label, color) = match result {
            NoteResult::Hit { accuracy, .. } => match accuracy {
                HitAccuracy::Perfect => ("PERFECT", theme.perfect),
                HitAccuracy::Great => ("GREAT  ", theme.great),
                HitAccuracy::Good => ("GOOD   ", theme.good),
                HitAccuracy::Ok => ("OK     ", theme.ok),
                HitAccuracy::Miss => ("MISS   ", theme.miss),
            },
            NoteResult::Miss { .. } => ("MISS   ", theme.miss),
            NoteResult::WrongPiece { .. } => ("WRONG  ", theme.wrong_piece),
            NoteResult::Extra { .. } => ("EXTRA  ", theme.extra),
        };

        // Fade: alpha decreases over 300ms — use dim modifier as approach
        let age_frac = age_ms / 300.0;
        let style = if age_frac < 0.5 {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        let x = area.x + col_offset;
        if x + label.len() as u16 > area.x + area.width {
            break;
        }

        let buf = frame.buffer_mut();
        let mut cx = x;
        for ch in label.chars() {
            if let Some(cell) = buf.cell_mut((cx, hit_zone_y)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            cx = cx.saturating_add(1);
        }

        col_offset += col_step;
        if col_offset >= area.width {
            break;
        }
    }
}
