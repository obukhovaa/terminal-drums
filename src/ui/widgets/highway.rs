use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::GameState;
use crate::engine::scoring::{HitAccuracy, NoteResult};
use crate::midi::types::{DrumPiece, VelocityLevel};
use crate::ui::themes::Theme;

/// Canonical lane order and their display abbreviations.
static LANE_ORDER: &[(DrumPiece, &str)] = &[
    (DrumPiece::CrashCymbal1, "CR1"),
    (DrumPiece::ClosedHiHat, "HH"),
    (DrumPiece::OpenHiHat, "OH"),
    (DrumPiece::PedalHiHat, "PH"),
    (DrumPiece::HighTom, "TH"),
    (DrumPiece::Snare, "SN"),
    (DrumPiece::CrossStick, "XS"),
    (DrumPiece::Kick, "KK"),
    (DrumPiece::MidTom, "TM"),
    (DrumPiece::LowTom, "TL"),
    (DrumPiece::RideCymbal, "RD"),
    (DrumPiece::RideBell, "RB"),
    (DrumPiece::CrashCymbal2, "CR2"),
    (DrumPiece::Splash, "SP"),
    (DrumPiece::China, "CH"),
];

/// Look-ahead window in milliseconds for note display.
const LOOK_AHEAD_MS: f64 = 2000.0;

/// Duration (ms) for hit flash and feedback effects.
const FLASH_DURATION_MS: f64 = 200.0;
const FEEDBACK_DURATION_MS: f64 = 300.0;

fn piece_glyph(piece: DrumPiece, velocity_level: VelocityLevel) -> &'static str {
    match (piece, velocity_level) {
        (DrumPiece::OpenHiHat, _) => "\u{25CA}",   // diamond
        (DrumPiece::PedalHiHat, _) => "\u{2573}",  // X
        (_, VelocityLevel::Ghost) => "\u{2591}",    // light shade
        (_, VelocityLevel::Soft) => "\u{2592}",     // medium shade
        (_, VelocityLevel::Normal) => "\u{2593}",   // dark shade
        (_, VelocityLevel::Accent) => "\u{2588}",   // full block
    }
}

fn piece_color(piece: DrumPiece, theme: &Theme) -> ratatui::style::Color {
    match piece {
        DrumPiece::Kick => theme.kick,
        DrumPiece::Snare | DrumPiece::CrossStick => theme.snare,
        DrumPiece::ClosedHiHat | DrumPiece::OpenHiHat | DrumPiece::PedalHiHat => theme.hihat,
        DrumPiece::CrashCymbal1 | DrumPiece::CrashCymbal2 => theme.crash,
        DrumPiece::RideCymbal | DrumPiece::RideBell => theme.ride,
        DrumPiece::HighTom => theme.tom_high,
        DrumPiece::MidTom => theme.tom_mid,
        DrumPiece::LowTom => theme.tom_low,
        DrumPiece::Splash => theme.splash,
        DrumPiece::China => theme.china,
    }
}

/// Find the key hint label for a given DrumPiece.
fn key_label_for(piece: DrumPiece, key_hints: &[(DrumPiece, String)]) -> Option<&str> {
    key_hints
        .iter()
        .find(|(p, _)| *p == piece)
        .map(|(_, label)| label.as_str())
}

