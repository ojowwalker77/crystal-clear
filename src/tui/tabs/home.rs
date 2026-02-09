//! Home tab - two-column view for cleaning and app management.

use chrono::Local;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::cleaners::RiskLevel;
use crate::tui::app::{App, FocusPanel, RowCleanStatus};
use crate::tui::theme::{Styles, Theme};
use crate::tui::widgets::{disk_gauge, format_size, loading_indicator};

/// Render the home tab.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    // Layout: disk bar | status | last report | two columns | selection summary
    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    render_disk_bar(app, frame, layout[0]);
    render_status(app, frame, layout[1]);
    render_report_strip(app, frame, layout[2]);

    let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[3]);
    render_clean_panel(app, frame, columns[0]);
    render_apps_panel(app, frame, columns[1]);

    render_summary(app, frame, layout[4]);
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
    } else if app.clean.cleaning {
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Theme::PRIMARY)),
            Span::raw("Cleaning selected items..."),
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
        let visible = app.visible_clean_indices().len();
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Theme::SUCCESS)),
            Span::raw(format!(
                "{} cleanable ({} shown)  |  {} apps  |  ",
                app.clean.items.len(),
                visible,
                app.apps.apps.len()
            )),
            Span::styled("h/l", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" switch  "),
            Span::styled("Space", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" select  "),
            Span::styled("z", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" 0 B"),
        ])
    };

    if app.dashboard.discovering || is_scanning || app.clean.cleaning {
        loading_indicator(
            frame,
            Rect::new(area.x, area.y, area.width, 1),
            if app.dashboard.discovering {
                app.dashboard
                    .discovery_progress
                    .as_deref()
                    .unwrap_or("Discovering...")
            } else if app.clean.cleaning {
                "Cleaning selected items..."
            } else {
                app.scan
                    .progress_message
                    .as_deref()
                    .unwrap_or("Scanning...")
            },
        );
    } else {
        frame.render_widget(Paragraph::new(status), area);
    }
}

