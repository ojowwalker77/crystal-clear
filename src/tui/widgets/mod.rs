//! Custom widgets for the TUI.

pub mod help_footer;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Gauge};
use ratatui::Frame;

use super::theme::{Styles, Theme};

/// Render a disk usage gauge with label below the bar.
pub fn disk_gauge(frame: &mut Frame, area: Rect, _title: &str, used: u64, total: u64) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::widgets::Paragraph;
    use ratatui::layout::Alignment;

    let percentage = if total > 0 {
        ((used as f64 / total as f64) * 100.0) as u16
    } else {
        0
    };

    let color = if percentage > 90 {
        Theme::ERROR
    } else if percentage > 75 {
        Theme::WARNING
    } else {
        Theme::SUCCESS
    };

    // Split area: bar on top, label below
    let chunks = Layout::vertical([
        Constraint::Length(1), // Bar
        Constraint::Length(1), // Label
    ])
    .split(area);

    // Render bar (no label inside)
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .ratio(used as f64 / total.max(1) as f64);

    frame.render_widget(gauge, chunks[0]);

    // Render centered label below
    let label = format!(
        "{} / {} ({}%)",
        format_size(used),
        format_size(total),
        percentage
    );
    let label_widget = Paragraph::new(label)
        .style(Style::default().fg(Theme::FG))
        .alignment(Alignment::Center);

    frame.render_widget(label_widget, chunks[1]);
}

/// Render a simple bar chart for categories.
pub fn category_bar(
    frame: &mut Frame,
    area: Rect,
    categories: &[(String, u64)],
    max_items: usize,
) {
    use ratatui::widgets::Paragraph;

    if categories.is_empty() {
        let empty = Paragraph::new("No data. Press 's' to scan.")
            .style(Styles::dim())
            .block(Block::default().borders(Borders::ALL).title(" Categories "));
        frame.render_widget(empty, area);
        return;
    }

    // Find max for scaling
    let max_size = categories.iter().map(|(_, s)| *s).max().unwrap_or(1);
    let bar_width = area.width.saturating_sub(30) as usize;

    let mut lines: Vec<Line> = Vec::new();

    for (name, size) in categories.iter().take(max_items) {
        let bar_len = if max_size > 0 {
            (*size as f64 / max_size as f64 * bar_width as f64) as usize
        } else {
            0
        };

        let bar: String = "█".repeat(bar_len);
        let empty: String = "░".repeat(bar_width.saturating_sub(bar_len));

        let icon = super::theme::category_icon(name);
        let name_display = format!("{} {:12}", icon, truncate(name, 12));
        let size_display = format_size(*size);

        lines.push(Line::from(vec![
            ratatui::text::Span::raw(name_display),
            ratatui::text::Span::styled(bar, Style::default().fg(Theme::PRIMARY)),
            ratatui::text::Span::styled(empty, Styles::dim()),
            ratatui::text::Span::raw(format!(" {:>8}", size_display)),
        ]));
    }

    if categories.len() > max_items {
        lines.push(Line::from(format!(
            "  ... and {} more",
            categories.len() - max_items
        )));
    }

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Categories "),
    );
    frame.render_widget(para, area);
}

/// Format bytes as human-readable size.
pub fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string()
}

/// Truncate a string to max length.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

/// Render a loading spinner.
pub fn loading_indicator(frame: &mut Frame, area: Rect, message: &str) {
    use ratatui::widgets::Paragraph;

    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 100) as usize
        % spinner_frames.len();

    let text = format!("{} {}", spinner_frames[idx], message);
    let para = Paragraph::new(text).style(Style::default().fg(Theme::PRIMARY));
    frame.render_widget(para, area);
}
