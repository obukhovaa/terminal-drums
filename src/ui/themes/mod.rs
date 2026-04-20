use ratatui::style::Color;

/// A complete color theme for the application.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,

    // Base colors
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub accent: Color,

    // UI chrome
    pub border: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub status_bg: Color,
    pub status_fg: Color,

    // Drum piece colors (used in highway lanes)
    pub kick: Color,
    pub snare: Color,
    pub cross_stick: Color,
    pub hihat: Color,
    pub crash: Color,
    pub ride: Color,
    pub tom_high: Color,
    pub tom_mid: Color,
    pub tom_low: Color,
    pub splash: Color,
    pub china: Color,

    // Hit accuracy feedback
    pub perfect: Color,
    pub great: Color,
    pub good: Color,
    pub ok: Color,
    pub miss: Color,
    pub wrong_piece: Color,
    pub extra: Color,

    // Metronome
    pub metronome_fg: Color,
    pub metronome_accent: Color,

    // Console
    pub console_bg: Color,
    pub console_fg: Color,
    pub console_prompt: Color,
    pub autocomplete_bg: Color,
    pub autocomplete_fg: Color,
    pub autocomplete_selected_bg: Color,

    // Score
    pub score_fg: Color,
    pub combo_fg: Color,
    pub progress_bar_fg: Color,
    pub progress_bar_bg: Color,
}

/// Names for the 9 bundled themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeName {
    Gruvbox,
    Desert,
    Evening,
    Slate,
    Blue,
    Pablo,
    Quiet,
    Shine,
    Run,
}

impl Default for ThemeName {
    fn default() -> Self {
        ThemeName::Gruvbox
    }
}

/// Get a theme by name.
pub fn get_theme(name: ThemeName) -> Theme {
    match name {
        ThemeName::Gruvbox => gruvbox(),
        ThemeName::Desert => desert(),
        ThemeName::Evening => evening(),
        ThemeName::Slate => slate(),
        ThemeName::Blue => blue(),
        ThemeName::Pablo => pablo(),
        ThemeName::Quiet => quiet(),
        ThemeName::Shine => shine(),
        ThemeName::Run => run(),
    }
}

fn gruvbox() -> Theme {
    Theme {
        name: "gruvbox",

        // Base colors
        bg: Color::Rgb(40, 40, 40),         // #282828
        fg: Color::Rgb(235, 219, 178),      // #ebdbb2
        fg_dim: Color::Rgb(146, 131, 116),  // #928374
        accent: Color::Rgb(254, 128, 25),   // #fe8019

        // UI chrome
        border: Color::Rgb(80, 73, 69),     // #504945
        header_bg: Color::Rgb(50, 48, 47),  // #32302f
        header_fg: Color::Rgb(235, 219, 178),
        status_bg: Color::Rgb(50, 48, 47),
        status_fg: Color::Rgb(168, 153, 132),

        // Drum piece colors
        kick: Color::Rgb(204, 36, 29),        // #cc241d
        snare: Color::Rgb(215, 153, 33),      // #d79921
        cross_stick: Color::Rgb(177, 98, 134),// #b16286
        hihat: Color::Rgb(104, 157, 106),     // #689d6a
        crash: Color::Rgb(250, 189, 47),      // #fabd2f
        ride: Color::Rgb(69, 133, 136),       // #458588
        tom_high: Color::Rgb(177, 98, 134),   // #b16286
        tom_mid: Color::Rgb(177, 98, 134),    // #b16286
        tom_low: Color::Rgb(177, 98, 134),    // #b16286
        splash: Color::Rgb(250, 189, 47),     // #fabd2f
        china: Color::Rgb(250, 189, 47),      // #fabd2f

        // Hit accuracy feedback
        perfect: Color::Rgb(184, 187, 38),    // #b8bb26
        great: Color::Rgb(131, 165, 152),     // #83a598
        good: Color::Rgb(215, 153, 33),       // #d79921
        ok: Color::Rgb(254, 128, 25),         // #fe8019
        miss: Color::Rgb(204, 36, 29),        // #cc241d
        wrong_piece: Color::Rgb(254, 128, 25),// #fe8019
        extra: Color::Rgb(146, 131, 116),     // #928374

        // Metronome
        metronome_fg: Color::Rgb(235, 219, 178),
        metronome_accent: Color::Rgb(254, 128, 25),

        // Console
        console_bg: Color::Rgb(29, 32, 33),   // #1d2021
        console_fg: Color::Rgb(235, 219, 178),
        console_prompt: Color::Rgb(184, 187, 38),
        autocomplete_bg: Color::Rgb(60, 56, 54),
        autocomplete_fg: Color::Rgb(235, 219, 178),
        autocomplete_selected_bg: Color::Rgb(80, 73, 69),

        // Score
        score_fg: Color::Rgb(235, 219, 178),
        combo_fg: Color::Rgb(250, 189, 47),
        progress_bar_fg: Color::Rgb(184, 187, 38),
        progress_bar_bg: Color::Rgb(60, 56, 54),
    }
}

