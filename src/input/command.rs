use thiserror::Error;

/// A registered slash command.
pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub args: ArgSpec,
    pub description: &'static str,
}

/// Argument specification for a command.
#[derive(Debug, Clone)]
pub enum ArgSpec {
    None,
    Optional(ArgType),
    Required(ArgType),
}

/// Argument type for validation.
#[derive(Debug, Clone)]
pub enum ArgType {
    Number,
    Text,
    Choice(&'static [&'static str]),
}

/// Error from command parsing or execution.
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Unknown command: {0}")]
    Unknown(String),

    #[error("Missing required argument for /{command}")]
    MissingArg { command: String },

    #[error("Invalid argument for /{command}: {reason}")]
    InvalidArg { command: String, reason: String },

    #[error("Command not available in current state: /{0}")]
    NotAvailable(String),
}

/// A successfully parsed command ready for dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    pub name: String,
    pub arg: Option<String>,
}

/// Registry of all available slash commands.
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    /// Build the registry with all commands from spec §9.2.
    pub fn new() -> Self {
        let commands = vec![
            Command {
                name: "play",
                aliases: &["p"],
                args: ArgSpec::None,
                description: "Start/resume playback",
            },
            Command {
                name: "pause",
                aliases: &[],
                args: ArgSpec::None,
                description: "Pause playback",
            },
            Command {
                name: "replay",
                aliases: &["r"],
                args: ArgSpec::None,
                description: "Restart from beginning",
            },
            Command {
                name: "loop",
                aliases: &[],
                args: ArgSpec::Optional(ArgType::Number),
                description: "Loop next N bars (default: 2)",
            },
            Command {
                name: "track",
                aliases: &[],
                args: ArgSpec::None,
                description: "Open track browser",
            },
            Command {
                name: "cassette",
                aliases: &[],
                args: ArgSpec::None,
                description: "Open track browser",
            },
            Command {
                name: "bpm",
                aliases: &[],
                args: ArgSpec::Required(ArgType::Number),
                description: "Set BPM override",
            },
            Command {
                name: "kit",
                aliases: &[],
                args: ArgSpec::None,
                description: "Open kit browser",
            },
            Command {
                name: "difficulty",
                aliases: &[],
                args: ArgSpec::Required(ArgType::Choice(&["easy", "medium", "hard"])),
                description: "Set difficulty",
            },
            Command {
                name: "timing",
                aliases: &[],
                args: ArgSpec::Required(ArgType::Choice(&["relaxed", "standard", "strict"])),
                description: "Set timing windows",
            },
            Command {
                name: "practice",
                aliases: &[],
                args: ArgSpec::None,
                description: "Toggle practice mode (requires active loop)",
            },
            Command {
                name: "autoplay",
                aliases: &[],
                args: ArgSpec::None,
                description: "Toggle autoplay mode",
            },
            Command {
                name: "scoreboard",
                aliases: &[],
                args: ArgSpec::None,
                description: "Show scores for current track",
            },
            Command {
                name: "mute",
                aliases: &[],
                args: ArgSpec::None,
                description: "Toggle all audio mute",
            },
            Command {
                name: "mute-metronome",
                aliases: &[],
                args: ArgSpec::None,
                description: "Toggle metronome mute",
            },
            Command {
                name: "mute-backtrack",
                aliases: &[],
                args: ArgSpec::None,
                description: "Toggle backtrack mute",
            },
            Command {
                name: "mute-kit",
                aliases: &[],
                args: ArgSpec::None,
                description: "Toggle kit sound mute",
            },
            Command {
                name: "theme",
                aliases: &[],
                args: ArgSpec::None,
                description: "Open theme browser",
            },
            Command {
                name: "calibrate",
                aliases: &[],
                args: ArgSpec::None,
                description: "Run input calibration",
            },
            Command {
                name: "channel",
                aliases: &[],
                args: ArgSpec::Required(ArgType::Number),
                description: "Override MIDI drum channel",
            },
            Command {
                name: "help",
                aliases: &[],
                args: ArgSpec::None,
                description: "Show command reference",
            },
            Command {
                name: "reset",
                aliases: &[],
                args: ArgSpec::None,
                description: "Reset profile and scores",
            },
            Command {
                name: "quit",
                aliases: &["q", "qa", "q!"],
                args: ArgSpec::None,
                description: "Exit application",
            },
        ];

        Self { commands }
    }

    /// Parse a slash-command input string into a `ParsedCommand`.
    ///
    /// `input` may start with '/' or not — both are accepted. The name is
    /// matched against both `name` and `aliases`. Args are validated according
    /// to the command's `ArgSpec`.
    pub fn parse(&self, input: &str) -> Result<ParsedCommand, CommandError> {
        // Strip all leading '/' characters (handles double-slash from : + /cmd)
        let input = input.trim_start_matches('/').trim();

        // Split into name + optional arg (at most one space-separated argument;
        // everything after the first space is the arg, to allow text args with spaces).
        let (cmd_name, raw_arg) = match input.find(' ') {
            Some(pos) => {
                let arg = input[pos + 1..].trim();
                (&input[..pos], if arg.is_empty() { None } else { Some(arg) })
            }
            None => (input, None),
        };

        // Find the matching command: exact match first, then unique prefix match.
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name == cmd_name || c.aliases.contains(&cmd_name))
            .or_else(|| {
                // Prefix match: if exactly one command starts with this prefix, use it
                let prefix_matches: Vec<_> = self
                    .commands
                    .iter()
                    .filter(|c| {
                        c.name.starts_with(cmd_name)
                            || c.aliases.iter().any(|a| a.starts_with(cmd_name))
                    })
                    .collect();
                if prefix_matches.len() == 1 {
                    Some(prefix_matches[0])
                } else {
                    None
                }
            })
            .ok_or_else(|| CommandError::Unknown(cmd_name.to_string()))?;

        // Validate argument against spec.
        let validated_arg = match &cmd.args {
            ArgSpec::None => {
                // Ignore any trailing text — silently accepted.
                None
            }
            ArgSpec::Required(arg_type) => {
                let arg = raw_arg.ok_or_else(|| CommandError::MissingArg {
                    command: cmd.name.to_string(),
                })?;
                validate_arg(cmd.name, arg, arg_type)?;
                Some(arg.to_string())
            }
            ArgSpec::Optional(arg_type) => match raw_arg {
                None => None,
                Some(arg) => {
                    validate_arg(cmd.name, arg, arg_type)?;
                    Some(arg.to_string())
                }
            },
        };

        Ok(ParsedCommand {
            name: cmd.name.to_string(),
            arg: validated_arg,
        })
    }

    /// Return the argument hint string for a given command name.
    ///
    /// Returns `None` for commands that take no arguments.
    pub fn arg_hint(&self, cmd_name: &str) -> Option<String> {
        let name = cmd_name.trim_start_matches('/');
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name == name || c.aliases.contains(&name))?;
        match &cmd.args {
            ArgSpec::None => None,
            ArgSpec::Optional(t) | ArgSpec::Required(t) => Some(arg_type_hint(t)),
        }
    }

    /// Return up to 8 autocomplete suggestions for a partial input string.
    ///
    /// Spec §9.3: input must start with '/'. Suggestions are returned as
    /// "/name" strings, sorted alphabetically. Returns (suggestions, total_matches)
    /// so the UI can show "... and N more" when truncated.
    pub fn autocomplete_with_count(&self, input: &str) -> (Vec<String>, usize) {
        if !input.starts_with('/') {
            return (Vec::new(), 0);
        }

        let query = &input[1..];

        let mut matches: Vec<String> = self
            .commands
            .iter()
            .filter(|cmd| {
                cmd.name.starts_with(query)
                    || cmd.aliases.iter().any(|a| a.starts_with(query))
            })
            .map(|cmd| format!("/{}", cmd.name))
            .collect();

        matches.sort();
        let total = matches.len();
        matches.truncate(8);
        (matches, total)
    }

    /// Convenience wrapper returning just the suggestions (backward compat).
    pub fn autocomplete(&self, input: &str) -> Vec<String> {
        self.autocomplete_with_count(input).0
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Return a human-readable hint for an argument type.
fn arg_type_hint(t: &ArgType) -> String {
    match t {
        ArgType::Number => "<number>".to_string(),
        ArgType::Text => "<name>".to_string(),
        ArgType::Choice(choices) => choices.join(" | "),
    }
}

/// Validate a raw argument string against the expected `ArgType`.
fn validate_arg(cmd: &str, arg: &str, arg_type: &ArgType) -> Result<(), CommandError> {
    match arg_type {
        ArgType::Number => {
            if arg.parse::<f64>().is_err() {
                return Err(CommandError::InvalidArg {
                    command: cmd.to_string(),
                    reason: format!("expected a number, got '{}'", arg),
                });
            }
        }
        ArgType::Text => {
            // Any non-empty string is valid; we already checked for empty above.
        }
        ArgType::Choice(choices) => {
            if !choices.contains(&arg) {
                return Err(CommandError::InvalidArg {
                    command: cmd.to_string(),
                    reason: format!(
                        "expected one of [{}], got '{}'",
                        choices.join(", "),
                        arg
                    ),
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::new()
    }

    // --- Parsing: happy paths ---

    #[test]
    fn parse_play_no_arg() {
        let reg = registry();
        assert_eq!(
            reg.parse("/play").unwrap(),
            ParsedCommand { name: "play".into(), arg: None }
        );
    }

    #[test]
    fn parse_play_alias() {
        let reg = registry();
        assert_eq!(
            reg.parse("/p").unwrap(),
            ParsedCommand { name: "play".into(), arg: None }
        );
    }

    #[test]
    fn parse_pause() {
        let reg = registry();
        assert_eq!(
            reg.parse("/pause").unwrap(),
            ParsedCommand { name: "pause".into(), arg: None }
        );
    }

    #[test]
    fn parse_replay_no_slash() {
        // Input without leading '/' should still parse.
        let reg = registry();
        assert_eq!(
            reg.parse("replay").unwrap(),
            ParsedCommand { name: "replay".into(), arg: None }
        );
    }

    #[test]
    fn parse_replay_alias() {
        let reg = registry();
        assert_eq!(
            reg.parse("/r").unwrap(),
            ParsedCommand { name: "replay".into(), arg: None }
        );
    }

    #[test]
    fn parse_loop_optional_no_arg() {
        let reg = registry();
        assert_eq!(
            reg.parse("/loop").unwrap(),
            ParsedCommand { name: "loop".into(), arg: None }
        );
    }

    #[test]
    fn parse_loop_with_arg() {
        let reg = registry();
        assert_eq!(
            reg.parse("/loop 4").unwrap(),
            ParsedCommand { name: "loop".into(), arg: Some("4".into()) }
        );
    }

    #[test]
    fn parse_track_no_arg() {
        let reg = registry();
        assert_eq!(
            reg.parse("/track").unwrap(),
            ParsedCommand { name: "track".into(), arg: None }
        );
    }

    #[test]
    fn parse_bpm_with_number() {
        let reg = registry();
        assert_eq!(
            reg.parse("/bpm 120").unwrap(),
            ParsedCommand { name: "bpm".into(), arg: Some("120".into()) }
        );
    }

    #[test]
    fn parse_bpm_float() {
        let reg = registry();
        assert_eq!(
            reg.parse("/bpm 98.5").unwrap(),
            ParsedCommand { name: "bpm".into(), arg: Some("98.5".into()) }
        );
    }

    #[test]
    fn parse_kit_no_arg() {
        let reg = registry();
        assert_eq!(
            reg.parse("/kit").unwrap(),
            ParsedCommand { name: "kit".into(), arg: None }
        );
    }

    #[test]
    fn parse_difficulty_easy() {
        let reg = registry();
        assert_eq!(
            reg.parse("/difficulty easy").unwrap(),
            ParsedCommand { name: "difficulty".into(), arg: Some("easy".into()) }
        );
    }

    #[test]
    fn parse_difficulty_medium() {
        let reg = registry();
        assert_eq!(
            reg.parse("/difficulty medium").unwrap(),
            ParsedCommand { name: "difficulty".into(), arg: Some("medium".into()) }
        );
    }

    #[test]
    fn parse_difficulty_hard() {
        let reg = registry();
        assert_eq!(
            reg.parse("/difficulty hard").unwrap(),
            ParsedCommand { name: "difficulty".into(), arg: Some("hard".into()) }
        );
    }

    #[test]
    fn parse_timing_relaxed() {
        let reg = registry();
        assert_eq!(
            reg.parse("/timing relaxed").unwrap(),
            ParsedCommand { name: "timing".into(), arg: Some("relaxed".into()) }
        );
    }

    #[test]
    fn parse_timing_standard() {
        let reg = registry();
        assert_eq!(
            reg.parse("/timing standard").unwrap(),
            ParsedCommand { name: "timing".into(), arg: Some("standard".into()) }
        );
    }

    #[test]
    fn parse_timing_strict() {
        let reg = registry();
        assert_eq!(
            reg.parse("/timing strict").unwrap(),
            ParsedCommand { name: "timing".into(), arg: Some("strict".into()) }
        );
    }

    #[test]
    fn parse_practice() {
        let reg = registry();
        assert_eq!(
            reg.parse("/practice").unwrap(),
            ParsedCommand { name: "practice".into(), arg: None }
        );
    }

    #[test]
    fn parse_scoreboard() {
        let reg = registry();
        assert_eq!(
            reg.parse("/scoreboard").unwrap(),
            ParsedCommand { name: "scoreboard".into(), arg: None }
        );
    }

    #[test]
    fn parse_mute() {
        let reg = registry();
        assert_eq!(
            reg.parse("/mute").unwrap(),
            ParsedCommand { name: "mute".into(), arg: None }
        );
    }

    #[test]
    fn parse_mute_metronome() {
        let reg = registry();
        assert_eq!(
            reg.parse("/mute-metronome").unwrap(),
            ParsedCommand { name: "mute-metronome".into(), arg: None }
        );
    }

    #[test]
    fn parse_mute_backtrack() {
        let reg = registry();
        assert_eq!(
            reg.parse("/mute-backtrack").unwrap(),
            ParsedCommand { name: "mute-backtrack".into(), arg: None }
        );
    }

    #[test]
    fn parse_mute_kit() {
        let reg = registry();
        assert_eq!(
            reg.parse("/mute-kit").unwrap(),
            ParsedCommand { name: "mute-kit".into(), arg: None }
        );
    }

    #[test]
    fn parse_theme() {
        let reg = registry();
        assert_eq!(
            reg.parse("/theme").unwrap(),
            ParsedCommand { name: "theme".into(), arg: None }
        );
    }

    #[test]
    fn parse_calibrate() {
        let reg = registry();
        assert_eq!(
            reg.parse("/calibrate").unwrap(),
            ParsedCommand { name: "calibrate".into(), arg: None }
        );
    }

    #[test]
    fn parse_channel() {
        let reg = registry();
        assert_eq!(
            reg.parse("/channel 10").unwrap(),
            ParsedCommand { name: "channel".into(), arg: Some("10".into()) }
        );
    }

    #[test]
    fn parse_help() {
        let reg = registry();
        assert_eq!(
            reg.parse("/help").unwrap(),
            ParsedCommand { name: "help".into(), arg: None }
        );
    }

    #[test]
    fn parse_quit() {
        let reg = registry();
        assert_eq!(
            reg.parse("/quit").unwrap(),
            ParsedCommand { name: "quit".into(), arg: None }
        );
    }

    #[test]
    fn parse_quit_alias() {
        let reg = registry();
        assert_eq!(
            reg.parse("/q").unwrap(),
            ParsedCommand { name: "quit".into(), arg: None }
        );
    }

    #[test]
    fn parse_cassette() {
        let reg = registry();
        assert_eq!(
            reg.parse("/cassette").unwrap(),
            ParsedCommand { name: "cassette".into(), arg: None }
        );
    }

    // --- Parsing: error paths ---

    #[test]
    fn parse_unknown_command_returns_error() {
        let reg = registry();
        let err = reg.parse("/foobar").unwrap_err();
        assert!(matches!(err, CommandError::Unknown(_)));
    }

    #[test]
    fn parse_empty_command_returns_error() {
        let reg = registry();
        let err = reg.parse("/").unwrap_err();
        assert!(matches!(err, CommandError::Unknown(_)));
    }

    #[test]
    fn parse_missing_required_arg_bpm() {
        let reg = registry();
        let err = reg.parse("/bpm").unwrap_err();
        assert!(matches!(err, CommandError::MissingArg { ref command } if command == "bpm"));
    }

    // /track no longer requires args (opens modal browser)

    #[test]
    fn parse_missing_required_arg_difficulty() {
        let reg = registry();
        let err = reg.parse("/difficulty").unwrap_err();
        assert!(matches!(err, CommandError::MissingArg { ref command } if command == "difficulty"));
    }

    #[test]
    fn parse_invalid_choice_difficulty() {
        let reg = registry();
        let err = reg.parse("/difficulty extreme").unwrap_err();
        assert!(
            matches!(err, CommandError::InvalidArg { ref command, .. } if command == "difficulty")
        );
    }

    #[test]
    fn parse_invalid_choice_timing() {
        let reg = registry();
        let err = reg.parse("/timing fast").unwrap_err();
        assert!(matches!(err, CommandError::InvalidArg { ref command, .. } if command == "timing"));
    }

    #[test]
    fn parse_invalid_number_bpm() {
        let reg = registry();
        let err = reg.parse("/bpm notanumber").unwrap_err();
        assert!(matches!(err, CommandError::InvalidArg { ref command, .. } if command == "bpm"));
    }

    #[test]
    fn parse_invalid_number_channel() {
        let reg = registry();
        let err = reg.parse("/channel abc").unwrap_err();
        assert!(matches!(err, CommandError::InvalidArg { ref command, .. } if command == "channel"));
    }

    // --- Autocomplete ---

    #[test]
    fn autocomplete_empty_slash_returns_max_8() {
        let reg = registry();
        let suggestions = reg.autocomplete("/");
        assert_eq!(suggestions.len(), 8);
        for s in &suggestions {
            assert!(s.starts_with('/'));
        }
    }

    #[test]
    fn autocomplete_play_prefix() {
        let reg = registry();
        let suggestions = reg.autocomplete("/pl");
        assert_eq!(suggestions, vec!["/play"]);
    }

    #[test]
    fn autocomplete_mute_prefix() {
        let reg = registry();
        let mut suggestions = reg.autocomplete("/mute");
        suggestions.sort();
        assert_eq!(
            suggestions,
            vec!["/mute", "/mute-backtrack", "/mute-kit", "/mute-metronome"]
        );
    }

    #[test]
    fn autocomplete_p_prefix_includes_play_pause_practice() {
        let reg = registry();
        let suggestions = reg.autocomplete("/p");
        assert!(suggestions.contains(&"/play".to_string()));
        assert!(suggestions.contains(&"/pause".to_string()));
        assert!(suggestions.contains(&"/practice".to_string()));
    }

    #[test]
    fn autocomplete_no_leading_slash_returns_empty() {
        let reg = registry();
        assert!(reg.autocomplete("play").is_empty());
    }

    #[test]
    fn autocomplete_no_match_returns_empty() {
        let reg = registry();
        assert!(reg.autocomplete("/zzz").is_empty());
    }

    #[test]
    fn autocomplete_sorted_alphabetically() {
        let reg = registry();
        let suggestions = reg.autocomplete("/");
        let mut sorted = suggestions.clone();
        sorted.sort();
        assert_eq!(suggestions, sorted);
    }

    #[test]
    fn autocomplete_quit_via_alias_prefix() {
        let reg = registry();
        // Alias 'q' starts with 'q', so /quit should appear for /q
        let suggestions = reg.autocomplete("/q");
        assert!(suggestions.contains(&"/quit".to_string()));
    }
}
