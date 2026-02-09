//! Help footer widget showing context-sensitive key hints.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::theme::{Styles, Theme};

/// Key hint for display in footer.
pub struct KeyHint {
    pub key: &'static str,
    pub description: &'static str,
}

impl KeyHint {
    pub fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

/// Render a help footer with key hints.
pub fn render(frame: &mut Frame, area: Rect, hints: &[KeyHint]) {
    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|hint| {
            vec![
                Span::styled(
                    format!(" {} ", hint.key),
                    Style::default().fg(Theme::PRIMARY),
                ),
                Span::styled(format!("{} ", hint.description), Styles::dim()),
                Span::raw("│"),
            ]
        })
        .collect();

    let mut line = Line::from(spans);
    // Remove trailing separator
    if let Some(last) = line.spans.last() {
        if last.content == "│" {
            line.spans.pop();
        }
    }

    let footer = Paragraph::new(line).style(Styles::footer());
    frame.render_widget(footer, area);
}
