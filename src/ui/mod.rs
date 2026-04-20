pub mod layout;
pub mod themes;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};

use crate::app::{AppState, GameState};
use crate::ui::layout::build_layout;
use crate::ui::themes::get_theme;

/// Main render entry point. Called each frame from the render thread.
pub fn render(frame: &mut Frame, state: &GameState) {
    let area = frame.area();
    let theme = get_theme(state.theme);

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(theme.bg));
    frame.render_widget(bg_block, area);

    match &state.app_state {
        AppState::Welcome => {
            widgets::welcome::render(frame, area, state, &theme);
        }

        AppState::TrackSelect => {
            widgets::cassette::render(frame, area, state, &theme);
        }

        AppState::KitSelect => {
            widgets::kit_select::render(frame, area, state, &theme);
        }

        AppState::ThemeSelect => {
            // Show session underneath with live preview
            render_session_layout(frame, area, state, &theme);
            widgets::theme_select::render(frame, area, state, &theme);
        }

        AppState::Scoreboard => {
            // Show the session view underneath, then scoreboard overlay on top
            render_session_layout(frame, area, state, &theme);
            widgets::scoreboard::render(frame, area, state, &theme);
        }

        AppState::Session(_) => {
            render_session_layout(frame, area, state, &theme);
        }

        AppState::Calibrating => {
            // Render session underneath (or a blank background)
            render_session_layout(frame, area, state, &theme);
            // Overlay the calibration widget
            widgets::calibrate::render(frame, area, state, &theme);
        }

        AppState::Quitting => {
            // Blank screen / nothing needed
        }
    }

    // Help overlay (rendered on top of everything)
    if state.help_visible {
        widgets::help::render(frame, area, state, &theme);
    }
}

/// Render the full session layout: header, highway, info bar (score + metronome), console.
fn render_session_layout(frame: &mut Frame, area: Rect, state: &GameState, theme: &crate::ui::themes::Theme) {
    let app_layout = match build_layout(area.width, area.height, false, state) {
        Ok(l) => l,
        Err(_) => {
            render_too_small(frame, area, theme);
            return;
        }
    };

    // Console is always 1 line docked to the very bottom
    let console_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(app_layout.console_height),
        width: area.width,
        height: app_layout.console_height,
    };

    // Everything above the console
    let above_console = area.height.saturating_sub(app_layout.console_height);
    let has_info = app_layout.info_height > 0;

    let mut constraints = vec![
        Constraint::Length(app_layout.header_height),
        Constraint::Length(app_layout.highway_height),
    ];
    if has_info {
        constraints.push(Constraint::Length(app_layout.info_height));
    }

    let upper_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: above_console,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(upper_area);

    let header_area = chunks[0];
    let highway_area = chunks[1];

    // Header
    widgets::header::render(frame, header_area, state, theme);

    // Highway (now includes hit feedback overlay)
    widgets::highway::render(frame, highway_area, state, theme);

    // Info bar: score (left, 60%) + metronome (right, 40%)
    if has_info {
        if let Some(&info_area) = chunks.get(2) {
            let info_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60),
                    Constraint::Percentage(40),
                ])
                .split(info_area);

            widgets::score::render(frame, info_chunks[0], state, theme);
            widgets::metronome::render(frame, info_chunks[1], state, theme);
        }
    }

    // Console (always at the very bottom)
    widgets::console::render(frame, console_area, state, theme);
}

fn render_too_small(frame: &mut Frame, area: Rect, theme: &crate::ui::themes::Theme) {
    let msg = Paragraph::new("Terminal too small. Minimum: 80x24")
        .style(Style::default().fg(theme.miss).bg(theme.bg));
    frame.render_widget(msg, area);
}
