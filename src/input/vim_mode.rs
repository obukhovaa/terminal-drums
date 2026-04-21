use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::key_map::KeyMap;
use crate::midi::types::DrumPiece;

/// Input mode for the vim-style state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Normal mode: key presses map to drum hits.
    Normal,
    /// Command mode: key presses go to the command console.
    Command,
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Normal
    }
}

/// Actions that the vim mode state machine can produce.
pub enum VimAction {
    /// A drum piece was struck.
    DrumHit(DrumPiece),
    /// Transition to Command mode with '/' pre-filled (vim ':' behaviour).
    EnterCommand,
    /// Transition back to Normal mode, clearing the console.
    ExitCommand,
    /// Execute the current console_input as a command, then clear it.
    ExecuteCommand(String),
    /// Accept the currently highlighted autocomplete suggestion.
    AcceptAutocomplete,
    /// Move autocomplete selection up.
    AutocompleteUp,
    /// Move autocomplete selection down.
    AutocompleteDown,
    /// Append a printable character to the console input.
    AppendChar(char),
    /// Delete the last character in the console input.
    Backspace,
    /// Request application quit (Ctrl+Q / Ctrl+C in Normal mode).
    QuitRequest,
    /// No action (key was consumed but produced no meaningful output).
    None,
}

/// Vim-style modal input state machine.
///
/// The game thread owns one of these and calls `handle_key` for every
/// `InputEvent::Key` it receives.  The returned `VimAction` tells the
/// game thread what to do next.
pub struct VimModeHandler {
    pub mode: InputMode,
    /// Current text in the command console (always starts with '/').
    pub console_input: String,
    /// Cursor position within `console_input` (byte index).
    pub console_cursor: usize,
}

impl VimModeHandler {
    pub fn new() -> Self {
        Self {
            mode: InputMode::Normal,
            console_input: String::new(),
            console_cursor: 0,
        }
    }

    /// Process a key event and return the action the game thread should take.
    pub fn handle_key(&mut self, key: KeyEvent, key_map: &KeyMap) -> VimAction {
        match self.mode {
            InputMode::Normal => self.handle_normal(key, key_map),
            InputMode::Command => self.handle_command(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent, key_map: &KeyMap) -> VimAction {
        // Ctrl+Q or Ctrl+C → quit
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('c') => return VimAction::QuitRequest,
                _ => {}
            }
        }

        match key.code {
            // ':' — enter COMMAND mode with '/' pre-filled (vim-style command prefix)
            KeyCode::Char(':') => {
                self.mode = InputMode::Command;
                self.console_input = "/".to_string();
                self.console_cursor = 1;
                VimAction::EnterCommand
            }
            // All other keys: check key map first
            _ => {
                if let Some(piece) = key_map.get(&key.code) {
                    VimAction::DrumHit(piece)
                } else {
                    VimAction::None
                }
            }
        }
    }

    fn handle_command(&mut self, key: KeyEvent) -> VimAction {
        // Ctrl+C in COMMAND → cancel (return to Normal, do NOT quit)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.exit_command();
            return VimAction::ExitCommand;
        }

        match key.code {
            // Esc → return to Normal, clear console
            KeyCode::Esc => {
                self.exit_command();
                VimAction::ExitCommand
            }
            // Enter → if autocomplete is visible, this will be handled as AcceptAutocomplete
            // by the app layer; otherwise execute the typed command.
            KeyCode::Enter => {
                // The app layer checks if autocomplete is active and dispatches accordingly.
                let cmd = self.console_input.clone();
                self.exit_command();
                VimAction::ExecuteCommand(cmd)
            }
            // Tab → cycle autocomplete down
            KeyCode::Tab => VimAction::AutocompleteDown,
            // Shift+Tab → cycle autocomplete up
            KeyCode::BackTab => VimAction::AutocompleteUp,
            // Up/Down → also navigate autocomplete
            KeyCode::Up => VimAction::AutocompleteUp,
            KeyCode::Down => VimAction::AutocompleteDown,
            // Backspace → remove last char
            KeyCode::Backspace => {
                if !self.console_input.is_empty() {
                    self.console_input.pop();
                    self.console_cursor = self.console_input.len();
                    VimAction::Backspace
                } else {
                    VimAction::None
                }
            }
            // Printable character → append
            KeyCode::Char(c) => {
                self.console_input.push(c);
                self.console_cursor = self.console_input.len();
                VimAction::AppendChar(c)
            }
            _ => VimAction::None,
        }
    }

    /// Transition to Normal mode and clear the console.
    fn exit_command(&mut self) {
        self.mode = InputMode::Normal;
        self.console_input.clear();
        self.console_cursor = 0;
    }
}

impl Default for VimModeHandler {
    fn default() -> Self {
        Self::new()
    }
}
