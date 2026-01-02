//! Main UI rendering.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;
use strum::IntoEnumIterator;

use super::app::{App, AppMode, ConfirmAction, Tab};
use super::event::get_help_text;
use super::tabs;
use super::theme::{Styles, Theme};

/// Render the entire UI.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // Main layout: header, content, footer
    let layout = Layout::vertical([
        Constraint::Length(3), // Header with tabs
        Constraint::Min(0),    // Content
        Constraint::Length(1), // Footer with key hints
    ])
    .split(area);

    render_header(app, frame, layout[0]);
    render_content(app, frame, layout[1]);
    render_footer(app, frame, layout[2]);

    // Render overlays
    match app.mode {
        AppMode::Confirming(action) => render_confirm_dialog(app, frame, action),
        AppMode::ShowingHelp => render_help_overlay(app, frame),
        _ => {}
    }
}

/// Render the header with tabs.
fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let titles: Vec<Line> = Tab::iter()
        .map(|t| {
            let style = if t == app.current_tab {
                Styles::tab_active()
            } else {
                Styles::tab_inactive()
            };
            Line::from(Span::styled(format!(" {} ", t), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" CleanMac ")
                .title_style(Styles::title()),
        )
        .select(app.current_tab as usize)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

/// Render the main content area.
fn render_content(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match app.current_tab {
        Tab::Home => tabs::home::render(app, frame, inner),
        Tab::Settings => tabs::settings::render(app, frame, inner),
    }
}

/// Render the footer with key hints.
fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let help = get_help_text(app.current_tab.id());
    let spans: Vec<Span> = help
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), Style::default().fg(Theme::PRIMARY)),
                Span::styled(format!("{} ", desc), Styles::dim()),
                Span::raw("│"),
            ]
        })
        .collect();

    let mut line = Line::from(spans);
    // Remove trailing separator
    if let Some(last) = line.spans.last_mut() {
        if last.content == "│" {
            line.spans.pop();
        }
    }

    // Add status message if present
    let footer_line = if let Some((msg, level, _)) = &app.status_message {
        let style = match level {
            super::app::StatusLevel::Info => Style::default().fg(Theme::PRIMARY),
            super::app::StatusLevel::Success => Styles::success(),
            super::app::StatusLevel::Warning => Styles::warning(),
            super::app::StatusLevel::Error => Styles::error(),
        };
        Line::from(vec![
            Span::styled(format!(" {} ", msg), style),
            Span::raw("│"),
        ])
        .patch_style(Styles::footer())
    } else {
        line
    };

    let footer = Paragraph::new(footer_line).style(Styles::footer());
    frame.render_widget(footer, area);
}

/// Render confirmation dialog.
fn render_confirm_dialog(_app: &App, frame: &mut Frame, action: ConfirmAction) {
    let area = frame.area();

    // Calculate dialog size and position
    let dialog_width = 50;
    let dialog_height = 8;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let (title, message) = match action {
        ConfirmAction::CleanSelected => (
            "Confirm Clean",
            "Are you sure you want to clean the selected items?\nFiles will be moved to Trash.",
        ),
        ConfirmAction::CleanAll => (
            "Confirm Clean All",
            "Are you sure you want to clean ALL items?\nThis may take a while.",
        ),
        ConfirmAction::ClearAuditLog => (
            "Clear Audit Log",
            "Are you sure you want to clear the audit log?\nThis action cannot be undone.",
        ),
        ConfirmAction::ResetSettings => (
            "Reset Settings",
            "Are you sure you want to reset all settings\nto their default values?",
        ),
    };

    let text = format!("{}\n\n[y] Yes  [n/Esc] Cancel", message);
    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .title_style(Styles::warning()),
        )
        .style(Style::default().fg(Theme::FG));

    frame.render_widget(dialog, dialog_area);
}

/// Render help overlay.
fn render_help_overlay(_app: &App, frame: &mut Frame) {
    let area = frame.area();

    // Calculate help size and position
    let help_width = 60;
    let help_height = 20;
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;
    let help_area = Rect::new(x, y, help_width, help_height);

    // Clear background
    frame.render_widget(Clear, help_area);

    let help_text = r#"
  Global Keys
  ───────────────────────────────
  Tab/l/h      Switch tabs
  1-6          Jump to tab
  q            Quit
  ?            Toggle help
  Esc          Cancel/Close

  Navigation
  ───────────────────────────────
  j/k/↑/↓      Move up/down
  Enter        Select/Expand
  Space        Toggle selection
  a            Select all
  n            Select none

  Actions
  ───────────────────────────────
  s            Start scan
  c            Clean selected
  r            Refresh

  Press any key to close
"#;

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .title_style(Styles::title()),
    );

    frame.render_widget(help, help_area);
}