fn desert() -> Theme {
    Theme {
        name: "desert",

        bg: Color::Rgb(51, 51, 51),         // #333333
        fg: Color::Rgb(255, 160, 160),      // #ffa0a0
        fg_dim: Color::Rgb(160, 100, 100),
        accent: Color::Rgb(240, 230, 140),  // #f0e68c

        border: Color::Rgb(100, 80, 60),
        header_bg: Color::Rgb(60, 55, 45),
        header_fg: Color::Rgb(255, 160, 160),
        status_bg: Color::Rgb(60, 55, 45),
        status_fg: Color::Rgb(180, 130, 100),

        kick: Color::Rgb(220, 80, 80),
        snare: Color::Rgb(240, 180, 80),
        cross_stick: Color::Rgb(200, 140, 100),
        hihat: Color::Rgb(160, 200, 120),
        crash: Color::Rgb(240, 230, 140),
        ride: Color::Rgb(120, 180, 180),
        tom_high: Color::Rgb(200, 120, 160),
        tom_mid: Color::Rgb(180, 100, 140),
        tom_low: Color::Rgb(160, 80, 120),
        splash: Color::Rgb(230, 210, 100),
        china: Color::Rgb(210, 190, 80),

        perfect: Color::Rgb(180, 220, 100),
        great: Color::Rgb(120, 180, 180),
        good: Color::Rgb(240, 180, 80),
        ok: Color::Rgb(240, 230, 140),
        miss: Color::Rgb(220, 80, 80),
        wrong_piece: Color::Rgb(200, 140, 80),
        extra: Color::Rgb(140, 120, 100),

        metronome_fg: Color::Rgb(255, 160, 160),
        metronome_accent: Color::Rgb(240, 230, 140),

        console_bg: Color::Rgb(40, 38, 30),
        console_fg: Color::Rgb(255, 160, 160),
        console_prompt: Color::Rgb(240, 230, 140),
        autocomplete_bg: Color::Rgb(70, 65, 50),
        autocomplete_fg: Color::Rgb(255, 160, 160),
        autocomplete_selected_bg: Color::Rgb(100, 90, 60),

        score_fg: Color::Rgb(255, 160, 160),
        combo_fg: Color::Rgb(240, 230, 140),
        progress_bar_fg: Color::Rgb(180, 220, 100),
        progress_bar_bg: Color::Rgb(70, 65, 50),
    }
}

