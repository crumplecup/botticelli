//! UI rendering for TUI.

use crate::app::{App, AppMode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

/// Draw the main UI.
#[tracing::instrument(skip_all)]
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    // Draw header
    draw_header(f, app, chunks[0]);

    // Draw main content based on mode
    match app.mode {
        AppMode::List => draw_list_view(f, app, chunks[1]),
        AppMode::Detail => draw_detail_view(f, app, chunks[1]),
        AppMode::Edit => draw_edit_view(f, app, chunks[1]),
        AppMode::Compare => draw_compare_view(f, app, chunks[1]),
        AppMode::Export => draw_export_view(f, app, chunks[1]),
    }

    // Draw status bar
    draw_status_bar(f, app, chunks[2]);
}

/// Draw the header.
#[tracing::instrument(skip_all)]
fn draw_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = format!("Botticelli Content Review - {}", app.table_name);
    let header = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(header, area);
}

/// Draw the status bar with help text.
#[tracing::instrument(skip_all)]
fn draw_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help_text = match app.mode {
        AppMode::List => {
            "↑↓: Navigate | Enter: Detail | E: Edit | C: Compare | D: Delete | R: Reload | Q: Quit"
        }
        AppMode::Detail => "Esc: Back | E: Edit | Q: Quit",
        AppMode::Edit => "Ctrl+Enter: Save | Esc: Cancel",
        AppMode::Compare => "Esc: Back | Q: Quit",
        AppMode::Export => "Esc: Back | Q: Quit",
    };

    let status_text = format!("{} | {}", app.status_message, help_text);
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(status, area);
}

/// Draw the list view.
#[tracing::instrument(skip_all)]
fn draw_list_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let header = Row::new(vec!["ID", "Status", "Rating", "Tags", "Preview"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .content_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let rating_str = if let Some(r) = item.rating {
                "★".repeat(r as usize) + &"☆".repeat(5 - r as usize)
            } else {
                "---".to_string()
            };

            let tags_str = item.tags.join(", ");

            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if app.compare_selection.contains(&i) {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Row::new(vec![
                item.id.to_string(),
                item.review_status.clone(),
                rating_str,
                tags_str,
                item.preview.clone(),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Content List"))
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(table, area);
}

/// Draw the detail view.
#[tracing::instrument(skip_all)]
fn draw_detail_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if let Some(item) = app.content_items.get(app.selected_index) {
        let content_json = serde_json::to_string_pretty(&item.content).unwrap_or_default();

        let details = vec![
            format!("ID: {}", item.id),
            format!("Status: {}", item.review_status),
            format!(
                "Rating: {}",
                item.rating
                    .map(|r| "★".repeat(r as usize))
                    .unwrap_or_else(|| "---".to_string())
            ),
            format!("Tags: {}", item.tags.join(", ")),
            format!(
                "Narrative: {}",
                item.source_narrative.as_deref().unwrap_or("N/A")
            ),
            format!("Act: {}", item.source_act.as_deref().unwrap_or("N/A")),
            String::new(),
            "Content:".to_string(),
            content_json,
        ];

        let detail = Paragraph::new(details.join("\n"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Content Detail"),
            )
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(detail, area);
    }
}

/// Draw the edit view.
#[tracing::instrument(skip_all)]
fn draw_edit_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if let Some(buffer) = &app.edit_buffer {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .margin(2)
            .split(area);

        // Tags field
        let tags_block = Block::default()
            .borders(Borders::ALL)
            .title("Tags (comma-separated)");
        let tags = Paragraph::new(buffer.tags.as_str()).block(tags_block);
        f.render_widget(tags, chunks[0]);

        // Rating field
        let rating_block = Block::default().borders(Borders::ALL).title("Rating (1-5)");
        let rating_text = buffer
            .rating
            .map(|r| r.to_string())
            .unwrap_or_else(|| "None".to_string());
        let rating = Paragraph::new(rating_text).block(rating_block);
        f.render_widget(rating, chunks[1]);

        // Status field
        let status_block = Block::default()
            .borders(Borders::ALL)
            .title("Status (pending/approved/rejected)");
        let status = Paragraph::new(buffer.status.as_str()).block(status_block);
        f.render_widget(status, chunks[2]);
    }
}

/// Draw the compare view (side-by-side comparison).
#[tracing::instrument(skip_all)]
fn draw_compare_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if app.compare_selection.len() >= 2 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left panel
        if let Some(&idx) = app.compare_selection.first()
            && let Some(item) = app.content_items.get(idx)
        {
            let content_json = serde_json::to_string_pretty(&item.content).unwrap_or_default();
            let details = [
                format!("ID: {}", item.id),
                format!("Status: {}", item.review_status),
                format!("Rating: {:?}", item.rating),
                format!("Tags: {}", item.tags.join(", ")),
                String::new(),
                content_json,
            ];

            let left = Paragraph::new(details.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("Item 1"))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(left, chunks[0]);
        }

        // Right panel
        if let Some(&idx) = app.compare_selection.get(1)
            && let Some(item) = app.content_items.get(idx)
        {
            let content_json = serde_json::to_string_pretty(&item.content).unwrap_or_default();
            let details = [
                format!("ID: {}", item.id),
                format!("Status: {}", item.review_status),
                format!("Rating: {:?}", item.rating),
                format!("Tags: {}", item.tags.join(", ")),
                String::new(),
                content_json,
            ];

            let right = Paragraph::new(details.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("Item 2"))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(right, chunks[1]);
        }
    }
}

/// Draw the export view.
#[tracing::instrument(skip_all)]
fn draw_export_view(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let export_text = "Export functionality coming soon...";
    let export = Paragraph::new(export_text)
        .block(Block::default().borders(Borders::ALL).title("Export"))
        .alignment(Alignment::Center);
    f.render_widget(export, area);
}
