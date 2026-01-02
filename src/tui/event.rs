//! Event handling for the TUI.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Application events.
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    /// A key was pressed.
    Key(KeyEvent),
    /// A tick event for updating the UI.
    Tick,
    /// Terminal was resized.
    Resize(u16, u16),
}

/// Event handler configuration.
pub struct EventHandler {
    /// Tick rate for UI updates.
    tick_rate: Duration,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(100),
        }
    }
}

impl EventHandler {
    /// Create a new event handler with custom tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Poll for the next event.
    pub fn next(&self) -> std::io::Result<AppEvent> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => {
                    // Only handle key press events, not release
                    if key.kind == KeyEventKind::Press {
                        Ok(AppEvent::Key(key))
                    } else {
                        Ok(AppEvent::Tick)
                    }
                }
                Event::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}

/// Key action represents a user action triggered by a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    // Navigation
    NextTab,
    PrevTab,
    JumpToTab(usize),
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,

    // Selection
    Select,
    SelectAll,
    SelectNone,
    ToggleExpand,

    // Actions
    Confirm,
    Cancel,
    Refresh,
    StartScan,
    StartClean,
    Delete,
    Search,
    Filter,
    Save,
    Reset,
    Discover,

    // App
    Quit,
    Help,
    ForceQuit,

    // No action
    None,
}

/// Parse a key event into an action.
pub fn parse_key(key: KeyEvent) -> KeyAction {
    // Handle Ctrl+C as force quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return KeyAction::ForceQuit;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Esc => KeyAction::Cancel,

        // Tab navigation (Tab key only - h/l and arrows are for panel switching)
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                KeyAction::PrevTab
            } else {
                KeyAction::NextTab
            }
        }
        KeyCode::BackTab => KeyAction::PrevTab,
        // Left/Right for panel switching (handled in handle_home_action)
        KeyCode::Right | KeyCode::Char('l') => KeyAction::Right,
        KeyCode::Left | KeyCode::Char('h') => KeyAction::Left,

        // Jump to tab (1-6)
        KeyCode::Char('1') => KeyAction::JumpToTab(0),
        KeyCode::Char('2') => KeyAction::JumpToTab(1),
        KeyCode::Char('3') => KeyAction::JumpToTab(2),
        KeyCode::Char('4') => KeyAction::JumpToTab(3),
        KeyCode::Char('5') => KeyAction::JumpToTab(4),
        KeyCode::Char('6') => KeyAction::JumpToTab(5),

        // List navigation
        KeyCode::Char('j') | KeyCode::Down => KeyAction::Down,
        KeyCode::Char('k') | KeyCode::Up => KeyAction::Up,
        KeyCode::PageDown => KeyAction::PageDown,
        KeyCode::PageUp => KeyAction::PageUp,
        KeyCode::Char('g') => KeyAction::Home,
        KeyCode::Char('G') => KeyAction::End,

        // Selection
        KeyCode::Char(' ') => KeyAction::Select,
        KeyCode::Char('a') => KeyAction::SelectAll,
        KeyCode::Char('n') => KeyAction::SelectNone,
        KeyCode::Enter => KeyAction::Confirm,

        // Actions
        KeyCode::Char('s') => KeyAction::StartScan,
        KeyCode::Char('c') => KeyAction::StartClean,
        KeyCode::Char('r') => KeyAction::Refresh,
        KeyCode::Char('d') | KeyCode::Char('D') => KeyAction::Discover,
        KeyCode::Delete => KeyAction::Delete,
        KeyCode::Char('/') => KeyAction::Search,
        KeyCode::Char('f') => KeyAction::Filter,
        KeyCode::Char('S') => KeyAction::Save,
        KeyCode::Char('R') => KeyAction::Reset,

        // Help
        KeyCode::Char('?') => KeyAction::Help,

        // Expand/collapse
        KeyCode::Char('e') => KeyAction::ToggleExpand,

        _ => KeyAction::None,
    }
}

/// Get help text for current context.
pub fn get_help_text(tab: &str) -> Vec<(&'static str, &'static str)> {
    let mut help = vec![
        ("Tab", "Switch"),
        ("q", "Quit"),
    ];

    match tab {
        "home" => {
            help.extend([
                ("s", "Scan"),
                ("j/k", "Navigate"),
                ("Space", "Select"),
                ("a/n", "All/None"),
                ("c", "Clean"),
            ]);
        }
        "settings" => {
            help.extend([
                ("j/k", "Navigate"),
                ("Space", "Toggle"),
                ("S", "Save"),
            ]);
        }
        _ => {}
    }

    help
}