fn evening() -> Theme {
    Theme {
        name: "evening",

        bg: Color::Rgb(0, 0, 42),           // #00002a
        fg: Color::Rgb(192, 192, 192),      // #c0c0c0
        fg_dim: Color::Rgb(100, 100, 140),
        accent: Color::Rgb(112, 112, 255),  // #7070ff

        border: Color::Rgb(50, 50, 100),
        header_bg: Color::Rgb(10, 10, 55),
        header_fg: Color::Rgb(192, 192, 192),
        status_bg: Color::Rgb(10, 10, 55),
        status_fg: Color::Rgb(130, 130, 180),

        kick: Color::Rgb(180, 80, 180),
        snare: Color::Rgb(100, 180, 240),
        cross_stick: Color::Rgb(150, 100, 200),
        hihat: Color::Rgb(100, 200, 180),
        crash: Color::Rgb(200, 200, 100),
        ride: Color::Rgb(80, 160, 220),
        tom_high: Color::Rgb(160, 100, 220),
        tom_mid: Color::Rgb(140, 80, 200),
        tom_low: Color::Rgb(120, 60, 180),
        splash: Color::Rgb(180, 180, 80),
        china: Color::Rgb(200, 180, 60),

        perfect: Color::Rgb(100, 220, 100),
        great: Color::Rgb(80, 160, 220),
        good: Color::Rgb(200, 200, 80),
        ok: Color::Rgb(220, 160, 60),
        miss: Color::Rgb(220, 60, 60),
        wrong_piece: Color::Rgb(200, 140, 60),
        extra: Color::Rgb(100, 100, 140),

        metronome_fg: Color::Rgb(192, 192, 192),
        metronome_accent: Color::Rgb(112, 112, 255),

        console_bg: Color::Rgb(5, 5, 30),
        console_fg: Color::Rgb(192, 192, 192),
        console_prompt: Color::Rgb(112, 112, 255),
        autocomplete_bg: Color::Rgb(20, 20, 70),
        autocomplete_fg: Color::Rgb(192, 192, 192),
        autocomplete_selected_bg: Color::Rgb(50, 50, 120),

        score_fg: Color::Rgb(192, 192, 192),
        combo_fg: Color::Rgb(112, 112, 255),
        progress_bar_fg: Color::Rgb(80, 160, 220),
        progress_bar_bg: Color::Rgb(20, 20, 70),
    }
}

fn slate() -> Theme {
    Theme {
        name: "slate",

        bg: Color::Rgb(38, 38, 38),         // #262626
        fg: Color::Rgb(198, 200, 209),      // #c6c8d1
        fg_dim: Color::Rgb(120, 122, 130),
        accent: Color::Rgb(107, 112, 137),  // #6b7089

        border: Color::Rgb(80, 85, 100),
        header_bg: Color::Rgb(48, 50, 60),
        header_fg: Color::Rgb(198, 200, 209),
        status_bg: Color::Rgb(48, 50, 60),
        status_fg: Color::Rgb(150, 155, 170),

        kick: Color::Rgb(180, 100, 120),
        snare: Color::Rgb(160, 180, 120),
        cross_stick: Color::Rgb(140, 130, 170),
        hihat: Color::Rgb(100, 170, 180),
        crash: Color::Rgb(190, 190, 120),
        ride: Color::Rgb(100, 150, 180),
        tom_high: Color::Rgb(150, 120, 180),
        tom_mid: Color::Rgb(135, 105, 165),
        tom_low: Color::Rgb(120, 90, 150),
        splash: Color::Rgb(180, 180, 110),
        china: Color::Rgb(170, 170, 100),

        perfect: Color::Rgb(150, 200, 150),
        great: Color::Rgb(100, 160, 200),
        good: Color::Rgb(190, 190, 120),
        ok: Color::Rgb(200, 170, 100),
        miss: Color::Rgb(200, 100, 100),
        wrong_piece: Color::Rgb(190, 160, 90),
        extra: Color::Rgb(120, 122, 130),

        metronome_fg: Color::Rgb(198, 200, 209),
        metronome_accent: Color::Rgb(107, 112, 137),

        console_bg: Color::Rgb(30, 32, 38),
        console_fg: Color::Rgb(198, 200, 209),
        console_prompt: Color::Rgb(150, 160, 200),
        autocomplete_bg: Color::Rgb(55, 58, 70),
        autocomplete_fg: Color::Rgb(198, 200, 209),
        autocomplete_selected_bg: Color::Rgb(80, 85, 105),

        score_fg: Color::Rgb(198, 200, 209),
        combo_fg: Color::Rgb(150, 160, 200),
        progress_bar_fg: Color::Rgb(107, 112, 137),
        progress_bar_bg: Color::Rgb(55, 58, 70),
    }
}

