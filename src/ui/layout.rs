use crate::app::GameState;
use crate::error::AppError;

/// Pre-computed layout heights for the main screen regions.
pub struct AppLayout {
    pub header_height: u16,
    pub highway_height: u16,
    pub info_height: u16,
    pub console_height: u16,
}

/// Build the responsive layout from the terminal area.
///
/// Returns an error if the terminal is too small (< 80x24).
pub fn build_layout(cols: u16, rows: u16, _has_console: bool, state: &GameState) -> Result<AppLayout, AppError> {
    if rows < 24 || cols < 80 {
        return Err(AppError::TerminalTooSmall {
            need_cols: 80,
            need_rows: 24,
            have_cols: cols,
            have_rows: rows,
        });
    }

    let header_height = crate::ui::widgets::header::header_height(state, cols);
    // Console always takes exactly 1 line (docked to bottom)
    let console_height: u16 = 1;

    // Remaining rows for highway + info
    let remaining = rows
        .saturating_sub(header_height)
        .saturating_sub(console_height);

    // Compute ideal info height: max of metronome needs and score widget minimum.
    // Score needs 5 rows (border + 3 content lines + border).
    // Metronome wraps beats into rows based on available width.
    let metro_width = (cols as f32 * 0.4) as u16;
    let (ts_num, _) = state.time_sig;
    let metro_h = crate::ui::widgets::metronome::metronome_height(ts_num, metro_width);
    let score_min: u16 = 5; // 2 borders + score line + breakdown line + progress bar
    let ideal_info = metro_h.max(score_min);

    // Give info section what it needs, but cap so highway gets at least 8 rows
    let min_highway: u16 = 8;
    let info_height = if remaining > min_highway + ideal_info {
        ideal_info
    } else if remaining > min_highway {
        remaining - min_highway
    } else {
        0
    };

    let highway_height = remaining.saturating_sub(info_height);

    Ok(AppLayout {
        header_height,
        highway_height,
        info_height,
        console_height,
    })
}
