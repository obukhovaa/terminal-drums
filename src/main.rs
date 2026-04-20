#[allow(dead_code)]
mod app;
#[allow(dead_code)]
mod audio;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod data;
#[allow(dead_code)]
mod engine;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod input;
#[allow(dead_code)]
mod midi;
#[allow(dead_code)]
mod ui;

use clap::Parser;

#[derive(Parser)]
#[command(name = "tdrums", version, about = "Terminal-based drum training app")]
struct Cli {
    /// Path to a MIDI file to load
    path: Option<String>,

    /// Override BPM
    #[arg(long)]
    bpm: Option<f64>,

    /// Select drum kit
    #[arg(long)]
    kit: Option<String>,

    /// Select color theme
    #[arg(long)]
    theme: Option<String>,

    /// Visual-only mode (no audio)
    #[arg(long)]
    visual_only: bool,

    /// Custom config file path
    #[arg(long)]
    config: Option<String>,
}

fn main() {
    // Custom panic handler to restore terminal
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        default_panic(info);
    }));

    let cli = Cli::parse();

    let args = app::CliArgs {
        path: cli.path,
        bpm: cli.bpm,
        kit: cli.kit,
        theme: cli.theme,
        visual_only: cli.visual_only,
        config: cli.config,
    };

    if let Err(e) = app::run_with_args(args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
