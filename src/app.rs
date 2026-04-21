use std::cell::UnsafeCell;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::audio::engine::{AudioEngine, AudioTrack};
use crate::audio::kit::DrumKit;
use crate::audio::metronome::Metronome;
use crate::config::AppConfig;
use crate::data::db::Database;
use crate::engine::playback::PlaybackEngine;
use crate::engine::scoring::{
    NoteResult, ScoreAccumulator, ScoringEngine, TimingPreset,
};
use crate::error::AppError;
use crate::input::command::{CommandError, CommandRegistry};
use crate::input::key_map::KeyMap;
use crate::input::thread::{InputEvent, TimestampedEvent};
use crate::input::vim_mode::{VimAction, VimModeHandler};
use crate::midi::parser;
use crate::midi::types::{Difficulty, DrumNote, DrumPiece, DrumTrack};

use crossterm::event::KeyCode;
use crate::ui::themes::ThemeName;

// Re-export InputMode so UI modules can import it from crate::app
pub use crate::input::vim_mode::InputMode;

/// Top-level application state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    /// First-run name entry.
    Welcome,
    /// Cassette browser for track selection.
    TrackSelect,
    /// Kit selection browser.
    KitSelect,
    /// Theme selection browser.
    ThemeSelect,
    /// Active session with sub-state.
    Session(SessionState),
    /// Viewing scores.
    Scoreboard,
    /// Input latency calibration.
    Calibrating,
    /// Cleanup and exit.
    Quitting,
}

/// Session sub-state. Maps 1:1 to PlaybackEngine's running/paused state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Track loaded, not yet started (position = 0).
    Ready,
    /// Actively playing (PlaybackEngine advancing).
    Playing,
    /// Paused mid-track (PlaybackEngine frozen).
    Paused,
}

/// Audio output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioMode {
    VisualOnly,
    VisualAudio,
}

/// Render-ready snapshot of game state. Produced by the game thread each tick,
/// consumed by the render thread. This is a SNAPSHOT -- the render thread never
/// mutates it.
pub struct GameState {
    // Session
    pub app_state: AppState,
    pub track_name: Option<String>,
    pub difficulty: Difficulty,
    pub timing_preset: TimingPreset,
    pub pieces_used: HashSet<DrumPiece>,

    // Playback
    pub position_ms: f64,
    pub effective_bpm: f64,
    pub time_sig: (u8, u8),
    pub current_bar: u32,
    pub current_beat: f64,
    pub total_bars: u32,
    pub track_duration_ms: f64,

    // Preroll (countdown before playback starts)
    pub preroll_active: bool,
    pub preroll_beats_total: f64,
    pub preroll_beats_elapsed: f64,
    pub preroll_start: Option<Instant>,

    // Loop
    pub loop_active: bool,
    pub loop_start_bar: u32,
    pub loop_end_bar: u32,

    // Free play (drums without active playback)
    pub freeplay_clock_ms: f64,

    // Autoplay
    pub autoplay: bool,

    // Practice mode
    pub practice_mode: bool,
    pub practice_bpm: f64,
    pub practice_target_bpm: f64,
    pub practice_shown_info: bool,

    // Theme selection
    pub theme_list: Vec<(&'static str, ThemeName)>,
    pub theme_selected: usize,

    // Scoring
    pub score_full: ScoreAccumulator,
    pub score_8bar: ScoreAccumulator,
    pub score_16bar: ScoreAccumulator,
    pub score_32bar: ScoreAccumulator,
    pub personal_best: Option<f64>,

    // Score milestone feedback
    pub score_milestone: u8,              // last reached milestone (0, 20, 40, 60, 80, 100)
    pub score_milestone_time: Option<std::time::Instant>, // when milestone was reached

    // Visual feedback (recent events for rendering, last 2 seconds)
    pub recent_results: VecDeque<(f64, NoteResult)>,
    pub metronome_phase: f64,

    // Visible notes for highway rendering
    pub visible_notes: Vec<DrumNote>,

    // Audio state
    pub audio_mode: AudioMode,
    pub mute_metronome: bool,
    pub mute_backtrack: bool,
    pub mute_kit: bool,
    pub mute_all: bool,
    pub metronome_volume: f64,
    pub kit_volume: f64,
    pub backtrack_volume: f64,

    // Input mode
    pub input_mode: InputMode,
    pub console_input: String,
    pub console_cursor: usize,
    pub autocomplete_suggestions: Vec<String>,
    pub autocomplete_selected: Option<usize>,
    pub autocomplete_total: usize, // total matches (may exceed displayed count)

    // Key hints: (DrumPiece, key_label) for showing key bindings in lane headers
    pub key_hints: Vec<(DrumPiece, String)>,

    // UI
    pub theme: ThemeName,
    pub terminal_size: (u16, u16),

    // Status message (shown in console area, auto-fades after 3 seconds)
    pub status_message: Option<(String, std::time::Instant)>,
    // Placeholder hint for command args (shown grayed out in console)
    pub console_placeholder: String,
    // Help overlay visible
    pub help_visible: bool,

    // Welcome screen name entry
    pub welcome_name: String,

    // Track selection
    pub track_list: Vec<String>,
    pub track_selected: usize,
    pub track_search: String,
    pub track_search_active: bool,
    pub track_filtered: Vec<usize>, // indices into track_list

    // Kit selection
    pub kit_list: Vec<String>,
    pub kit_selected: usize,

    // Calibration
    pub calibration_taps: Vec<f64>,      // delta_ms for each tap
    pub calibration_beat: u32,           // current beat number (0-15)
    pub calibration_total_beats: u32,    // 16
    pub calibration_result: Option<f64>, // median offset once computed
    pub calibration_phase: f64,          // 0.0-1.0 for visual metronome at 100 BPM
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            app_state: AppState::Welcome,
            track_name: None,
            difficulty: Difficulty::Hard,
            timing_preset: TimingPreset::Standard,
            pieces_used: HashSet::new(),

            position_ms: 0.0,
            effective_bpm: 120.0,
            time_sig: (4, 4),
            current_bar: 0,
            current_beat: 0.0,
            total_bars: 0,
            track_duration_ms: 0.0,

            preroll_active: false,
            preroll_beats_total: 0.0,
            preroll_beats_elapsed: 0.0,
            preroll_start: None,

            loop_active: false,
            loop_start_bar: 0,
            loop_end_bar: 0,

            freeplay_clock_ms: 0.0,

            autoplay: false,

            practice_mode: false,
            practice_bpm: 0.0,
            practice_target_bpm: 0.0,
            practice_shown_info: false,

            theme_list: vec![
                ("Gruvbox", ThemeName::Gruvbox),
                ("Desert", ThemeName::Desert),
                ("Evening", ThemeName::Evening),
                ("Slate", ThemeName::Slate),
                ("Blue", ThemeName::Blue),
                ("Pablo", ThemeName::Pablo),
                ("Quiet", ThemeName::Quiet),
                ("Shine", ThemeName::Shine),
                ("Run", ThemeName::Run),
            ],
            theme_selected: 0,

            score_full: ScoreAccumulator::default(),
            score_8bar: ScoreAccumulator::default(),
            score_16bar: ScoreAccumulator::default(),
            score_32bar: ScoreAccumulator::default(),
            personal_best: None,

            score_milestone: 0,
            score_milestone_time: None,

            recent_results: VecDeque::new(),
            metronome_phase: 0.0,

            visible_notes: Vec::new(),

            audio_mode: AudioMode::VisualOnly,
            mute_metronome: false,
            mute_backtrack: false,
            mute_kit: false,
            mute_all: false,
            metronome_volume: 0.7,
            kit_volume: 1.0,
            backtrack_volume: 0.8,

            input_mode: InputMode::Normal,
            console_input: String::new(),
            console_cursor: 0,
            autocomplete_suggestions: Vec::new(),
            autocomplete_selected: None,
            autocomplete_total: 0,

            key_hints: Vec::new(),

            theme: ThemeName::default(),
            terminal_size: (80, 24),

            status_message: None,
            console_placeholder: String::new(),
            help_visible: false,

            welcome_name: String::new(),

            track_list: Vec::new(),
            track_selected: 0,
            track_search: String::new(),
            track_search_active: false,
            track_filtered: Vec::new(),

            kit_list: Vec::new(),
            kit_selected: 0,

            calibration_taps: Vec::new(),
            calibration_beat: 0,
            calibration_total_beats: 16,
            calibration_result: None,
            calibration_phase: 0.0,
        }
    }
}