fn blue() -> Theme {
    Theme {
        name: "blue",

        bg: Color::Rgb(26, 26, 46),         // #1a1a2e
        fg: Color::Rgb(224, 224, 255),      // #e0e0ff
        fg_dim: Color::Rgb(130, 140, 180),
        accent: Color::Rgb(79, 195, 247),   // #4fc3f7

        border: Color::Rgb(60, 80, 120),
        header_bg: Color::Rgb(36, 36, 66),
        header_fg: Color::Rgb(224, 224, 255),
        status_bg: Color::Rgb(36, 36, 66),
        status_fg: Color::Rgb(160, 170, 210),

        kick: Color::Rgb(200, 80, 160),
        snare: Color::Rgb(80, 200, 240),
        cross_stick: Color::Rgb(160, 120, 220),
        hihat: Color::Rgb(80, 220, 200),
        crash: Color::Rgb(220, 220, 100),
        ride: Color::Rgb(79, 195, 247),
        tom_high: Color::Rgb(180, 120, 240),
        tom_mid: Color::Rgb(160, 100, 220),
        tom_low: Color::Rgb(140, 80, 200),
        splash: Color::Rgb(200, 200, 80),
        china: Color::Rgb(180, 180, 60),

        perfect: Color::Rgb(100, 240, 140),
        great: Color::Rgb(79, 195, 247),
        good: Color::Rgb(220, 220, 100),
        ok: Color::Rgb(240, 180, 80),
        miss: Color::Rgb(240, 80, 100),
        wrong_piece: Color::Rgb(220, 160, 80),
        extra: Color::Rgb(130, 140, 180),

        metronome_fg: Color::Rgb(224, 224, 255),
        metronome_accent: Color::Rgb(79, 195, 247),

        console_bg: Color::Rgb(20, 20, 38),
        console_fg: Color::Rgb(224, 224, 255),
        console_prompt: Color::Rgb(79, 195, 247),
        autocomplete_bg: Color::Rgb(45, 50, 85),
        autocomplete_fg: Color::Rgb(224, 224, 255),
        autocomplete_selected_bg: Color::Rgb(70, 80, 130),

        score_fg: Color::Rgb(224, 224, 255),
        combo_fg: Color::Rgb(79, 195, 247),
        progress_bar_fg: Color::Rgb(79, 195, 247),
        progress_bar_bg: Color::Rgb(45, 50, 85),
    }
}

fn pablo() -> Theme {
    Theme {
        name: "pablo",

        bg: Color::Rgb(0, 0, 0),            // #000000
        fg: Color::Rgb(255, 255, 255),      // #ffffff
        fg_dim: Color::Rgb(160, 160, 160),
        accent: Color::Rgb(255, 102, 0),    // #ff6600

        border: Color::Rgb(100, 100, 100),
        header_bg: Color::Rgb(20, 20, 20),
        header_fg: Color::Rgb(255, 255, 255),
        status_bg: Color::Rgb(20, 20, 20),
        status_fg: Color::Rgb(200, 200, 200),

        kick: Color::Rgb(255, 60, 60),
        snare: Color::Rgb(255, 200, 0),
        cross_stick: Color::Rgb(220, 100, 180),
        hihat: Color::Rgb(0, 220, 180),
        crash: Color::Rgb(255, 220, 0),
        ride: Color::Rgb(0, 180, 255),
        tom_high: Color::Rgb(220, 80, 220),
        tom_mid: Color::Rgb(200, 60, 200),
        tom_low: Color::Rgb(180, 40, 180),
        splash: Color::Rgb(240, 200, 0),
        china: Color::Rgb(255, 220, 0),

        perfect: Color::Rgb(0, 255, 100),
        great: Color::Rgb(0, 180, 255),
        good: Color::Rgb(255, 200, 0),
        ok: Color::Rgb(255, 102, 0),
        miss: Color::Rgb(255, 0, 0),
        wrong_piece: Color::Rgb(255, 140, 0),
        extra: Color::Rgb(160, 160, 160),

        metronome_fg: Color::Rgb(255, 255, 255),
        metronome_accent: Color::Rgb(255, 102, 0),

        console_bg: Color::Rgb(10, 10, 10),
        console_fg: Color::Rgb(255, 255, 255),
        console_prompt: Color::Rgb(255, 102, 0),
        autocomplete_bg: Color::Rgb(30, 30, 30),
        autocomplete_fg: Color::Rgb(255, 255, 255),
        autocomplete_selected_bg: Color::Rgb(80, 60, 0),

        score_fg: Color::Rgb(255, 255, 255),
        combo_fg: Color::Rgb(255, 102, 0),
        progress_bar_fg: Color::Rgb(255, 102, 0),
        progress_bar_bg: Color::Rgb(40, 40, 40),
    }
}

