//! Theme and styling constants for the TUI.

use ratatui::style::{Color, Modifier, Style};

/// Application color palette.
pub struct Theme;

impl Theme {
    // Base colors
    pub const BG: Color = Color::Reset;
    pub const FG: Color = Color::White;
    pub const DIM: Color = Color::DarkGray;

    // Accent colors
    pub const PRIMARY: Color = Color::Cyan;
    pub const SECONDARY: Color = Color::Blue;
    pub const SUCCESS: Color = Color::Green;
    pub const WARNING: Color = Color::Yellow;
    pub const ERROR: Color = Color::Red;

    // Risk level colors
    pub const RISK_LOW: Color = Color::Green;
    pub const RISK_MEDIUM: Color = Color::Yellow;
    pub const RISK_HIGH: Color = Color::Red;

    // Tab colors
    pub const TAB_ACTIVE: Color = Color::Cyan;
    pub const TAB_INACTIVE: Color = Color::DarkGray;

    // Category icons
    pub const ICON_SYSTEM: &'static str = "🖥️";
    pub const ICON_CACHE: &'static str = "📦";
    pub const ICON_LOGS: &'static str = "📋";
    pub const ICON_TRASH: &'static str = "🗑️";
    pub const ICON_XCODE: &'static str = "🔨";
    pub const ICON_HOMEBREW: &'static str = "🍺";
    pub const ICON_NPM: &'static str = "📦";
    pub const ICON_YARN: &'static str = "🧶";
    pub const ICON_CARGO: &'static str = "🦀";
    pub const ICON_PIP: &'static str = "🐍";
    pub const ICON_DOCKER: &'static str = "🐳";
    pub const ICON_APPS: &'static str = "📱";

    // Disk health indicators
    pub const HEALTH_GOOD: &'static str = "●";
    pub const HEALTH_WARNING: &'static str = "◐";
    pub const HEALTH_CRITICAL: &'static str = "○";
}

/// Common styles.
pub struct Styles;

impl Styles {
    pub fn title() -> Style {
        Style::default()
            .fg(Theme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlight() -> Style {
        Style::default()
            .fg(Theme::BG)
            .bg(Theme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected() -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }

    pub fn dim() -> Style {
        Style::default().fg(Theme::DIM)
    }

    pub fn success() -> Style {
        Style::default().fg(Theme::SUCCESS)
    }

    pub fn warning() -> Style {
        Style::default().fg(Theme::WARNING)
    }

    pub fn error() -> Style {
        Style::default().fg(Theme::ERROR)
    }

    pub fn risk_low() -> Style {
        Style::default().fg(Theme::RISK_LOW)
    }

    pub fn risk_medium() -> Style {
        Style::default().fg(Theme::RISK_MEDIUM)
    }

    pub fn risk_high() -> Style {
        Style::default().fg(Theme::RISK_HIGH)
    }

    pub fn tab_active() -> Style {
        Style::default()
            .fg(Theme::TAB_ACTIVE)
            .add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive() -> Style {
        Style::default().fg(Theme::TAB_INACTIVE)
    }

    pub fn header() -> Style {
        Style::default()
            .fg(Theme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn footer() -> Style {
        Style::default().fg(Theme::DIM)
    }
}

/// Get icon for a cleaner category.
pub fn category_icon(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "system_cache" | "systemcache" | "cache" => Theme::ICON_CACHE,
        "system_logs" | "systemlogs" | "logs" => Theme::ICON_LOGS,
        "trash" => Theme::ICON_TRASH,
        "xcode" => Theme::ICON_XCODE,
        "homebrew" | "brew" => Theme::ICON_HOMEBREW,
        "npm" => Theme::ICON_NPM,
        "yarn" => Theme::ICON_YARN,
        "cargo" | "rust" => Theme::ICON_CARGO,
        "pip" | "python" => Theme::ICON_PIP,
        "docker" => Theme::ICON_DOCKER,
        "apps" | "applications" => Theme::ICON_APPS,
        _ => Theme::ICON_SYSTEM,
    }
}