/// Lock-free double buffer for sharing GameState between game and render threads.
///
/// The game thread writes into the inactive slot, then atomically swaps the read index.
/// The render thread reads from the slot indicated by read_idx.
pub struct SwapBuffer {
    pub slots: [UnsafeCell<GameState>; 2],
    /// Index of the slot the render thread should read (0 or 1).
    pub read_idx: AtomicUsize,
}

// SAFETY: Only the game thread writes (to the non-read slot), and only the render
// thread reads (from the read slot). The AtomicUsize synchronizes visibility.
unsafe impl Sync for SwapBuffer {}
unsafe impl Send for SwapBuffer {}

/// CLI arguments parsed by clap in main.rs, passed to run().
pub struct CliArgs {
    pub path: Option<String>,
    pub bpm: Option<f64>,
    pub kit: Option<String>,
    pub theme: Option<String>,
    pub visual_only: bool,
    pub config: Option<String>,
}

/// Mutable session context that lives alongside GameState.
/// Holds engines and state that the render thread never sees directly.
struct SessionContext {
    playback: Option<PlaybackEngine>,
    scoring: Option<ScoringEngine>,
    vim: VimModeHandler,
    key_map: KeyMap,
    command_registry: CommandRegistry,
    db: Database,
    config: AppConfig,
    profile_id: Option<i64>,
    discovered_tracks: Vec<crate::data::track_bundle::TrackInfo>,
    discovered_kits: Vec<crate::data::kit_bundle::KitInfo>,
    audio_engine: Option<AudioEngine>,
    metronome: Option<Metronome>,
    /// State to return to after calibration completes or is cancelled.
    pre_calibration_state: Option<AppState>,
    /// When calibration started (used to compute beat phases and offsets).
    calibration_start: Option<Instant>,
    /// Timestamp of the last expected beat boundary during calibration.
    calibration_last_beat_instant: Option<Instant>,
    /// Which absolute beat index was last triggered during calibration.
    calibration_last_beat_idx: Option<u64>,
    /// When calibration result was set (to show it for 2 seconds).
    calibration_result_time: Option<Instant>,
    /// Config path so we can save offset after calibration.
    config_path: PathBuf,
}

/// Main application entry point.
pub fn run_with_args(args: CliArgs) -> Result<(), AppError> {
    // 1. Load config
    let config_path = args
        .config
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(AppConfig::default_config_path);
    let config = AppConfig::load(&config_path)?;

    // 2. Ensure directories exist
    AppConfig::ensure_dirs()?;

    // 3. Open SQLite database
    let db_path = AppConfig::default_db_path();
    let db = Database::open(&db_path)?;

    // 4. Check profile
    let profile = db.get_profile()?;

    // 5. Resolve theme from CLI > config > default
    let theme = resolve_theme(args.theme.as_deref(), &config);

    // 6. Build initial game state
    let mut game_state = GameState::default();
    game_state.theme = theme;
    game_state.audio_mode = if args.visual_only {
        AudioMode::VisualOnly
    } else {
        AudioMode::VisualAudio
    };
    game_state.metronome_volume = config.audio.metronome_volume;
    game_state.kit_volume = config.audio.kit_volume;
    game_state.backtrack_volume = config.audio.backtrack_volume;

    let profile_id = if let Some(ref p) = profile {
        game_state.app_state = AppState::TrackSelect;
        Some(p.id)
    } else if args.path.is_some() {
        // CLI path provided but no profile — auto-create a default one
        // so scores can be saved without requiring the Welcome screen
        let p = db.create_profile("Player")?;
        game_state.app_state = AppState::TrackSelect;
        Some(p.id)
    } else {
        game_state.app_state = AppState::Welcome;
        None
    };

    // If a MIDI path was given, try to load it immediately
    let mut playback: Option<PlaybackEngine> = None;
    let mut scoring: Option<ScoringEngine> = None;

    if let Some(ref midi_path) = args.path {
        match parser::parse_midi_file(std::path::Path::new(midi_path)) {
            Ok(mut track) => {
                // Try to get name and default_bpm from meta.toml in the same directory
                let midi_p = std::path::Path::new(midi_path);
                let mut meta_bpm: Option<f64> = None;
                if let Some(parent) = midi_p.parent() {
                    let meta_path = parent.join("meta.toml");
                    if let Ok(meta_str) = std::fs::read_to_string(&meta_path) {
                        if let Ok(meta) = toml::from_str::<toml::Value>(&meta_str) {
                            if let Some(name) = meta
                                .get("track")
                                .and_then(|t| t.get("name"))
                                .and_then(|n| n.as_str())
                            {
                                track.name = name.to_string();
                            }
                            if let Some(bpm) = meta
                                .get("track")
                                .and_then(|t| t.get("default_bpm"))
                                .and_then(|b| b.as_integer())
                            {
                                if bpm > 0 {
                                    meta_bpm = Some(bpm as f64);
                                }
                            }
                        }
                    }
                }
                let windows = game_state.timing_preset.windows();
                let note_count = track.notes.len();
                game_state.track_name = Some(track.name.clone());
                game_state.total_bars = track.total_bars;
                game_state.track_duration_ms = track.duration_ms;
                game_state.pieces_used = track.pieces_used.clone();
                if let Some(ts) = track.time_signatures.first() {
                    game_state.time_sig = (ts.numerator, ts.denominator);
                }
                let mut engine = PlaybackEngine::new(track);
                game_state.effective_bpm = engine.original_bpm;
                // Apply BPM: CLI flag > meta.toml > MIDI auto-detect
                if let Some(bpm) = args.bpm {
                    engine.set_bpm(bpm);
                    game_state.effective_bpm = bpm;
                } else if let Some(bpm) = meta_bpm {
                    engine.set_bpm(bpm);
                    game_state.effective_bpm = bpm;
                }
                playback = Some(engine);
                scoring = Some(ScoringEngine::new(windows, note_count));
                game_state.app_state = AppState::Session(SessionState::Ready);
            }
            Err(_e) => {
                // Fall through to track select; the error will be visible
            }
        }
    }

    // Apply BPM override if we have a playback engine
    if let (Some(bpm), Some(ref mut pb)) = (args.bpm, &mut playback) {
        pb.set_bpm(bpm);
    }

    let key_map = KeyMap::split_preset();
    game_state.key_hints = build_key_hints(&key_map);
    let command_registry = CommandRegistry::new();
    let vim = VimModeHandler::new();

    // Initialize audio engine
    let mut audio_engine = if !args.visual_only {
        match AudioEngine::new() {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("Warning: Audio init failed: {e}. Running in visual-only mode.");
                None
            }
        }
    } else {
        None
    };

    // Load placeholder drum kit
    if let Some(ref mut audio) = audio_engine {
        let kit_path = PathBuf::from("assets/kits/placeholder");
        match DrumKit::load(&kit_path) {
            Ok(kit) => {
                audio.load_kit_samples(kit.samples);
            }
            Err(e) => eprintln!("Warning: Kit load failed: {e}"),
        }
    }

    // Initialize metronome
    let metronome = {
        let hi_path = PathBuf::from("assets/metronome/click_hi.wav");
        let lo_path = PathBuf::from("assets/metronome/click_lo.wav");
        if hi_path.exists() && lo_path.exists() {
            match Metronome::new(&hi_path, &lo_path) {
                Ok(m) => Some(m),
                Err(e) => {
                    eprintln!("Warning: Metronome init failed: {e}");
                    None
                }
            }
        } else {
            None
        }
    };

    let mut ctx = SessionContext {
        playback,
        scoring,
        vim,
        key_map,
        command_registry,
        db,
        config,
        profile_id,
        discovered_tracks: Vec::new(),
        discovered_kits: Vec::new(),
        audio_engine,
        metronome,
        pre_calibration_state: None,
        calibration_start: None,
        calibration_last_beat_instant: None,
        calibration_last_beat_idx: None,
        calibration_result_time: None,
        config_path: config_path.clone(),
    };

    // Pre-discover tracks if starting in TrackSelect
    if game_state.app_state == AppState::TrackSelect {
        ensure_tracks_discovered(&mut game_state, &mut ctx);
    }

    // 7. Enable terminal raw mode and alternate screen
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;
    crossterm::terminal::enable_raw_mode()?;

    // 8. Create terminal
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Get initial terminal size
    if let Ok(size) = crossterm::terminal::size() {
        game_state.terminal_size = size;
    }

    // 9. Spawn input thread
    let (tx, rx) = crossbeam_channel::unbounded();
    let shutdown = Arc::new(AtomicBool::new(false));
    let input_handle =
        crate::input::thread::spawn_input_thread(tx, shutdown.clone());

    // 10. Main loop at 60 FPS (Phase 1: single-threaded approach)
    #[allow(deprecated)]
    let mut loop_helper = spin_sleep::LoopHelper::builder()
        .report_interval_s(1.0)
        .build_with_target_rate(60.0);

    loop {
        loop_helper.loop_start();

        // Drain input events
        while let Ok(event) = rx.try_recv() {
            handle_input(&mut game_state, &mut ctx, event)?;
        }

        // Check quit
        if game_state.app_state == AppState::Quitting {
            break;
        }

        // Update game state (tick playback, check misses, etc.)
        update_game_state(&mut game_state, &mut ctx);

        // Render
        terminal.draw(|frame| {
            crate::ui::render(frame, &game_state);
        })?;

        loop_helper.loop_sleep();
    }

    // 11. Cleanup
    shutdown.store(true, Ordering::Relaxed);
    input_handle.join().ok();
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    Ok(())
}