fn quiet() -> Theme {
    Theme {
        name: "quiet",

        bg: Color::Rgb(28, 28, 28),         // #1c1c1c
        fg: Color::Rgb(128, 128, 128),      // #808080
        fg_dim: Color::Rgb(80, 80, 80),
        accent: Color::Rgb(80, 80, 80),     // #505050

        border: Color::Rgb(60, 60, 60),
        header_bg: Color::Rgb(35, 35, 35),
        header_fg: Color::Rgb(128, 128, 128),
        status_bg: Color::Rgb(35, 35, 35),
        status_fg: Color::Rgb(100, 100, 100),

        kick: Color::Rgb(150, 80, 80),
        snare: Color::Rgb(140, 130, 80),
        cross_stick: Color::Rgb(120, 100, 120),
        hihat: Color::Rgb(80, 130, 110),
        crash: Color::Rgb(140, 130, 80),
        ride: Color::Rgb(80, 110, 130),
        tom_high: Color::Rgb(120, 90, 130),
        tom_mid: Color::Rgb(110, 80, 120),
        tom_low: Color::Rgb(100, 70, 110),
        splash: Color::Rgb(130, 120, 70),
        china: Color::Rgb(120, 110, 60),

        perfect: Color::Rgb(100, 140, 100),
        great: Color::Rgb(80, 120, 140),
        good: Color::Rgb(140, 130, 80),
        ok: Color::Rgb(140, 110, 70),
        miss: Color::Rgb(150, 80, 80),
        wrong_piece: Color::Rgb(140, 110, 70),
        extra: Color::Rgb(80, 80, 80),

        metronome_fg: Color::Rgb(100, 100, 100),
        metronome_accent: Color::Rgb(80, 80, 80),

        console_bg: Color::Rgb(22, 22, 22),
        console_fg: Color::Rgb(110, 110, 110),
        console_prompt: Color::Rgb(90, 90, 90),
        autocomplete_bg: Color::Rgb(38, 38, 38),
        autocomplete_fg: Color::Rgb(110, 110, 110),
        autocomplete_selected_bg: Color::Rgb(55, 55, 55),

        score_fg: Color::Rgb(128, 128, 128),
        combo_fg: Color::Rgb(100, 100, 100),
        progress_bar_fg: Color::Rgb(80, 80, 80),
        progress_bar_bg: Color::Rgb(40, 40, 40),
    }
}