fn render_report_strip(app: &App, frame: &mut Frame, area: Rect) {
    let line = if let Some(report) = &app.clean.clean_report {
        let completed_local = report.completed_at.with_timezone(&Local);
        Line::from(vec![
            Span::raw(" Last clean "),
            Span::styled(
                completed_local.format("%H:%M:%S").to_string(),
                Style::default().fg(Theme::PRIMARY),
            ),
            Span::raw("  "),
            Span::styled(
                format!("cleaned {}", report.cleaned_count),
                Style::default().fg(Theme::SUCCESS),
            ),
            Span::raw("  "),
            Span::styled(
                format!("failed {}", report.failed_count),
                if report.failed_count > 0 {
                    Styles::error()
                } else {
                    Styles::dim()
                },
            ),
            Span::raw("  "),
            Span::styled(
                format!("freed {}", format_size(report.bytes_freed)),
                Style::default().fg(Theme::SUCCESS),
            ),
            Span::raw("  |  "),
            Span::styled("v", Style::default().fg(Theme::PRIMARY)),
            Span::raw(" full report"),
        ])
    } else {
        Line::from(vec![
            Span::raw(" "),
            Span::styled("No clean report yet", Styles::dim()),
        ])
    };

    frame.render_widget(Paragraph::new(line), area);
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

    let visible = app.visible_clean_indices();
    if visible.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled("  All rows hidden (0 B)", Styles::dim())]),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("z", Style::default().fg(Theme::PRIMARY)),
                Span::raw(" to show"),
            ]),
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

    let rows: Vec<Row> = visible
        .iter()
        .enumerate()
        .map(|(visible_idx, item_idx)| {
            let item = &app.clean.items[*item_idx];
            let selected = app.clean.table_state.selected() == Some(visible_idx);
            let checkbox = if item.selected { "[x]" } else { "[ ]" };

            let mut style = match item.clean_status {
                Some(RowCleanStatus::Cleaned) => Style::default().fg(Theme::SUCCESS),
                Some(RowCleanStatus::Failed) => Style::default().fg(Theme::ERROR),
                Some(RowCleanStatus::Skipped) => Style::default().fg(Theme::WARNING),
                Some(RowCleanStatus::Pending) => Style::default().fg(Theme::PRIMARY),
                None => Style::default(),
            };
            if selected && is_focused {
                style = style.add_modifier(Modifier::REVERSED);
            } else if item.selected {
                style = style.add_modifier(Modifier::BOLD);
            }

            Row::new(vec![
                checkbox.to_string(),
                clean_status_label(item.clean_status).to_string(),
                risk_badge(item.item.risk_level).to_string(),
                category_short_label(&item.category).to_string(),
                truncate_path(&item.item.path.to_string_lossy(), 26),
                format_size(item.item.size),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),  // Checkbox
            Constraint::Length(6),  // Result
            Constraint::Length(3),  // Risk
            Constraint::Length(8),  // Category
            Constraint::Min(12),    // Path
            Constraint::Length(10), // Size
        ],
    )
    .header(
        Row::new(vec!["", "Res", "R", "Cat", "Path", "Size"])
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

            let mut style = Style::default();
            if checked {
                style = Style::default().fg(Theme::WARNING);
            }
            if selected && is_focused {
                style = style.add_modifier(Modifier::REVERSED);
            }

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
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(10),
            Constraint::Length(10),
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
    let apps_selected = app.apps.selected_for_uninstall.len();
    let apps_selected_size: u64 = app
        .apps
        .selected_for_uninstall
        .iter()
        .filter_map(|&idx| app.apps.apps.get(idx))
        .map(|item| item.size + item.leftover_size)
        .sum();

    let summary = Line::from(vec![
        Span::raw(" Selected "),
        Span::styled(
            format!("{} items", app.clean.selected_count),
            Style::default().fg(Theme::PRIMARY),
        ),
        Span::raw(" ("),
        Span::styled(
            format_size(app.clean.selected_size),
            Style::default().fg(Theme::SUCCESS),
        ),
        Span::raw(")  "),
        Span::styled(
            format!(
                "risk L/M/H {}/{}/{}",
                app.clean.selected_low_risk,
                app.clean.selected_medium_risk,
                app.clean.selected_high_risk
            ),
            Styles::dim(),
        ),
        Span::raw("  |  "),
        Span::styled(
            format!("{} apps", apps_selected),
            Style::default().fg(Theme::WARNING),
        ),
        Span::raw(" ("),
        Span::styled(
            format_size(apps_selected_size),
            Style::default().fg(Theme::SUCCESS),
        ),
        Span::raw(")  |  "),
        Span::raw("Press "),
        Span::styled("c", Style::default().fg(Theme::PRIMARY)),
        Span::raw(" clean"),
    ]);

    frame.render_widget(Paragraph::new(summary), area);
}

fn clean_status_label(status: Option<RowCleanStatus>) -> &'static str {
    match status {
        Some(RowCleanStatus::Pending) => "RUN",
        Some(RowCleanStatus::Cleaned) => "OK",
        Some(RowCleanStatus::Failed) => "FAIL",
        Some(RowCleanStatus::Skipped) => "SKIP",
        None => "",
    }
}

fn risk_badge(risk: RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "L",
        RiskLevel::Medium => "M",
        RiskLevel::High => "H",
    }
}

fn category_short_label(category: &str) -> &'static str {
    let key = category.to_lowercase();
    match key.as_str() {
        "system-cache" => "CACHE",
        "system-logs" => "LOGS",
        "trash" => "TRASH",
        "xcode" => "XCODE",
        "npm" => "NPM",
        "yarn" => "YARN",
        "cargo" => "CARGO",
        "pip" => "PIP",
        "homebrew" => "BREW",
        "docker" => "DOCKER",
        "apps" => "APPS",
        _ => "OTHER",
    }
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