/// Extract the DrumPiece from a NoteResult, if any.
fn result_piece(result: &NoteResult) -> Option<DrumPiece> {
    match result {
        NoteResult::Hit { note, .. } => Some(note.piece),
        NoteResult::Miss { note } => Some(note.piece),
        NoteResult::WrongPiece { expected, .. } => Some(expected.piece),
        NoteResult::Extra { piece, .. } => Some(*piece),
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    if area.height < 3 || area.width < 4 {
        return;
    }

    // Collect lanes that are actually used in this track
    let active_lanes: Vec<(DrumPiece, &str)> = LANE_ORDER
        .iter()
        .filter(|(piece, _)| state.pieces_used.contains(piece))
        .copied()
        .collect();

    // If nothing, just draw a blank highway
    if active_lanes.is_empty() {
        draw_empty_highway(frame, area, theme);
        return;
    }

    let num_lanes = active_lanes.len();
    let highway_height = area.height.saturating_sub(2); // 1 for header row, 1 for hit zone
    let lane_width = (area.width as usize / num_lanes).max(3);

    // Check which lanes have recent hits (for flash effects).
    // Match any user keypress (Hit, Extra, WrongPiece) — use the piece the
    // user actually pressed so feedback always appears in the correct lane.
    // Use max of position_ms and freeplay_clock_ms for age calculation so
    // free-play hits (when not playing) still flash and expire correctly.
    let ref_time = state.position_ms.max(state.freeplay_clock_ms);
    let mut lane_hit_flash: Vec<bool> = vec![false; num_lanes];
    for (time_ms, result) in state.recent_results.iter().rev() {
        let age_ms = ref_time - time_ms;
        if !(0.0..=FLASH_DURATION_MS).contains(&age_ms) {
            continue;
        }
        let pressed_piece = match result {
            NoteResult::Hit { note, .. } => Some(note.piece),
            NoteResult::Extra { piece, .. } => Some(*piece),
            NoteResult::WrongPiece { actual_piece, .. } => Some(*actual_piece),
            _ => None,
        };
        if let Some(piece) = pressed_piece {
            if let Some(idx) = active_lanes.iter().position(|(p, _)| *p == piece) {
                lane_hit_flash[idx] = true;
            }
        }
    }

    // Render lane headers (row 0) with key hints
    for (i, (piece, abbr)) in active_lanes.iter().enumerate() {
        let x = area.x + (i * lane_width) as u16;
        if x >= area.x + area.width {
            break;
        }
        let color = piece_color(*piece, theme);
        let header_style = Style::default().fg(theme.fg_dim);

        // Solid filled circle during flash, empty otherwise
        let is_flashing = lane_hit_flash[i];
        let hit_dot = if is_flashing { "\u{25CF}" } else { "\u{25CB}" }; // ● vs ○

        // Build label: "● SN [a]" format
        let key_hint = key_label_for(*piece, &state.key_hints);
        let label = if let Some(key) = key_hint {
            format!("{} {} [{}]", hit_dot, abbr, key)
        } else {
            format!("{} {}", hit_dot, abbr)
        };

        let padded = format!("{:^width$}", label, width = lane_width.min(10));

        // Only the circle dot gets colored; text stays dim
        let buf = frame.buffer_mut();
        let mut cx = x;
        for ch in padded.chars() {
            if cx >= area.x + area.width {
                break;
            }
            if let Some(cell) = buf.cell_mut((cx, area.y)) {
                cell.set_char(ch);
                if ch == '\u{25CF}' {
                    cell.set_style(Style::default().fg(color).add_modifier(Modifier::BOLD));
                } else if ch == '\u{25CB}' {
                    cell.set_style(Style::default().fg(theme.fg_dim));
                } else {
                    cell.set_style(header_style);
                }
            }
            cx = cx.saturating_add(1);
        }
    }

    // Draw lane separator lines (vertical bars between lanes)
    {
        let buf = frame.buffer_mut();
        for i in 1..num_lanes {
            let x = area.x + (i * lane_width) as u16;
            if x == 0 || x >= area.x + area.width {
                break;
            }
            let sep_x = x - 1;
            for row in 0..highway_height + 1 {
                let y = area.y + row;
                if let Some(cell) = buf.cell_mut((sep_x, y)) {
                    cell.set_char('\u{2502}'); // vertical line
                    cell.set_style(Style::default().fg(theme.border));
                }
            }
        }
    }

    // Paint lane background gradient for notes approaching the hit zone.
    // For each lane, find the closest note's time_until and paint the bottom
    // portion of the lane with a subtle glow that intensifies as the note approaches.
    {
        let buf = frame.buffer_mut();
        for (lane_i, (piece, _)) in active_lanes.iter().enumerate() {
            // Find the closest note for this lane
            let closest_time = state
                .visible_notes
                .iter()
                .filter(|n| n.piece == *piece)
                .map(|n| n.time_ms - state.position_ms)
                .filter(|t| *t >= 0.0 && *t < 500.0)
                .fold(f64::MAX, f64::min);

            if closest_time >= 500.0 || closest_time == f64::MAX {
                continue;
            }

            // Intensity: 0.0 at 500ms, 1.0 at 0ms
            let intensity = 1.0 - (closest_time / 500.0);
            let color = piece_color(*piece, theme);

            // Paint bottom rows of the lane with increasing intensity
            let glow_rows = ((intensity * 8.0) as u16).min(highway_height);
            let lane_x_start = area.x + (lane_i * lane_width) as u16;
            let lane_x_end = (lane_x_start + lane_width as u16).min(area.x + area.width);

            for row in 0..glow_rows {
                let y = area.y + 1 + highway_height.saturating_sub(row + 1);
                if y <= area.y || y >= area.y + area.height {
                    continue;
                }
                // Closer rows (lower row index from bottom) get brighter
                let row_intensity = intensity * (1.0 - row as f64 / glow_rows.max(1) as f64);
                if row_intensity < 0.15 {
                    continue;
                }
                for x in lane_x_start..lane_x_end {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        // Only paint background if cell is empty (no note glyph)
                        let current = cell.symbol().to_string();
                        if current.trim().is_empty() || current == " " {
                            if row_intensity > 0.6 {
                                cell.set_symbol("\u{2591}"); // ░ light shade
                                cell.set_style(Style::default().fg(color));
                            } else if row_intensity > 0.3 {
                                cell.set_symbol("\u{00B7}"); // · middle dot
                                cell.set_style(Style::default().fg(color));
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw notes scrolling top -> bottom
    for note in &state.visible_notes {
        let time_until_ms = note.time_ms - state.position_ms;
        if time_until_ms < 0.0 || time_until_ms > LOOK_AHEAD_MS {
            continue;
        }

        let y_frac = time_until_ms / LOOK_AHEAD_MS;
        let note_row = ((1.0 - y_frac) * highway_height as f64) as u16;
        let screen_y = area.y + 1 + (highway_height.saturating_sub(1).saturating_sub(note_row));

        if screen_y >= area.y + area.height {
            continue;
        }

        let lane_idx = active_lanes
            .iter()
            .position(|(p, _)| *p == note.piece);

        let lane_idx = match lane_idx {
            Some(idx) => idx,
            None => continue,
        };

        let lane_x = area.x + (lane_idx * lane_width) as u16;
        let glyph_x = lane_x + (lane_width / 2) as u16;

        if glyph_x >= area.x + area.width {
            continue;
        }

        let vel_level = VelocityLevel::from(note.velocity);
        let color = piece_color(note.piece, theme);

        let (glyph, style) = if time_until_ms < 300.0 {
            // Approaching hit zone — bold emphasis
            let g = piece_glyph(note.piece, vel_level);
            (g, Style::default().fg(color).bg(theme.header_bg).add_modifier(Modifier::BOLD))
        } else {
            let g = piece_glyph(note.piece, vel_level);
            let s = if vel_level == VelocityLevel::Accent {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            (g, s)
        };

        {
            let buf = frame.buffer_mut();
            if let Some(cell) = buf.cell_mut((glyph_x, screen_y)) {
                cell.set_symbol(glyph);
                cell.set_style(style);
            }
        }

        // Draw tail for sustained notes
        if note.duration_ms > 0.0 {
            let tail_cells = (note.duration_ms / LOOK_AHEAD_MS * highway_height as f64) as u16;
            for t in 1..=tail_cells {
                let tail_y = screen_y + t;
                if tail_y >= area.y + area.height.saturating_sub(1) {
                    break;
                }
                let tail_glyph = if t == tail_cells {
                    "\u{2575}" // up light
                } else {
                    "\u{2502}" // vertical line
                };
                let buf = frame.buffer_mut();
                if let Some(cell) = buf.cell_mut((glyph_x, tail_y)) {
                    cell.set_symbol(tail_glyph);
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }

    // Draw hit zone line at the bottom
    let hit_zone_y = area.y + 1 + highway_height;
    if hit_zone_y < area.y + area.height {
        let buf = frame.buffer_mut();
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, hit_zone_y)) {
                cell.set_char('\u{2550}'); // double horizontal
                cell.set_style(Style::default().fg(theme.accent));
            }
        }

        // Draw hit flash markers at the hit zone for recently hit lanes
        for (i, flashing) in lane_hit_flash.iter().enumerate() {
            if *flashing {
                let lane_x = area.x + (i * lane_width) as u16;
                let center_x = lane_x + (lane_width / 2) as u16;
                if center_x < area.x + area.width {
                    let color = piece_color(active_lanes[i].0, theme);
                    if let Some(cell) = buf.cell_mut((center_x, hit_zone_y)) {
                        cell.set_symbol("\u{25C9}"); // fisheye (circle with dot)
                        cell.set_style(
                            Style::default()
                                .fg(color)
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        }
    }

    // --- Integrated hit feedback (replaces hit_feedback.rs) ---
    // Show recent results (within last 300ms) aligned to their lane positions.
    // Feedback appears one row below the hit zone.
    let feedback_y = hit_zone_y.saturating_add(1);
    if feedback_y >= area.y + area.height {
        // No room for feedback row -- render on the hit zone itself as fallback
        render_hit_feedback_on_row(frame, area, hit_zone_y, state, theme, &active_lanes, lane_width);
    } else {
        render_hit_feedback_on_row(frame, area, feedback_y, state, theme, &active_lanes, lane_width);
    }

    // --- Preroll overlay ---
    if state.preroll_active {
        let msg = if state.autoplay { "Watch me drum!" } else { "Get Ready!" };
        let msg_len = msg.len() as u16;
        let cx = area.x + area.width.saturating_sub(msg_len) / 2;
        let cy = area.y + area.height / 3;
        if cy < area.y + area.height {
            let style = Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD);
            let buf = frame.buffer_mut();
            for (i, ch) in msg.chars().enumerate() {
                let x = cx + i as u16;
                if x < area.x + area.width {
                    if let Some(cell) = buf.cell_mut((x, cy)) {
                        cell.set_char(ch);
                        cell.set_style(style);
                    }
                }
            }
        }

        // Show beat countdown below
        let beats_left = (state.preroll_beats_total - state.preroll_beats_elapsed).ceil() as u32;
        let count_str = format!("{}", beats_left.max(1));
        let count_len = count_str.len() as u16;
        let count_x = area.x + area.width.saturating_sub(count_len) / 2;
        let count_y = cy + 2;
        if count_y < area.y + area.height {
            let style = Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD);
            let buf = frame.buffer_mut();
            for (i, ch) in count_str.chars().enumerate() {
                let x = count_x + i as u16;
                if x < area.x + area.width {
                    if let Some(cell) = buf.cell_mut((x, count_y)) {
                        cell.set_char(ch);
                        cell.set_style(style);
                    }
                }
            }
        }
    }
}

/// Render hit feedback labels centered on the correct lane columns.
fn render_hit_feedback_on_row(
    frame: &mut Frame,
    area: Rect,
    row_y: u16,
    state: &GameState,
    theme: &Theme,
    active_lanes: &[(DrumPiece, &str)],
    lane_width: usize,
) {
    // Clear the feedback row first to prevent stale text from previous frames
    {
        let buf = frame.buffer_mut();
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, row_y)) {
                cell.set_symbol(" ");
                cell.set_style(Style::default().bg(theme.bg));
            }
        }
    }

    // Track which lanes already have feedback so newest result wins
    let mut lane_drawn: Vec<bool> = vec![false; active_lanes.len()];

    let ref_time_fb = state.position_ms.max(state.freeplay_clock_ms);
    for (time_ms, result) in state.recent_results.iter().rev() {
        let age_ms = ref_time_fb - time_ms;
        if !(0.0..=FEEDBACK_DURATION_MS).contains(&age_ms) {
            continue;
        }

        // Use the piece the user actually pressed for lane placement
        let piece = match result {
            NoteResult::Hit { note, .. } => note.piece,
            NoteResult::Extra { piece, .. } => *piece,
            NoteResult::WrongPiece { actual_piece, .. } => *actual_piece,
            NoteResult::Miss { note } => note.piece,
        };
        let lane_idx = match active_lanes.iter().position(|(p, _)| *p == piece) {
            Some(idx) => idx,
            None => continue,
        };

        // Only show the most recent result per lane
        if lane_drawn[lane_idx] {
            continue;
        }
        lane_drawn[lane_idx] = true;

        let (label, color) = match result {
            NoteResult::Hit { accuracy, .. } => match accuracy {
                HitAccuracy::Perfect => ("PERFECT", theme.perfect),
                HitAccuracy::Great => ("GREAT", theme.great),
                HitAccuracy::Good => ("GOOD", theme.good),
                HitAccuracy::Ok => ("OK", theme.ok),
                HitAccuracy::Miss => ("MISS", theme.miss),
            },
            NoteResult::Miss { .. } => ("MISS", theme.miss),
            NoteResult::WrongPiece { .. } => ("WRONG", theme.wrong_piece),
            NoteResult::Extra { .. } => ("EXTRA", theme.extra),
        };

        // Fade: bold in first half, normal in second half
        let age_frac = age_ms / FEEDBACK_DURATION_MS;
        let style = if age_frac < 0.5 {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        // Center the label within the lane
        let lane_x = area.x + (lane_idx * lane_width) as u16;
        let label_len = label.len() as u16;
        let centered_x = if lane_width as u16 >= label_len {
            lane_x + (lane_width as u16 - label_len) / 2
        } else {
            lane_x
        };

        let buf = frame.buffer_mut();
        let mut cx = centered_x;
        for ch in label.chars() {
            if cx >= area.x + area.width {
                break;
            }
            if let Some(cell) = buf.cell_mut((cx, row_y)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
            cx = cx.saturating_add(1);
        }
    }
}

fn draw_empty_highway(frame: &mut Frame, area: Rect, theme: &Theme) {
    let hit_zone_y = area.y + area.height.saturating_sub(1);
    let buf = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, hit_zone_y)) {
            cell.set_char('\u{2550}'); // double horizontal
            cell.set_style(Style::default().fg(theme.accent));
        }
    }
}