fn shine() -> Theme {
    Theme {
        name: "shine",

        bg: Color::Rgb(10, 10, 10),         // #0a0a0a
        fg: Color::Rgb(255, 255, 255),      // #ffffff
        fg_dim: Color::Rgb(160, 160, 160),
        accent: Color::Rgb(0, 255, 136),    // #00ff88

        border: Color::Rgb(0, 180, 100),
        header_bg: Color::Rgb(18, 18, 18),
        header_fg: Color::Rgb(255, 255, 255),
        status_bg: Color::Rgb(18, 18, 18),
        status_fg: Color::Rgb(180, 255, 200),

        kick: Color::Rgb(255, 80, 80),
        snare: Color::Rgb(255, 220, 0),
        cross_stick: Color::Rgb(200, 100, 255),
        hihat: Color::Rgb(0, 255, 180),
        crash: Color::Rgb(255, 255, 0),
        ride: Color::Rgb(0, 220, 255),
        tom_high: Color::Rgb(180, 80, 255),
        tom_mid: Color::Rgb(160, 60, 235),
        tom_low: Color::Rgb(140, 40, 215),
        splash: Color::Rgb(255, 240, 0),
        china: Color::Rgb(255, 220, 0),

        perfect: Color::Rgb(0, 255, 136),
        great: Color::Rgb(0, 220, 255),
        good: Color::Rgb(255, 220, 0),
        ok: Color::Rgb(255, 160, 0),
        miss: Color::Rgb(255, 60, 60),
        wrong_piece: Color::Rgb(255, 120, 0),
        extra: Color::Rgb(160, 160, 160),

        metronome_fg: Color::Rgb(255, 255, 255),
        metronome_accent: Color::Rgb(0, 255, 136),

        console_bg: Color::Rgb(8, 8, 8),
        console_fg: Color::Rgb(255, 255, 255),
        console_prompt: Color::Rgb(0, 255, 136),
        autocomplete_bg: Color::Rgb(25, 25, 25),
        autocomplete_fg: Color::Rgb(255, 255, 255),
        autocomplete_selected_bg: Color::Rgb(0, 60, 35),

        score_fg: Color::Rgb(255, 255, 255),
        combo_fg: Color::Rgb(0, 255, 136),
        progress_bar_fg: Color::Rgb(0, 255, 136),
        progress_bar_bg: Color::Rgb(30, 30, 30),
    }
}

fn run() -> Theme {
    Theme {
        name: "run",

        bg: Color::Rgb(13, 2, 33),          // #0d0221
        fg: Color::Rgb(255, 0, 255),        // #ff00ff
        fg_dim: Color::Rgb(160, 0, 160),
        accent: Color::Rgb(0, 255, 255),    // #00ffff

        border: Color::Rgb(100, 0, 140),
        header_bg: Color::Rgb(22, 5, 50),
        header_fg: Color::Rgb(255, 0, 255),
        status_bg: Color::Rgb(22, 5, 50),
        status_fg: Color::Rgb(200, 100, 255),

        kick: Color::Rgb(255, 0, 100),
        snare: Color::Rgb(0, 255, 255),
        cross_stick: Color::Rgb(200, 0, 255),
        hihat: Color::Rgb(0, 255, 180),
        crash: Color::Rgb(255, 255, 0),
        ride: Color::Rgb(0, 200, 255),
        tom_high: Color::Rgb(255, 0, 200),
        tom_mid: Color::Rgb(220, 0, 180),
        tom_low: Color::Rgb(190, 0, 160),
        splash: Color::Rgb(255, 220, 0),
        china: Color::Rgb(240, 200, 0),

        perfect: Color::Rgb(0, 255, 180),
        great: Color::Rgb(0, 255, 255),
        good: Color::Rgb(255, 255, 0),
        ok: Color::Rgb(255, 160, 0),
        miss: Color::Rgb(255, 0, 60),
        wrong_piece: Color::Rgb(255, 100, 0),
        extra: Color::Rgb(160, 0, 160),

        metronome_fg: Color::Rgb(255, 0, 255),
        metronome_accent: Color::Rgb(0, 255, 255),

        console_bg: Color::Rgb(8, 2, 20),
        console_fg: Color::Rgb(255, 0, 255),
        console_prompt: Color::Rgb(0, 255, 255),
        autocomplete_bg: Color::Rgb(30, 10, 55),
        autocomplete_fg: Color::Rgb(255, 0, 255),
        autocomplete_selected_bg: Color::Rgb(60, 0, 100),

        score_fg: Color::Rgb(255, 0, 255),
        combo_fg: Color::Rgb(0, 255, 255),
        progress_bar_fg: Color::Rgb(0, 255, 255),
        progress_bar_bg: Color::Rgb(35, 10, 60),
    }
}
