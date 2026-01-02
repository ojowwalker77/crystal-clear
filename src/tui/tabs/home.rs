//! Home tab - two-column view for cleaning and app management.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::tui::app::{App, FocusPanel};
use crate::tui::theme::{Styles, Theme};
use crate::tui::widgets::{disk_gauge, format_size, loading_indicator};

/// Render the home tab.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    // Layout: disk bar (2 lines) | status (1 line) | two columns | summary (2 lines)
    let layout = Layout::vertical([
        Constraint::Length(2),  // Disk bar + label
        Constraint::Length(2),  // Status/progress
        Constraint::Min(5),     // Two-column area
        Constraint::Length(2),  // Selection summary
    ])
    .split(area);

    // Disk usage - compact
    render_disk_bar(app, frame, layout[0]);

    // Status line
    render_status(app, frame, layout[1]);

    // Two columns: cleanable items | apps
    let columns = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(layout[2]);

    render_clean_panel(app, frame, columns[0]);
    render_apps_panel(app, frame, columns[1]);

    // Selection summary
    render_summary(app, frame, layout[3]);
}

fn render_disk_bar(app: &App, frame: &mut Frame, area: Rect) {
    disk_gauge(
        frame,
        area,
        "Disk",
        app.dashboard.used_disk_space,
        app.dashboard.total_disk_space,
    );
}

fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    let is_scanning = app.scan.scanning || app.apps.scanning;

    let status = if app.dashboard.discovering {
        let msg = app
            .dashboard
            .discovery_progress
            .as_deref()
            .unwrap_or("Discovering...");
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Theme::WARNING)),
            Span::raw(msg),
        ])
    } else if is_scanning {
        let msg = app
            .scan
            .progress_message
            .as_deref()
            .unwrap_or("Scanning...");
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Theme::PRIMARY)),
            Span::raw(msg),
        ])
    } else if app.clean.items.is_empty() && app.apps.apps.is_empty() {
        Line::from(vec![
            Span::styled("  ", Styles::dim()),
            Span::raw("Press "),
            Span::styled("s", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" to scan"),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Theme::SUCCESS)),
            Span::raw(format!(
                "{} cleanable  |  {} apps  |  ",
                app.clean.items.len(),
                app.apps.apps.len()
            )),
            Span::styled("h/l", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" switch  "),
            Span::styled("Space", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" select  "),
            Span::styled("c", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" clean"),
        ])
    };

    // Add spinner for active operations
    if app.dashboard.discovering || is_scanning {
        let spinner_area = Rect::new(area.x, area.y, area.width, 1);
        loading_indicator(
            frame,
            spinner_area,
            if app.dashboard.discovering {
                app.dashboard.discovery_progress.as_deref().unwrap_or("Discovering...")
            } else {
                app.scan.progress_message.as_deref().unwrap_or("Scanning...")
            },
        );
    } else {
        frame.render_widget(Paragraph::new(status), area);
    }
}

fn render_clean_panel(app: &mut App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus_panel == FocusPanel::Left;
    let border_style = if is_focused {
        Style::default().fg(Theme::PRIMARY)
    } else {
        Style::default().fg(Theme::DIM)
    };

    if app.clean.items.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled("  No items", Styles::dim())]),
            Line::from(vec![Span::styled("  Press s to scan", Styles::dim())]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Cleanable "),
        );
        frame.render_widget(empty, area);
        return;
    }

    // Build table rows
    let rows: Vec<Row> = app
        .clean
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected = app.clean.table_state.selected() == Some(idx);
            let checkbox = if item.selected { "[x]" } else { "[ ]" };

            let style = if selected && is_focused {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if item.selected {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default()
            };

            Row::new(vec![
                checkbox.to_string(),
                truncate_path(&item.item.path.to_string_lossy(), 25),
                format_size(item.item.size),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),  // Checkbox
            Constraint::Min(15),    // Path
            Constraint::Length(10), // Size
        ],
    )
    .header(
        Row::new(vec!["", "Path", "Size"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Cleanable "),
    );

    frame.render_stateful_widget(table, area, &mut app.clean.table_state);
}

fn render_apps_panel(app: &mut App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus_panel == FocusPanel::Right;
    let border_style = if is_focused {
        Style::default().fg(Theme::PRIMARY)
    } else {
        Style::default().fg(Theme::DIM)
    };

    if app.apps.apps.is_empty() {
        let msg = if app.apps.scanning {
            "  Scanning apps..."
        } else {
            "  No apps found"
        };
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(msg, Styles::dim())]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Apps "),
        );
        frame.render_widget(empty, area);
        return;
    }

    // Build table rows
    let rows: Vec<Row> = app
        .apps
        .apps
        .iter()
        .enumerate()
        .map(|(idx, app_info)| {
            let selected = app.apps.table_state.selected() == Some(idx);
            let checked = app.apps.selected_for_uninstall.contains(&idx);
            let checkbox = if checked { "[x]" } else { "[ ]" };

            let total_size = app_info.size + app_info.leftover_size;
            let leftover_info = if app_info.leftover_size > 0 {
                format!("+{}", format_size(app_info.leftover_size))
            } else {
                String::new()
            };

            let style = if selected && is_focused {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if checked {
                Style::default().fg(Theme::WARNING)
            } else {
                Style::default()
            };

            Row::new(vec![
                checkbox.to_string(),
                truncate_str(&app_info.name, 20),
                format_size(total_size),
                leftover_info,
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),  // Checkbox
            Constraint::Min(12),    // Name
            Constraint::Length(10), // Size
            Constraint::Length(10), // Leftovers
        ],
    )
    .header(
        Row::new(vec!["", "App", "Size", "Extras"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Apps "),
    );

    frame.render_stateful_widget(table, area, &mut app.apps.table_state);
}

fn render_summary(app: &App, frame: &mut Frame, area: Rect) {
    let clean_selected = app.clean.selected_count > 0;
    let apps_selected = !app.apps.selected_for_uninstall.is_empty();

    let summary = if clean_selected || apps_selected {
        let mut parts = vec![Span::raw("  ")];

        if clean_selected {
            parts.push(Span::styled(
                format!("{} items", app.clean.selected_count),
                Style::default().fg(Theme::PRIMARY),
            ));
            parts.push(Span::raw(" ("));
            parts.push(Span::styled(
                format_size(app.clean.selected_size),
                Style::default().fg(Theme::SUCCESS),
            ));
            parts.push(Span::raw(")"));
        }

        if apps_selected {
            if clean_selected {
                parts.push(Span::raw("  +  "));
            }
            let apps_count = app.apps.selected_for_uninstall.len();
            let apps_size: u64 = app
                .apps
                .selected_for_uninstall
                .iter()
                .filter_map(|&i| app.apps.apps.get(i))
                .map(|a| a.size + a.leftover_size)
                .sum();
            parts.push(Span::styled(
                format!("{} apps", apps_count),
                Style::default().fg(Theme::WARNING),
            ));
            parts.push(Span::raw(" ("));
            parts.push(Span::styled(
                format_size(apps_size),
                Style::default().fg(Theme::SUCCESS),
            ));
            parts.push(Span::raw(")"));
        }

        parts.push(Span::raw("  |  Press "));
        parts.push(Span::styled("c", Style::default().fg(Theme::PRIMARY)));
        parts.push(Span::raw(" to clean"));

        Line::from(parts)
    } else {
        Line::from(vec![Span::styled("  No items selected", Styles::dim())])
    };

    frame.render_widget(Paragraph::new(summary), area);
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        path.to_string()
    } else {
        format!("...{}", &path[path.len().saturating_sub(max - 3)..])
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