/// Main application entry point (no-arg wrapper for backward compatibility).
pub fn run() -> Result<(), AppError> {
    run_with_args(CliArgs {
        path: None,
        bpm: None,
        kit: None,
        theme: None,
        visual_only: false,
        config: None,
    })
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

fn handle_input(
    state: &mut GameState,
    ctx: &mut SessionContext,
    event: TimestampedEvent,
) -> Result<(), AppError> {
    match event.event {
        InputEvent::Resize(cols, rows) => {
            state.terminal_size = (cols, rows);
        }
        InputEvent::Key(key_event) => {
            // Special handling for Welcome screen name entry
            if state.app_state == AppState::Welcome {
                handle_welcome_input(state, ctx, key_event)?;
                return Ok(());
            }

            // Special handling for Scoreboard overlay
            if state.app_state == AppState::Scoreboard {
                handle_scoreboard_input(state, ctx, key_event);
                return Ok(());
            }

            // Special handling for Help overlay
            if state.help_visible {
                match key_event.code {
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Char('q') => {
                        state.help_visible = false;
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Special handling for TrackSelect (cassette browser)
            if state.app_state == AppState::TrackSelect {
                handle_track_select_input(state, ctx, key_event)?;
                return Ok(());
            }

            // Special handling for KitSelect browser
            if state.app_state == AppState::KitSelect {
                handle_kit_select_input(state, ctx, key_event)?;
                return Ok(());
            }

            // Special handling for ThemeSelect browser
            if state.app_state == AppState::ThemeSelect {
                handle_theme_select_input(state, ctx, key_event);
                return Ok(());
            }

            // Special handling for Calibrating state
            if state.app_state == AppState::Calibrating {
                handle_calibration_input(state, ctx, key_event);
                return Ok(());
            }

            let action = ctx.vim.handle_key(key_event, &ctx.key_map);
            match action {
                VimAction::QuitRequest => {
                    state.app_state = AppState::Quitting;
                }
                VimAction::DrumHit(piece) => {
                    handle_drum_hit(state, ctx, piece, event.instant);
                }
                VimAction::EnterCommand => {
                    state.input_mode = ctx.vim.mode;
                    state.console_input = ctx.vim.console_input.clone();
                    state.console_cursor = ctx.vim.console_cursor;
                    update_autocomplete(state, ctx);
                }
                VimAction::ExitCommand => {
                    state.input_mode = InputMode::Normal;
                    state.console_input.clear();
                    state.console_cursor = 0;
                    state.autocomplete_suggestions.clear();
                    state.autocomplete_selected = None;
                }
                VimAction::ExecuteCommand(cmd_str) => {
                    // If autocomplete is active with a selection, accept it
                    let effective_cmd = if let Some(idx) = state.autocomplete_selected {
                        if let Some(suggestion) = state.autocomplete_suggestions.get(idx).cloned() {
                            suggestion
                        } else {
                            cmd_str
                        }
                    } else {
                        cmd_str
                    };

                    // If the command needs args and none were provided, fill input
                    // and stay in command mode so the user can type the argument.
                    let needs_fill = if state.autocomplete_selected.is_some() {
                        matches!(
                            ctx.command_registry.parse(&effective_cmd),
                            Err(CommandError::MissingArg { .. })
                        )
                    } else {
                        false
                    };

                    if needs_fill {
                        let hint = ctx
                            .command_registry
                            .arg_hint(&effective_cmd)
                            .unwrap_or_default();
                        let filled = format!("{} ", effective_cmd);
                        // Restore vim to Command mode (exit_command already fired)
                        ctx.vim.mode = InputMode::Command;
                        ctx.vim.console_input = filled.clone();
                        ctx.vim.console_cursor = filled.len();
                        state.input_mode = InputMode::Command;
                        state.console_input = filled;
                        state.console_cursor = ctx.vim.console_cursor;
                        state.autocomplete_suggestions.clear();
                        state.autocomplete_selected = None;
                        state.console_placeholder = hint;
                    } else {
                        state.input_mode = InputMode::Normal;
                        state.console_input.clear();
                        state.console_cursor = 0;
                        state.autocomplete_suggestions.clear();
                        state.autocomplete_selected = None;
                        state.console_placeholder.clear();
                        execute_command(state, ctx, &effective_cmd)?;
                    }
                }
                VimAction::AppendChar(_) | VimAction::Backspace => {
                    state.console_input = ctx.vim.console_input.clone();
                    state.console_cursor = ctx.vim.console_cursor;
                    state.console_placeholder.clear();
                    update_autocomplete(state, ctx);
                }
                VimAction::AcceptAutocomplete => {
                    // No longer produced by vim_mode (Tab → AutocompleteDown),
                    // but kept for completeness.
                }
                VimAction::AutocompleteDown => {
                    if !state.autocomplete_suggestions.is_empty() {
                        state.autocomplete_selected = Some(
                            state
                                .autocomplete_selected
                                .map(|i| {
                                    (i + 1)
                                        % state.autocomplete_suggestions.len()
                                })
                                .unwrap_or(0),
                        );
                        update_placeholder_for_selection(state, ctx);
                    }
                }
                VimAction::AutocompleteUp => {
                    if !state.autocomplete_suggestions.is_empty() {
                        let len = state.autocomplete_suggestions.len();
                        state.autocomplete_selected = Some(
                            state
                                .autocomplete_selected
                                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                                .unwrap_or(len - 1),
                        );
                        update_placeholder_for_selection(state, ctx);
                    }
                }
                VimAction::None => {}
            }
        }
    }
    Ok(())
}

/// Handle key input during the Welcome screen (name entry).
fn handle_welcome_input(
    state: &mut GameState,
    ctx: &mut SessionContext,
    key: crossterm::event::KeyEvent,
) -> Result<(), AppError> {
    use crossterm::event::{KeyCode, KeyModifiers};

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c') => {
                state.app_state = AppState::Quitting;
                return Ok(());
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Enter => {
            let name = state.welcome_name.trim().to_string();
            if !name.is_empty() {
                let profile = ctx.db.create_profile(&name)?;
                ctx.profile_id = Some(profile.id);
                state.welcome_name.clear();
                open_track_select(state, ctx);
            }
        }
        KeyCode::Backspace => {
            state.welcome_name.pop();
        }
        KeyCode::Char(c) => {
            if state.welcome_name.len() < 32 {
                state.welcome_name.push(c);
            }
        }
        KeyCode::Esc => {
            state.app_state = AppState::Quitting;
        }
        _ => {}
    }
    Ok(())
}

/// Handle key input on the Scoreboard overlay.
fn handle_scoreboard_input(
    state: &mut GameState,
    ctx: &mut SessionContext,
    key: crossterm::event::KeyEvent,
) {
    use crossterm::event::{KeyCode, KeyModifiers};

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c') => {
                state.app_state = AppState::Quitting;
                return;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            // Return to session (Ready) if a track is loaded, else track select
            if ctx.playback.is_some() {
                state.app_state = AppState::Session(SessionState::Ready);
            } else {
                open_track_select(state, ctx);
            }
        }
        KeyCode::Char('r') => {
            // Replay with preroll
            if ctx.playback.is_some() {
                if let Some(ref mut sc) = ctx.scoring {
                    sc.reset();
                }
                if let Some(ref mut metro) = ctx.metronome {
                    metro.reset();
                }
                state.score_full = ScoreAccumulator::default();
                state.score_milestone = 0;
                state.score_milestone_time = None;
                state.recent_results.clear();
                start_preroll(state, ctx);
            }
        }
        _ => {}
    }
}

/// Rebuild the filtered track indices from the current search query.
fn update_track_filter(state: &mut GameState) {
    let query = state.track_search.to_lowercase();
    if query.is_empty() {
        state.track_filtered = (0..state.track_list.len()).collect();
    } else {
        state.track_filtered = state
            .track_list
            .iter()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
    }
    // Clamp selection
    if state.track_filtered.is_empty() {
        state.track_selected = 0;
    } else if state.track_selected >= state.track_filtered.len() {
        state.track_selected = state.track_filtered.len() - 1;
    }
}

/// Handle key input on the TrackSelect (cassette browser) overlay.
fn handle_track_select_input(
    state: &mut GameState,
    ctx: &mut SessionContext,
    key: crossterm::event::KeyEvent,
) -> Result<(), AppError> {
    use crossterm::event::{KeyCode, KeyModifiers};

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c') => {
                state.app_state = AppState::Quitting;
                return Ok(());
            }
            _ => {}
        }
    }

    ensure_tracks_discovered(state, ctx);

    // Search mode: typing into the filter
    if state.track_search_active {
        match key.code {
            KeyCode::Esc => {
                state.track_search_active = false;
                state.track_search.clear();
                update_track_filter(state);
            }
            KeyCode::Enter => {
                state.track_search_active = false;
                // Keep the filter applied
            }
            KeyCode::Backspace => {
                state.track_search.pop();
                update_track_filter(state);
            }
            KeyCode::Char(c) => {
                state.track_search.push(c);
                update_track_filter(state);
            }
            _ => {}
        }
        return Ok(());
    }

    // Normal navigation mode
    let list_len = state.track_filtered.len();

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.track_search.clear();
            state.track_search_active = false;
            if ctx.playback.is_some() {
                state.app_state = AppState::Session(SessionState::Ready);
            } else {
                state.app_state = AppState::Quitting;
            }
        }
        KeyCode::Char('/') => {
            state.track_search_active = true;
            state.track_search.clear();
            update_track_filter(state);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if list_len > 0 {
                state.track_selected = (state.track_selected + 1) % list_len;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if list_len > 0 {
                state.track_selected = if state.track_selected == 0 {
                    list_len - 1
                } else {
                    state.track_selected - 1
                };
            }
        }
        KeyCode::Enter => {
            if let Some(&real_idx) = state.track_filtered.get(state.track_selected) {
                if let Some(track_info) = ctx.discovered_tracks.get(real_idx) {
                    let bpm_override = if track_info.default_bpm > 0 {
                        Some(track_info.default_bpm as f64)
                    } else {
                        None
                    };
                    match parser::parse_midi_file(&track_info.midi_path) {
                        Ok(mut track) => {
                            track.name = track_info.name.clone();
                            state.track_search.clear();
                            state.track_search_active = false;
                            load_track_with_bpm(state, ctx, track, bpm_override);
                        }
                        Err(_) => {}
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle key input on the KitSelect browser overlay.
fn handle_kit_select_input(
    state: &mut GameState,
    ctx: &mut SessionContext,
    key: crossterm::event::KeyEvent,
) -> Result<(), AppError> {
    use crossterm::event::{KeyCode, KeyModifiers};

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c') => {
                state.app_state = AppState::Quitting;
                return Ok(());
            }
            _ => {}
        }
    }

    ensure_kits_discovered(state, ctx);

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            if ctx.playback.is_some() {
                state.app_state = AppState::Session(SessionState::Ready);
            } else {
                open_track_select(state, ctx);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !state.kit_list.is_empty() {
                state.kit_selected =
                    (state.kit_selected + 1) % state.kit_list.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !state.kit_list.is_empty() {
                state.kit_selected = if state.kit_selected == 0 {
                    state.kit_list.len() - 1
                } else {
                    state.kit_selected - 1
                };
            }
        }
        KeyCode::Enter => {
            if let Some(kit_info) =
                ctx.discovered_kits.get(state.kit_selected)
            {
                // Load the selected kit into the audio engine
                if let Some(ref mut audio) = ctx.audio_engine {
                    let _ = audio.load_kit(&kit_info.samples);
                }
                state.status_message =
                    Some((format!("Kit: {}", kit_info.name), std::time::Instant::now()));
                if ctx.playback.is_some() {
                    state.app_state = AppState::Session(SessionState::Ready);
                } else {
                    open_track_select(state, ctx);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle key input on the ThemeSelect browser overlay.
fn handle_theme_select_input(
    state: &mut GameState,
    _ctx: &mut SessionContext,
    key: crossterm::event::KeyEvent,
) {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.app_state = AppState::Session(SessionState::Ready);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !state.theme_list.is_empty() {
                state.theme_selected =
                    (state.theme_selected + 1) % state.theme_list.len();
                // Live preview
                state.theme = state.theme_list[state.theme_selected].1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !state.theme_list.is_empty() {
                state.theme_selected = if state.theme_selected == 0 {
                    state.theme_list.len() - 1
                } else {
                    state.theme_selected - 1
                };
                state.theme = state.theme_list[state.theme_selected].1;
            }
        }
        KeyCode::Enter => {
            if let Some(&(_, theme_name)) = state.theme_list.get(state.theme_selected) {
                state.theme = theme_name;
            }
            state.app_state = AppState::Session(SessionState::Ready);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Calibration helpers
// ---------------------------------------------------------------------------

/// Calibration beat interval at 100 BPM (ms).
const CALIB_BEAT_MS: f64 = 600.0; // 60_000 / 100

/// Handle key input during the Calibration state.
fn handle_calibration_input(
    state: &mut GameState,
    ctx: &mut SessionContext,
    key: crossterm::event::KeyEvent,
) {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Allow quitting even during calibration
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c') => {
                state.app_state = AppState::Quitting;
                return;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc => {
            // Cancel calibration -- restore previous state without saving
            let prev = ctx
                .pre_calibration_state
                .take()
                .unwrap_or(AppState::TrackSelect);
            state.app_state = prev;
            ctx.calibration_start = None;
            ctx.calibration_last_beat_instant = None;
            ctx.calibration_last_beat_idx = None;
            ctx.calibration_result_time = None;
        }
        _ => {
            // Any other key = tap
            // Only record if we haven't finished yet
            if state.calibration_beat < state.calibration_total_beats
                && state.calibration_result.is_none()
            {
                if let Some(start) = ctx.calibration_start {
                    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

                    // Determine the expected beat time for the current beat
                    // Beats are numbered 0..15; beat N is expected at N * CALIB_BEAT_MS
                    let expected_ms =
                        state.calibration_beat as f64 * CALIB_BEAT_MS;

                    // Compute signed delta: negative means early, positive means late
                    let delta_ms = elapsed_ms - expected_ms;

                    state.calibration_taps.push(delta_ms);
                    state.calibration_beat += 1;

                    if state.calibration_beat >= state.calibration_total_beats {
                        // All 16 taps collected -- compute median of taps 5-16 (indices 4..16)
                        let mut data: Vec<f64> = state
                            .calibration_taps
                            .iter()
                            .skip(4) // discard first 4 warm-up taps
                            .copied()
                            .collect();

                        let offset = median(&mut data);
                        state.calibration_result = Some(offset);
                        ctx.calibration_result_time = Some(Instant::now());

                        // Save to config
                        ctx.config.audio.input_offset_ms = offset.round() as i32;
                        let _ = ctx.config.save(&ctx.config_path);
                    }
                }
            }
        }
    }
}

/// Compute median of a slice, sorting in place.
fn median(values: &mut Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = values.len();
    if len == 0 {
        return 0.0;
    }
    if len % 2 == 0 {
        (values[len / 2 - 1] + values[len / 2]) / 2.0
    } else {
        values[len / 2]
    }
}

/// Simple ~ expansion for paths.
/// Discover tracks and populate track_list if empty.
/// Open the track selection modal with a fresh (unfiltered) state.
fn open_track_select(state: &mut GameState, ctx: &mut SessionContext) {
    ensure_tracks_discovered(state, ctx);
    state.track_search.clear();
    state.track_search_active = false;
    state.track_selected = 0;
    update_track_filter(state);
    state.app_state = AppState::TrackSelect;
}

fn ensure_tracks_discovered(state: &mut GameState, ctx: &mut SessionContext) {
    if state.track_list.is_empty() {
        let track_dirs = vec![
            PathBuf::from("assets/tracks"),
            PathBuf::from(shellexpand_simple(&ctx.config.paths.tracks_dir)),
        ];
        let tracks = crate::data::track_bundle::discover_tracks(&track_dirs);
        state.track_list = tracks.iter().map(|t| t.name.clone()).collect();
        ctx.discovered_tracks = tracks;
        // Initialize filter to show all tracks
        update_track_filter(state);
    } else if state.track_filtered.is_empty() && !state.track_list.is_empty() {
        update_track_filter(state);
    }
}

/// Discover kits and populate kit_list if empty.
fn ensure_kits_discovered(state: &mut GameState, ctx: &mut SessionContext) {
    if state.kit_list.is_empty() {
        let kit_dirs = vec![
            PathBuf::from("assets/kits"),
            PathBuf::from(shellexpand_simple(&ctx.config.paths.kits_dir)),
        ];
        let kits = crate::data::kit_bundle::discover_kits(&kit_dirs);
        state.kit_list = kits.iter().map(|k| k.name.clone()).collect();
        ctx.discovered_kits = kits;
    }
}

fn shellexpand_simple(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

/// Handle a drum hit from the player.
fn handle_drum_hit(
    state: &mut GameState,
    ctx: &mut SessionContext,
    piece: DrumPiece,
    _instant: std::time::Instant,
) {
    if let AppState::Session(SessionState::Playing) = &state.app_state {
        // During preroll: play sound but skip scoring
        if state.preroll_active {
            if let Some(ref mut audio) = ctx.audio_engine {
                if state.audio_mode == AudioMode::VisualAudio
                    && !state.mute_kit
                    && !state.mute_all
                {
                    let _ = audio.trigger_hit(piece, 100);
                }
            }
            return;
        }
        // Use matched note's velocity for audio, default 100 for extras
        let mut hit_velocity: u8 = 100;

        if let (Some(ref playback), Some(ref mut scoring)) =
            (&ctx.playback, &mut ctx.scoring)
        {
            let (hittable, offset) =
                playback.hittable_notes(scoring.timing_windows.ok_ms);
            let result =
                scoring.process_hit(piece, playback.position_ms, hittable, offset);

            // Extract matched note velocity for audio
            hit_velocity = match &result {
                NoteResult::Hit { note, .. } => note.velocity,
                NoteResult::WrongPiece { expected, .. } => expected.velocity,
                _ => 100, // Extra hits use default
            };

            // Push to recent results for visual feedback
            state
                .recent_results
                .push_back((playback.position_ms, result));

            // Keep only the last 2 seconds of results
            let cutoff = playback.position_ms - 2000.0;
            while let Some(&(t, _)) = state.recent_results.front() {
                if t < cutoff {
                    state.recent_results.pop_front();
                } else {
                    break;
                }
            }

            // Update score snapshot
            state.score_full = scoring.score_full.clone();
            state.score_8bar = scoring.rolling_8bar.summarize();
            state.score_16bar = scoring.rolling_16bar.summarize();
            state.score_32bar = scoring.rolling_32bar.summarize();
        }

        // Trigger audio with the matched note's velocity
        if let Some(ref mut audio) = ctx.audio_engine {
            if state.audio_mode == AudioMode::VisualAudio
                && !state.mute_kit
                && !state.mute_all
            {
                let _ = audio.trigger_hit(piece, hit_velocity);
            }
        }
        return;
    }

    // Free play: not in Playing state — just trigger sound + circle animation
    if let Some(ref mut audio) = ctx.audio_engine {
        if state.audio_mode == AudioMode::VisualAudio
            && !state.mute_kit
            && !state.mute_all
        {
            let _ = audio.trigger_hit(piece, 100);
        }
    }
    // Push a visual-only result so the lane circle flashes.
    // Use freeplay_clock_ms so flashes expire correctly even when position_ms
    // is frozen (Ready/Paused).
    state.freeplay_clock_ms += 200.0;
    let ts = state.freeplay_clock_ms;
    state
        .recent_results
        .push_back((ts, NoteResult::Extra { piece, time_ms: ts }));
}

/// Update autocomplete suggestions based on current console input.
fn update_autocomplete(state: &mut GameState, ctx: &SessionContext) {
    let (suggestions, total) =
        ctx.command_registry.autocomplete_with_count(&state.console_input);
    state.autocomplete_suggestions = suggestions;
    state.autocomplete_total = total;
    state.autocomplete_selected = if state.autocomplete_suggestions.is_empty() {
        None
    } else {
        Some(0)
    };

    // Show placeholder hint for typed commands (e.g. "/bpm " → "<number>")
    // Check if input looks like a complete command name followed by a space
    let input = state.console_input.trim();
    if input.contains(' ') {
        // Already has a space — check if the command before space needs args
        let cmd_part = input.split_whitespace().next().unwrap_or("");
        if let Some(hint) = ctx.command_registry.arg_hint(cmd_part) {
            // Only show if no arg has been typed yet
            let after_space = input[cmd_part.len()..].trim();
            if after_space.is_empty() {
                state.console_placeholder = hint;
            } else {
                state.console_placeholder.clear();
            }
        } else {
            state.console_placeholder.clear();
        }
    } else {
        state.console_placeholder.clear();
    }
}

/// Update placeholder hint based on the currently selected autocomplete suggestion.
fn update_placeholder_for_selection(state: &mut GameState, ctx: &SessionContext) {
    if let Some(idx) = state.autocomplete_selected {
        if let Some(suggestion) = state.autocomplete_suggestions.get(idx) {
            if let Some(hint) = ctx.command_registry.arg_hint(suggestion) {
                state.console_placeholder = hint;
            } else {
                state.console_placeholder.clear();
            }
        }
    }
}

/// Start playback with a 1-bar preroll. The playback engine starts at a
/// negative offset so notes scroll into view during the countdown.
fn start_preroll(state: &mut GameState, ctx: &mut SessionContext) {
    let (ts_num, _) = state.time_sig;

    state.preroll_active = true;
    state.preroll_beats_total = ts_num as f64;
    state.preroll_beats_elapsed = 0.0;
    state.preroll_start = Some(Instant::now());

    // Compute preroll offset in the original track timeline.
    // Beat duration depends on time sig denominator (/8 = eighth note beats).
    if let Some(ref mut pb) = ctx.playback {
        let (_, ts_denom) = state.time_sig;
        let quarter_ms = 60_000.0 / pb.original_bpm.max(1.0);
        let beat_ms = quarter_ms * 4.0 / ts_denom as f64;
        let preroll_ms = ts_num as f64 * beat_ms;
        pb.pause_offset_ms = -preroll_ms;
        pb.position_ms = -preroll_ms;
        pb.note_index = 0;
        pb.playing = true;
        pb.start_instant = Instant::now();
    }

    state.app_state = AppState::Session(SessionState::Playing);
}

/// Execute a parsed slash command.
fn execute_command(
    state: &mut GameState,
    ctx: &mut SessionContext,
    cmd_str: &str,
) -> Result<(), AppError> {
    let parsed = match ctx.command_registry.parse(cmd_str) {
        Ok(p) => p,
        Err(e) => {
            state.status_message =
                Some((format!("Error: {}", e), std::time::Instant::now()));
            return Ok(());
        }
    };

    match parsed.name.as_str() {
        "quit" | "q" => {
            state.app_state = AppState::Quitting;
        }
        "play" | "p" => {
            if ctx.playback.is_some() {
                // Reset scoring when starting from Ready/Paused
                // (handles post-calibration and post-completion states)
                if let Some(ref mut sc) = ctx.scoring {
                    sc.reset();
                    // Re-apply difficulty filter
                    if state.difficulty != Difficulty::Hard {
                        if let Some(ref pb) = ctx.playback {
                            let diff = state.difficulty;
                            sc.reset_for_difficulty(&pb.track.notes, |n| {
                                diff.includes_note(n, n.beat < 1.0)
                            });
                        }
                    }
                }
                state.score_full = ScoreAccumulator::default();
                state.score_milestone = 0;
                state.score_milestone_time = None;
                state.recent_results.clear();
                start_preroll(state, ctx);
            }
        }
        "pause" => {
            if let Some(ref mut pb) = ctx.playback {
                pb.pause();
                state.preroll_active = false;
                state.app_state = AppState::Session(SessionState::Paused);
            }
        }
        "replay" | "r" => {
            if ctx.playback.is_some() {
                if let Some(ref mut sc) = ctx.scoring {
                    sc.reset();
                }
                if let Some(ref mut metro) = ctx.metronome {
                    metro.reset();
                }
                state.score_full = ScoreAccumulator::default();
                state.score_milestone = 0;
                state.score_milestone_time = None;
                state.recent_results.clear();
                // start_preroll sets position to -1bar and starts playback
                start_preroll(state, ctx);
            }
        }
        "bpm" => {
            if let Some(ref arg) = parsed.arg {
                if let Ok(bpm) = arg.parse::<f64>() {
                    if let Some(ref mut pb) = ctx.playback {
                        pb.set_bpm(bpm);
                        state.effective_bpm = bpm;
                    }
                    state.recent_results.clear();
                }
            }
        }
        "difficulty" => {
            if let Some(ref arg) = parsed.arg {
                state.difficulty = match arg.as_str() {
                    "easy" => Difficulty::Easy,
                    "medium" => Difficulty::Medium,
                    _ => Difficulty::Hard,
                };
                // Recompute pieces_used and reset scoring for new difficulty
                if let Some(ref pb) = ctx.playback {
                    let diff = state.difficulty;
                    state.pieces_used = pb
                        .track
                        .notes
                        .iter()
                        .filter(|n| diff.includes_note(n, n.beat < 1.0))
                        .map(|n| n.piece)
                        .collect();
                    // Reset scoring: excluded notes are pre-judged so they
                    // won't count as misses or match as hits
                    if let Some(ref mut scoring) = ctx.scoring {
                        scoring.reset_for_difficulty(&pb.track.notes, |n| {
                            diff.includes_note(n, n.beat < 1.0)
                        });
                    }
                    state.score_full = ScoreAccumulator::default();
                    state.recent_results.clear();
                }
            }
        }
        "timing" => {
            if let Some(ref arg) = parsed.arg {
                state.timing_preset = match arg.as_str() {
                    "relaxed" => TimingPreset::Relaxed,
                    "strict" => TimingPreset::Strict,
                    _ => TimingPreset::Standard,
                };
            }
        }
        "theme" => {
            // Find current theme index for pre-selection
            state.theme_selected = state
                .theme_list
                .iter()
                .position(|(_, t)| *t == state.theme)
                .unwrap_or(0);
            state.app_state = AppState::ThemeSelect;
        }
        "mute" => {
            state.mute_all = !state.mute_all;
            if let Some(ref mut audio) = ctx.audio_engine {
                audio.mute_track(AudioTrack::Metronome, state.mute_all);
                audio.mute_track(AudioTrack::Kit, state.mute_all);
                audio.mute_track(AudioTrack::Backtrack, state.mute_all);
            }
        }
        "mute-metronome" => {
            state.mute_metronome = !state.mute_metronome;
            if let Some(ref mut audio) = ctx.audio_engine {
                audio.mute_track(AudioTrack::Metronome, state.mute_metronome);
            }
        }
        "mute-backtrack" => {
            state.mute_backtrack = !state.mute_backtrack;
            if let Some(ref mut audio) = ctx.audio_engine {
                audio.mute_track(AudioTrack::Backtrack, state.mute_backtrack);
            }
        }
        "mute-kit" => {
            state.mute_kit = !state.mute_kit;
            if let Some(ref mut audio) = ctx.audio_engine {
                audio.mute_track(AudioTrack::Kit, state.mute_kit);
            }
        }
        "scoreboard" => {
            // Save current score before showing scoreboard
            if let Some(ref scoring) = ctx.scoring {
                save_score(state, ctx, &scoring.score_full);
            }
            load_personal_best(state, ctx);
            state.app_state = AppState::Scoreboard;
        }
        "cassette" => {
            open_track_select(state, ctx);
        }
        "loop" => {
            if let Some(ref mut pb) = ctx.playback {
                let num_bars: u32 = parsed
                    .arg
                    .as_deref()
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(2);
                let start_bar = state.current_bar;
                pb.set_loop(start_bar, num_bars);
                state.loop_active = true;
                state.loop_start_bar = start_bar;
                state.loop_end_bar = start_bar + num_bars;
            }
        }
        "track" => {
            open_track_select(state, ctx);
        }
        "kit" => {
            ensure_kits_discovered(state, ctx);
            state.app_state = AppState::KitSelect;
        }
        "autoplay" => {
            state.autoplay = !state.autoplay;
            state.status_message = Some((
                if state.autoplay {
                    "Autoplay ON".into()
                } else {
                    "Autoplay OFF".into()
                },
                std::time::Instant::now(),
            ));
        }
        "practice" => {
            if state.practice_mode {
                // Disable practice — restore original BPM
                state.practice_mode = false;
                if let Some(ref mut pb) = ctx.playback {
                    pb.set_bpm(state.practice_target_bpm);
                    state.effective_bpm = state.practice_target_bpm;
                }
                state.status_message =
                    Some(("Practice mode OFF".into(), std::time::Instant::now()));
            } else {
                // Enable practice — auto-enable loop if not active
                if let Some(ref mut pb) = ctx.playback {
                    if !pb.loop_active {
                        let start_bar = state.current_bar;
                        pb.set_loop(start_bar, 4);
                        state.loop_active = true;
                        state.loop_start_bar = start_bar;
                        state.loop_end_bar = start_bar + 4;
                    }
                    state.practice_target_bpm = pb.original_bpm;
                    state.practice_bpm = pb.original_bpm * 0.5;
                    pb.set_bpm(state.practice_bpm);
                    state.effective_bpm = state.practice_bpm;
                }
                state.practice_mode = true;
                // Show info message on first use in this session
                if !state.practice_shown_info {
                    state.practice_shown_info = true;
                    state.status_message = Some((
                        "Practice: loop active, speed adjusts with accuracy (>=90% faster, <70% slower)".into(),
                        std::time::Instant::now(),
                    ));
                } else {
                    state.status_message = Some((
                        format!("Practice ON at {:.0}% speed", (state.practice_bpm / state.practice_target_bpm * 100.0)),
                        std::time::Instant::now(),
                    ));
                }
            }
        }
        "calibrate" => {
            // Pause playback if currently playing
            if state.app_state == AppState::Session(SessionState::Playing) {
                if let Some(ref mut pb) = ctx.playback {
                    pb.pause();
                }
            }
            ctx.pre_calibration_state = Some(state.app_state.clone());
            state.app_state = AppState::Calibrating;
            // Force normal input mode -- no console during calibration
            state.input_mode = InputMode::Normal;
            state.console_input.clear();
            state.console_cursor = 0;
            state.autocomplete_suggestions.clear();
            state.autocomplete_selected = None;
            // Reset calibration state
            state.calibration_taps.clear();
            state.calibration_beat = 0;
            state.calibration_total_beats = 16;
            state.calibration_result = None;
            state.calibration_phase = 0.0;
            let now = Instant::now();
            ctx.calibration_start = Some(now);
            ctx.calibration_last_beat_instant = Some(now);
            ctx.calibration_last_beat_idx = None;
            ctx.calibration_result_time = None;
        }
        "help" => {
            state.help_visible = true;
        }
        "reset" => {
            // Wipe profile, scores, and preferences — go back to Welcome screen
            if let Err(e) = ctx.db.reset_all() {
                state.status_message =
                    Some((format!("Reset error: {}", e), std::time::Instant::now()));
            } else {
                ctx.profile_id = None;
                ctx.playback = None;
                ctx.scoring = None;
                state.track_name = None;
                state.score_full = ScoreAccumulator::default();
                state.score_8bar = ScoreAccumulator::default();
                state.score_16bar = ScoreAccumulator::default();
                state.score_32bar = ScoreAccumulator::default();
                state.personal_best = None;
                state.recent_results.clear();
                state.app_state = AppState::Welcome;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Load a track into the session.
fn load_track_with_bpm(
    state: &mut GameState,
    ctx: &mut SessionContext,
    track: DrumTrack,
    override_bpm: Option<f64>,
) {
    let windows = state.timing_preset.windows();
    let note_count = track.notes.len();
    state.track_name = Some(track.name.clone());
    state.total_bars = track.total_bars;
    state.track_duration_ms = track.duration_ms;
    state.pieces_used = track.pieces_used.clone();
    // Reset to 4/4 default, then override from track if present
    state.time_sig = if let Some(ts) = track.time_signatures.first() {
        (ts.numerator, ts.denominator)
    } else {
        (4, 4)
    };
    let mut engine = PlaybackEngine::new(track);
    state.effective_bpm = engine.original_bpm;
    // Apply BPM override (from meta.toml or CLI)
    if let Some(bpm) = override_bpm {
        if bpm > 0.0 {
            engine.set_bpm(bpm);
            state.effective_bpm = bpm;
        }
    }
    ctx.playback = Some(engine);
    let mut scoring = ScoringEngine::new(windows, note_count);
    // Apply difficulty filter: pre-judge notes excluded by current difficulty
    if state.difficulty != Difficulty::Hard {
        if let Some(ref pb) = ctx.playback {
            let diff = state.difficulty;
            scoring.reset_for_difficulty(&pb.track.notes, |n| {
                diff.includes_note(n, n.beat < 1.0)
            });
            state.pieces_used = pb
                .track
                .notes
                .iter()
                .filter(|n| diff.includes_note(n, n.beat < 1.0))
                .map(|n| n.piece)
                .collect();
        }
    }
    ctx.scoring = Some(scoring);
    state.score_full = ScoreAccumulator::default();
    state.score_8bar = ScoreAccumulator::default();
    state.score_16bar = ScoreAccumulator::default();
    state.score_32bar = ScoreAccumulator::default();
    state.recent_results.clear();
    // Reset metronome state for new track
    if let Some(ref mut metro) = ctx.metronome {
        metro.reset();
        metro.set_bpm(state.effective_bpm);
        metro.set_time_signature(state.time_sig.0, state.time_sig.1);
    }
    state.score_milestone = 0;
    state.score_milestone_time = None;
    state.app_state = AppState::Session(SessionState::Ready);
    load_personal_best(state, ctx);
}

// ---------------------------------------------------------------------------
// Game state update
// ---------------------------------------------------------------------------

fn update_game_state(state: &mut GameState, ctx: &mut SessionContext) {
    // --- Calibration update ---
    if state.app_state == AppState::Calibrating {
        if let Some(start) = ctx.calibration_start {
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Compute calibration_phase: 0.0-1.0 within the current beat
            let beats_elapsed = elapsed_ms / CALIB_BEAT_MS;
            state.calibration_phase = beats_elapsed.fract();

            // Play metronome clicks at beat boundaries
            let beat_idx = beats_elapsed.floor() as u64;
            if ctx.calibration_last_beat_idx != Some(beat_idx) {
                ctx.calibration_last_beat_idx = Some(beat_idx);
                ctx.calibration_last_beat_instant = Some(Instant::now());

                // Trigger audio click if available
                if let Some(ref mut metro) = ctx.metronome {
                    if let Some(ref mut audio) = ctx.audio_engine {
                        if !state.mute_metronome && !state.mute_all {
                            let (mgr, track) = audio.manager_and_metronome_track();
                            let _ = metro.tick(beats_elapsed, mgr, track);
                        }
                    }
                }
            }
        }

        // Check if result has been shown for 2+ seconds -> return to previous state
        if let Some(result_time) = ctx.calibration_result_time {
            if result_time.elapsed().as_secs_f64() >= 2.0 {
                let prev = ctx
                    .pre_calibration_state
                    .take()
                    .unwrap_or(AppState::TrackSelect);
                state.app_state = prev;
                ctx.calibration_start = None;
                ctx.calibration_last_beat_instant = None;
                ctx.calibration_last_beat_idx = None;
                ctx.calibration_result_time = None;
            }
        }
        return;
    }

    if let AppState::Session(SessionState::Playing) = &state.app_state {
        // Tick the playback engine
        if let Some(ref mut pb) = ctx.playback {
            let prev_pos = state.position_ms;
            pb.tick();
            state.position_ms = pb.position_ms;
            state.loop_active = pb.loop_active;

            // Position jumped backward (loop wrap) — reset judged notes and stale results
            if state.position_ms < prev_pos {
                // Practice mode: adjust speed based on loop iteration score
                if state.practice_mode {
                    let pct = state.score_full.percentage();
                    let step = state.practice_target_bpm * 0.05; // 5% of target
                    if pct >= 90.0 {
                        state.practice_bpm = (state.practice_bpm + step)
                            .min(state.practice_target_bpm);
                    } else if pct < 70.0 {
                        state.practice_bpm = (state.practice_bpm - step)
                            .max(state.practice_target_bpm * 0.3);
                    }
                    pb.set_bpm(state.practice_bpm);
                    state.effective_bpm = state.practice_bpm;
                    state.score_full = ScoreAccumulator::default();
                }

                state.recent_results.clear();
                if let Some(ref mut scoring) = ctx.scoring {
                    let notes = &pb.track.notes;
                    let start_idx = notes.partition_point(|n| n.time_ms < pb.loop_start_ms);
                    let end_idx = notes.partition_point(|n| n.time_ms < pb.loop_end_ms);
                    let diff = state.difficulty;
                    scoring.reset_loop_window(notes, start_idx, end_idx, |n| {
                        diff.includes_note(n, n.beat < 1.0)
                    });
                }
            }

            // Compute current bar/beat from position.
            // position_ms is in the original track timeline, so use original_bpm
            // for beat calculation. effective_bpm is only for wall-clock (preroll).
            // original_bpm is always in quarter-note BPM (MIDI spec).
            // The actual beat duration depends on the time sig denominator:
            //   /4 → beat = quarter note, /8 → beat = eighth note, etc.
            let original_bpm = pb.original_bpm;
            let (ts_num, ts_denom) = state.time_sig;
            if original_bpm > 0.0 {
                let quarter_ms = 60_000.0 / original_bpm;
                let beat_ms = quarter_ms * 4.0 / ts_denom as f64;

                if state.preroll_active {
                    // Preroll: same formula but from effective_bpm
                    let eff_quarter_ms = if state.effective_bpm > 0.0 {
                        60_000.0 / state.effective_bpm
                    } else {
                        quarter_ms
                    };
                    let eff_beat_ms = eff_quarter_ms * 4.0 / ts_denom as f64;
                    let preroll_elapsed = state.preroll_start
                        .map(|s| s.elapsed().as_secs_f64() * 1000.0)
                        .unwrap_or(0.0);
                    let beats = preroll_elapsed / eff_beat_ms;
                    state.preroll_beats_elapsed = beats;
                    state.current_beat = beats % ts_num as f64;
                    state.metronome_phase = beats.fract();
                    state.current_bar = 0;

                    // End preroll when position crosses 0
                    if state.position_ms >= 0.0 {
                        state.preroll_active = false;
                        state.preroll_start = None;
                    }
                } else {
                    let beats_elapsed = state.position_ms / beat_ms;
                    state.current_beat = beats_elapsed % ts_num as f64;
                    state.current_bar =
                        (beats_elapsed / ts_num as f64) as u32;
                    state.metronome_phase = beats_elapsed.fract();
                }

                // Tick metronome audio
                if let Some(ref mut metro) = ctx.metronome {
                    if let Some(ref mut audio) = ctx.audio_engine {
                        if !state.mute_metronome && !state.mute_all {
                            let (mgr, track) =
                                audio.manager_and_metronome_track();
                            let beats = if state.preroll_active {
                                state.preroll_beats_elapsed
                            } else {
                                state.position_ms / beat_ms
                            };
                            let _ = metro.tick(beats, mgr, track);
                        }
                    }
                }
            }

            // Update visible notes, filtered by difficulty
            let look_ahead = 3000.0; // 3 second look-ahead
            state.visible_notes = pb
                .visible_notes(look_ahead)
                .iter()
                .filter(|n| state.difficulty.includes_note(n, n.beat < 1.0))
                .cloned()
                .collect();

            // Autoplay: auto-hit notes at the exact right time
            if state.autoplay && !state.preroll_active {
                if let Some(ref mut scoring) = ctx.scoring {
                    let (hittable, offset) =
                        scoring.hittable_notes_from(pb.position_ms, &pb.track.notes);
                    // Hit each unhit note that has reached position
                    for (local_idx, note) in hittable.iter().enumerate() {
                        let abs_idx = offset + local_idx;
                        if scoring.is_judged(abs_idx) {
                            continue;
                        }
                        let diff = state.difficulty;
                        if !diff.includes_note(note, note.beat < 1.0) {
                            continue;
                        }
                        // Only auto-hit notes that are at or past position
                        if note.time_ms <= pb.position_ms {
                            let result = scoring.process_hit(
                                note.piece,
                                note.time_ms,
                                hittable,
                                offset,
                            );
                            state.recent_results.push_back((pb.position_ms, result));
                            // Trigger audio
                            if let Some(ref mut audio) = ctx.audio_engine {
                                if state.audio_mode == AudioMode::VisualAudio
                                    && !state.mute_kit
                                    && !state.mute_all
                                {
                                    let _ = audio.trigger_hit(note.piece, note.velocity);
                                }
                            }
                        }
                    }
                    state.score_full = scoring.score_full.clone();
                }
            }

            // Check for misses (only for notes visible at current difficulty)
            if let Some(ref mut scoring) = ctx.scoring {
                let diff = state.difficulty;
                let misses =
                    scoring.check_misses(pb.position_ms, &pb.track.notes, |n| {
                        diff.includes_note(n, n.beat < 1.0)
                    });
                for miss in misses {
                    state
                        .recent_results
                        .push_back((pb.position_ms, miss));
                }

                // Prune rolling windows
                scoring.prune_rolling(state.current_bar);

                // Update score snapshots
                state.score_full = scoring.score_full.clone();
                state.score_8bar = scoring.rolling_8bar.summarize();
                state.score_16bar = scoring.rolling_16bar.summarize();
                state.score_32bar = scoring.rolling_32bar.summarize();
            }

            // Check score milestones (every 20%)
            if state.score_full.total_notes > 0 {
                let pct = state.score_full.percentage();
                let new_milestone = ((pct / 20.0).floor() as u8) * 20;
                if new_milestone > state.score_milestone {
                    state.score_milestone = new_milestone;
                    state.score_milestone_time = Some(std::time::Instant::now());
                }
            }

            // Prune old recent results (keep last 2 seconds)
            let cutoff = pb.position_ms - 2000.0;
            while let Some(&(t, _)) = state.recent_results.front() {
                if t < cutoff {
                    state.recent_results.pop_front();
                } else {
                    break;
                }
            }

            // Check if track ended
            if !pb.playing && pb.position_ms >= pb.track.duration_ms {
                // Save score to database
                if let Some(ref scoring) = ctx.scoring {
                    save_score(state, ctx, &scoring.score_full);
                }
                load_personal_best(state, ctx);
                state.app_state = AppState::Scoreboard;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn save_score(
    state: &mut GameState,
    ctx: &SessionContext,
    score: &ScoreAccumulator,
) {
    let log_path = AppConfig::data_dir().join("debug.log");

    let Some(profile_id) = ctx.profile_id else {
        debug_log(&log_path, "save_score: no profile_id, skipping");
        return;
    };
    let Some(ref track_name) = state.track_name else {
        debug_log(&log_path, "save_score: no track_name, skipping");
        return;
    };
    let difficulty = match state.difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    };
    let timing = match state.timing_preset {
        TimingPreset::Relaxed => "relaxed",
        TimingPreset::Standard => "standard",
        TimingPreset::Strict => "strict",
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let record = crate::data::db::ScoreRecord {
        id: 0,
        track_name: track_name.clone(),
        difficulty: difficulty.to_string(),
        timing_preset: timing.to_string(),
        bpm: state.effective_bpm,
        scope: "full".to_string(),
        score_pct: score.percentage(),
        perfect: score.perfect_count,
        great: score.great_count,
        good: score.good_count,
        ok: score.ok_count,
        miss: score.miss_count,
        wrong_piece: score.wrong_piece_count,
        max_combo: score.max_combo,
        played_at: now,
    };

    debug_log(&log_path, &format!(
        "save_score: profile={}, track={}, diff={}, timing={}, bpm={:.0}, pct={:.1}%, notes={}",
        profile_id, track_name, difficulty, timing, state.effective_bpm,
        score.percentage(), score.total_notes
    ));

    match ctx.db.save_score(profile_id, &record, "") {
        Ok(()) => debug_log(&log_path, "save_score: OK"),
        Err(e) => {
            debug_log(&log_path, &format!("save_score: ERROR {}", e));
            state.status_message =
                Some((format!("Score save error: {}", e), std::time::Instant::now()));
        }
    }
}

fn load_personal_best(state: &mut GameState, ctx: &SessionContext) {
    let log_path = AppConfig::data_dir().join("debug.log");

    let Some(ref track_name) = state.track_name else {
        state.personal_best = None;
        return;
    };
    let difficulty = match state.difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    };
    let timing = match state.timing_preset {
        TimingPreset::Relaxed => "relaxed",
        TimingPreset::Standard => "standard",
        TimingPreset::Strict => "strict",
    };

    match ctx.db.top_scores(track_name, "full", difficulty, timing, 1) {
        Ok(scores) => {
            state.personal_best = scores.first().map(|s| s.score_pct);
            debug_log(&log_path, &format!(
                "load_best: track={}, diff={}, timing={}, result={:?}",
                track_name, difficulty, timing, state.personal_best
            ));
        }
        Err(e) => {
            debug_log(&log_path, &format!("load_best: ERROR {}", e));
            state.personal_best = None;
        }
    }
}

fn debug_log(path: &std::path::Path, msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "[{}] {}", chrono_timestamp(), msg);
    }
}

fn chrono_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}


/// Build key hint labels from a KeyMap for display in lane headers.
fn build_key_hints(key_map: &KeyMap) -> Vec<(DrumPiece, String)> {
    let mut hints = Vec::new();
    for (key_code, piece) in &key_map.map {
        let label = match key_code {
            KeyCode::Char(' ') => "\u{2423}".to_string(), // open box (space)
            KeyCode::Char(c) => c.to_string(),
            _ => continue,
        };
        hints.push((*piece, label));
    }
    hints
}

fn resolve_theme(cli_theme: Option<&str>, config: &AppConfig) -> ThemeName {
    let name = cli_theme.unwrap_or(&config.display.theme);
    match name {
        "desert" => ThemeName::Desert,
        "evening" => ThemeName::Evening,
        "slate" => ThemeName::Slate,
        "blue" => ThemeName::Blue,
        "pablo" => ThemeName::Pablo,
        "quiet" => ThemeName::Quiet,
        "shine" => ThemeName::Shine,
        "run" => ThemeName::Run,
        _ => ThemeName::Gruvbox,
    }
}
