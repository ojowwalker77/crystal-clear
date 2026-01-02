//! Settings tab - configuration editor.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::tui::app::{App, SettingValue};
use crate::tui::theme::{Styles, Theme};

/// Render the settings tab.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.settings.fields.is_empty() {
        render_empty(frame, area);
        return;
    }

    render_settings_table(app, frame, area);
}

/// Render empty state.
fn render_empty(frame: &mut Frame, area: Rect) {
    let text = "No settings available.";
    let para = Paragraph::new(text)
        .style(Styles::dim())
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(para, area);
}

/// Render the settings table.
fn render_settings_table(app: &mut App, frame: &mut Frame, area: Rect) {
    let mut rows: Vec<Row> = Vec::new();
    let mut current_category = String::new();

    for (idx, field) in app.settings.fields.iter().enumerate() {
        // Add category header if changed
        if field.category != current_category {
            current_category = field.category.clone();
            rows.push(
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(&*field.category).style(Styles::header()),
                    Cell::from(""),
                ])
                .height(1),
            );
        }

        let value_display = match &field.value {
            SettingValue::Bool(v) => {
                if *v {
                    "[x] Enabled".to_string()
                } else {
                    "[ ] Disabled".to_string()
                }
            }
            SettingValue::Number(v) => format!("{}", v),
            SettingValue::String(v) => v.clone(),
        };

        let value_style = match &field.value {
            SettingValue::Bool(true) => Styles::success(),
            SettingValue::Bool(false) => Styles::dim(),
            _ => Style::default(),
        };

        let is_selected = idx == app.settings.selected_field;
        let prefix = if is_selected { ">" } else { " " };

        let row = Row::new(vec![
            Cell::from(prefix),
            Cell::from(&*field.name),
            Cell::from(value_display).style(value_style),
        ]);

        rows.push(row);
    }

    let header = Row::new(vec![
        Cell::from("").style(Styles::header()),
        Cell::from("Setting").style(Styles::header()),
        Cell::from("Value").style(Styles::header()),
    ])
    .height(1)
    .bottom_margin(1);

    let widths = [
        Constraint::Length(2),
        Constraint::Length(25),
        Constraint::Min(20),
    ];

    let title = if app.settings.unsaved_changes {
        " Settings [Modified] "
    } else {
        " Settings "
    };

    let title_style = if app.settings.unsaved_changes {
        Style::default()
            .fg(Theme::WARNING)
            .add_modifier(Modifier::BOLD)
    } else {
        Styles::title()
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(title_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, area);

    // Render description for selected field
    if let Some(field) = app.settings.fields.get(app.settings.selected_field) {
        let desc_area = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(3),
            area.width,
            2,
        );
        let desc = Paragraph::new(&*field.description)
            .style(Styles::dim())
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(desc, desc_area);
    }
}
