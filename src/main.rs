//! Crystal Clear - A safe macOS system cleaner TUI.

use std::io;

use crystal_clear_core::config;
use crystal_clear_core::tui::App;

fn main() -> io::Result<()> {
    // Initialize error handling
    color_eyre::install().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    // Load configuration
    let config = config::load_config(None).unwrap_or_else(|e| {
        eprintln!("Warning: Could not load config: {}. Using defaults.", e);
        config::Config::default()
    });

    if let Err(e) = config::validate_config(&config) {
        eprintln!("Warning: Config validation failed: {}", e);
    }

    // Initialize terminal
    let terminal = ratatui::init();

    // Run application
    let result = App::new(config).run(terminal);

    // Restore terminal
    ratatui::restore();

    result
}
