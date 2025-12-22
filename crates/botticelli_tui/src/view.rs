use ratatui::Frame;

use crate::{AppState, Command, TuiResult};

/// Trait for TUI views.
pub trait View {
    /// Renders the view to the terminal frame.
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()>;

    /// Handles keyboard input and returns a command if applicable.
    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        state: &AppState,
    ) -> TuiResult<Option<Command>>;
}

/// Chat view implementation.
#[derive(Debug, Default)]
pub struct ChatView;

impl View for ChatView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(frame.area());

        // Message area - render messages with styling
        let message_lines = if let Some(conv_id) = state.current_conversation() {
            if let Some(msgs) = state.conversation_messages(conv_id) {
                let mut lines = Vec::new();
                for msg in msgs {
                    // Match based on role field
                    let (prefix, color) = match msg.role.as_str() {
                        "user" => ("You: ", Color::Green),
                        "assistant" => ("Assistant: ", Color::Blue),
                        "system" => ("System: ", Color::Yellow),
                        _ => ("Unknown: ", Color::White),
                    };
                    
                    lines.push(Line::from(vec![
                        Span::styled(
                            prefix,
                            Style::default()
                                .fg(color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&msg.content),
                    ]));
                    // Add blank line between messages
                    lines.push(Line::from(""));
                }
                lines
            } else {
                vec![Line::from("No messages")]
            }
        } else {
            vec![Line::from("No conversation selected")]
        };

        let messages_widget = Paragraph::new(message_lines)
            .block(Block::default().title("Chat").borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(messages_widget, chunks[0]);

        // Input area
        let input_widget = Paragraph::new(state.input_buffer().as_str())
            .block(Block::default().title("Input").borders(Borders::ALL));
        frame.render_widget(input_widget, chunks[1]);

        Ok(())
    }

    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        state: &AppState,
    ) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Enter, KeyModifiers::NONE) => {
                let input = state.input_buffer().to_string();
                if !input.is_empty() {
                    Ok(Some(Command::SendMessage(input)))
                } else {
                    Ok(None)
                }
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => Ok(Some(Command::DeleteChar)),
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                Ok(Some(Command::AppendChar(c)))
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Ok(Some(Command::ClearConversation)),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Some(Command::Quit)),
            _ => Ok(None),
        }
    }
}

/// Narrative browser view implementation.
#[derive(Debug, Default)]
pub struct NarrativeBrowserView;

impl View for NarrativeBrowserView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(frame.area());

        // Narrative list
        let narratives: Vec<ListItem> = state
            .narrative_list()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let prefix = if Some(i) == *state.selected_narrative() {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(format!("{}{}", prefix, name))
            })
            .collect();

        let list_widget =
            List::new(narratives).block(Block::default().title("Narratives").borders(Borders::ALL));
        frame.render_widget(list_widget, chunks[0]);

        // Preview area
        let preview_text = if let Some(idx) = state.selected_narrative() {
            if let Some(name) = state.narrative_list().get(*idx) {
                format!("Preview of: {}\n\n(Full preview to be implemented)", name)
            } else {
                String::from("No narrative selected")
            }
        } else {
            String::from("Select a narrative to preview")
        };

        let preview_widget = Paragraph::new(preview_text)
            .block(Block::default().title("Preview").borders(Borders::ALL));
        frame.render_widget(preview_widget, chunks[1]);

        Ok(())
    }

    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        _state: &AppState,
    ) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Some(Command::Quit)),
            (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => Ok(Some(Command::NavigateUp)),
            (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
                Ok(Some(Command::NavigateDown))
            }
            (KeyCode::Enter, KeyModifiers::NONE) => Ok(Some(Command::SelectNarrative)),
            _ => Ok(None),
        }
    }
}

/// Narrative editor view implementation.
#[derive(Debug, Default)]
pub struct NarrativeEditorView;

impl View for NarrativeEditorView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title bar
        let title_text = if let Some(idx) = state.selected_narrative() {
            if let Some(name) = state.narrative_list().get(*idx) {
                format!("Editing: {}", name)
            } else {
                String::from("No narrative loaded")
            }
        } else {
            String::from("No narrative selected")
        };

        let title_widget = Paragraph::new(title_text).block(
            Block::default()
                .title("Narrative Editor")
                .borders(Borders::ALL),
        );
        frame.render_widget(title_widget, chunks[0]);

        // Editor content
        let content_text = state.editor_content();
        let content_widget = Paragraph::new(content_text.as_str())
            .block(Block::default().title("Content").borders(Borders::ALL));
        frame.render_widget(content_widget, chunks[1]);

        // Status bar
        let status_text = "Ctrl+S: Save | Ctrl+C: Quit | Esc: Back to Browser";
        let status_widget =
            Paragraph::new(status_text).block(Block::default().borders(Borders::ALL));
        frame.render_widget(status_widget, chunks[2]);

        Ok(())
    }

    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        _state: &AppState,
    ) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Some(Command::Quit)),
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Ok(Some(Command::SaveNarrative)),
            (KeyCode::Esc, KeyModifiers::NONE) => {
                Ok(Some(Command::SwitchMode(crate::ViewMode::NarrativeBrowser)))
            }
            _ => Ok(None),
        }
    }
}

/// Conversation history browser view implementation.
#[derive(Debug, Default)]
pub struct ConversationHistoryView;

impl View for ConversationHistoryView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(frame.area());

        // Left panel: Conversation list
        let conversation_ids = state.conversation_ids();
        let items: Vec<ListItem> = conversation_ids
            .iter()
            .enumerate()
            .map(|(idx, id)| {
                let message_count = state
                    .conversation_messages(id)
                    .map(|msgs| msgs.len())
                    .unwrap_or(0);

                let content = format!("{} ({} messages)", id, message_count);

                let style = if Some(idx) == *state.selected_conversation_history() {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Conversation History"),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(list, chunks[0]);

        // Right panel: Preview of selected conversation
        let preview_text = if let Some(idx) = state.selected_conversation_history() {
            if let Some(id) = conversation_ids.get(*idx) {
                if let Some(messages) = state.conversation_messages(id) {
                    let mut lines = Vec::new();
                    for msg in messages.iter().take(10) {
                        let (prefix, style, content) = match msg.role.as_str() {
                            "user" => (
                                "You: ",
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                                &msg.content,
                            ),
                            "assistant" => (
                                "Bot: ",
                                Style::default()
                                    .fg(Color::Blue)
                                    .add_modifier(Modifier::BOLD),
                                &msg.content,
                            ),
                            _ => (
                                "System: ",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                                &msg.content,
                            ),
                        };
                        
                        lines.push(Line::from(vec![
                            Span::styled(prefix, style),
                            Span::raw(content),
                        ]));
                    }

                    if messages.len() > 10 {
                        lines.push(Line::from(vec![Span::styled(
                            format!("... ({} more messages)", messages.len() - 10),
                            Style::default().fg(Color::Gray),
                        )]));
                    }

                    lines
                } else {
                    vec![Line::from("No messages")]
                }
            } else {
                vec![Line::from("No conversation selected")]
            }
        } else {
            vec![Line::from("Select a conversation to preview")]
        };

        let preview = Paragraph::new(preview_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Preview (first 10 messages)"),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(preview, chunks[1]);

        Ok(())
    }

    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        _state: &AppState,
    ) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Some(Command::Quit)),
            (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => Ok(Some(Command::NavigateUp)),
            (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
                Ok(Some(Command::NavigateDown))
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                // Load the selected conversation
                Ok(Some(Command::SelectNarrative)) // We'll reuse this command
            }
            (KeyCode::Char('d'), KeyModifiers::NONE) => {
                // Delete the selected conversation
                Ok(Some(Command::ClearConversation)) // We'll handle this differently in the handler
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                Ok(Some(Command::SwitchMode(crate::ViewMode::Chat)))
            }
            _ => Ok(None),
        }
    }
}

/// Settings view implementation.
#[derive(Debug, Default)]
pub struct SettingsView;

impl View for SettingsView {
    fn render(&self, frame: &mut Frame, _state: &AppState) -> TuiResult<()> {
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, Borders, Paragraph};
        
        let title = Paragraph::new("Settings\n\n(To be implemented)")
            .block(Block::default()
                .title("Settings")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));
        frame.render_widget(title, frame.area());
        
        Ok(())
    }
    
    fn handle_input(&self, key: crossterm::event::KeyEvent, _state: &AppState) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        match (key.code, key.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) => {
                Ok(Some(Command::SwitchMode(crate::ViewMode::Chat)))
            }
            _ => Ok(None),
        }
    }
}

/// Bot status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotStatus {
    /// Bot is running.
    Running,
    /// Bot is stopped.
    Stopped,
    /// Bot configuration only.
    Configured,
}

/// Bot information for display.
#[derive(Debug, Clone)]
pub struct BotInfo {
    /// Bot name.
    pub name: String,
    /// Bot description.
    pub description: Option<String>,
    /// Platform.
    pub platform: String,
    /// Current status.
    pub status: BotStatus,
    /// Configuration path.
    pub config_path: Option<String>,
}

/// Bots management view.
#[derive(Debug, Default)]
pub struct BotsView;

impl View for BotsView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        
        let area = frame.area();
        
        // Split into list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Bot list
                Constraint::Percentage(60), // Bot details
            ])
            .split(area);

        // Render bot list
        self.render_bot_list(frame, chunks[0], state)?;

        // Render bot details
        self.render_bot_details(frame, chunks[1], state)?;

        Ok(())
    }

    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        _state: &AppState,
    ) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => {
                Ok(Some(Command::NavigateUp))
            }
            (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
                Ok(Some(Command::NavigateDown))
            }
            (KeyCode::Char('s'), KeyModifiers::NONE) => {
                // TODO: Start bot command
                Ok(None)
            }
            (KeyCode::Char('x'), KeyModifiers::NONE) => {
                // TODO: Stop bot command
                Ok(None)
            }
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                // TODO: Restart bot command
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

impl BotsView {
    /// Renders the bot list.
    fn render_bot_list(&self, frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) -> TuiResult<()> {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
        
        // Get bots from state
        let bots = state.bots();
        
        if bots.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No bots configured"),
                Line::from(""),
                Line::from("Press 'a' to add a bot"),
            ])
            .block(Block::default().title("Bots").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
            
            frame.render_widget(empty, area);
            return Ok(());
        }

        // Create list items
        let items: Vec<ListItem> = bots
            .iter()
            .enumerate()
            .map(|(i, bot)| {
                let status_color = match bot.status {
                    BotStatus::Running => Color::Green,
                    BotStatus::Stopped => Color::Red,
                    BotStatus::Configured => Color::Yellow,
                };

                let status_text = match bot.status {
                    BotStatus::Running => "●",
                    BotStatus::Stopped => "○",
                    BotStatus::Configured => "◐",
                };

                let prefix = if *state.selected_bot() == Some(i) {
                    "> "
                } else {
                    "  "
                };

                let content = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(status_text, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::raw(&bot.name),
                    Span::raw(" "),
                    Span::styled(
                        format!("[{}]", bot.platform),
                        Style::default().fg(Color::Cyan),
                    ),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Bots (↑/↓ navigate, s=start, x=stop, r=restart)")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(list, area);
        Ok(())
    }

    /// Renders bot details panel.
    fn render_bot_details(&self, frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) -> TuiResult<()> {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
        
        let bots = state.bots();
        
        let bot = state
            .selected_bot()
            .and_then(|idx| bots.get(idx));

        if let Some(bot) = bot {
            let mut lines = vec![];

            // Name
            lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&bot.name),
            ]));

            // Platform
            lines.push(Line::from(vec![
                Span::styled("Platform: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&bot.platform),
            ]));

            // Status
            let (status_text, status_color) = match bot.status {
                BotStatus::Running => ("Running", Color::Green),
                BotStatus::Stopped => ("Stopped", Color::Red),
                BotStatus::Configured => ("Configured", Color::Yellow),
            };
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]));

            // Description
            if let Some(desc) = &bot.description {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Description:", Style::default().add_modifier(Modifier::BOLD)),
                ]));
                lines.push(Line::from(desc.clone()));
            }

            // Config path
            if let Some(path) = &bot.config_path {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Config: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(path, Style::default().fg(Color::Cyan)),
                ]));
            }

            let details = Paragraph::new(lines)
                .block(Block::default().title("Bot Details").borders(Borders::ALL))
                .wrap(Wrap { trim: false });

            frame.render_widget(details, area);
        } else {
            let empty = Paragraph::new("No bot selected")
                .block(Block::default().title("Bot Details").borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));

            frame.render_widget(empty, area);
        }

        Ok(())
    }
}

// ============================================================================
// DatabaseView - Database browser with schema and content viewing
// ============================================================================

use serde_json::Value as JsonValue;

/// Database view mode within the database browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseViewMode {
    /// Browsing list of tables.
    Tables,
    /// Viewing table schema.
    Schema,
    /// Browsing table content.
    Content,
}

/// Table information.
#[derive(Debug, Clone)]
pub struct TableInfo {
    /// Table name.
    pub name: String,
    /// Row count (if known).
    pub row_count: Option<i64>,
    /// Whether this is a generated content table.
    pub is_content_table: bool,
}

impl TableInfo {
    /// Creates a new table info.
    pub fn new(name: String) -> Self {
        let is_content_table = name.starts_with("content_")
            || name.starts_with("generated_")
            || ![
                "narratives",
                "narrative_executions",
                "act_executions",
                "act_inputs",
                "model_responses",
                "actor_server_state",
                "actor_server_executions",
            ]
            .contains(&name.as_str());

        Self {
            name,
            row_count: None,
            is_content_table,
        }
    }

    /// Sets the row count.
    pub fn with_row_count(mut self, count: i64) -> Self {
        self.row_count = Some(count);
        self
    }
}

/// Column information for schema display.
#[derive(Debug, Clone)]
pub struct ColumnDisplay {
    /// Column name.
    pub name: String,
    /// Data type.
    pub data_type: String,
    /// Whether nullable.
    pub nullable: bool,
    /// Default value.
    pub default: Option<String>,
}

/// Content row for display.
#[derive(Debug, Clone)]
pub struct ContentRow {
    /// Row data as JSON.
    pub data: JsonValue,
}

/// Filter options for content browsing.
#[derive(Debug, Clone, Default)]
pub struct ContentFilter {
    /// Review status filter ("pending", "approved", "rejected", or None for all).
    pub review_status: Option<String>,
}

/// Database browser view.
#[derive(Debug, Default)]
pub struct DatabaseView;

impl View for DatabaseView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

        let area = frame.area();
        
        match state.database_view_mode() {
            DatabaseViewMode::Tables => self.render_tables(frame, area, state)?,
            DatabaseViewMode::Schema => self.render_schema(frame, area, state)?,
            DatabaseViewMode::Content => self.render_content(frame, area, state)?,
        }

        Ok(())
    }

    fn handle_input(
        &self,
        key: crossterm::event::KeyEvent,
        _state: &AppState,
    ) -> TuiResult<Option<Command>> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Char('t'), KeyModifiers::NONE) => {
                Ok(Some(Command::DatabaseShowTables))
            }
            (KeyCode::Char('s'), KeyModifiers::NONE) => {
                Ok(Some(Command::DatabaseShowSchema))
            }
            (KeyCode::Char('c'), KeyModifiers::NONE) => {
                Ok(Some(Command::DatabaseShowContent))
            }
            (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => {
                Ok(Some(Command::NavigateUp))
            }
            (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
                Ok(Some(Command::NavigateDown))
            }
            (KeyCode::Char('f'), KeyModifiers::NONE) => {
                Ok(Some(Command::DatabaseCycleFilter))
            }
            (KeyCode::Char('l'), KeyModifiers::NONE) => {
                Ok(Some(Command::DatabaseLoadTables))
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                Ok(Some(Command::DatabaseSelectTable))
            }
            _ => Ok(None),
        }
    }
}

impl DatabaseView {
    /// Renders the tables list view.
    fn render_tables(&self, frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),       // Table list
                Constraint::Length(3),    // Status bar
            ])
            .split(area);

        // Render table list
        let tables = state.database_tables();
        
        if tables.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from("No tables loaded"),
                Line::from(""),
                Line::from("Press 'l' to load tables from database"),
            ])
            .block(Block::default().title("Database Tables").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
            
            frame.render_widget(empty, chunks[0]);
        } else {
            let items: Vec<ListItem> = tables
                .iter()
                .enumerate()
                .map(|(i, table)| {
                    let prefix = if *state.selected_database_table() == Some(i) {
                        "> "
                    } else {
                        "  "
                    };

                    let table_type = if table.is_content_table {
                        Span::styled(" [content]", Style::default().fg(Color::Cyan))
                    } else {
                        Span::styled(" [system]", Style::default().fg(Color::Green))
                    };

                    let row_count = if let Some(count) = table.row_count {
                        Span::styled(
                            format!(" ({} rows)", count),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else {
                        Span::raw("")
                    };

                    let content = Line::from(vec![
                        Span::raw(prefix),
                        Span::raw(&table.name),
                        table_type,
                        row_count,
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Database Tables (t=tables, s=schema, c=content, l=load)")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::White));

            frame.render_widget(list, chunks[0]);
        }

        // Status bar
        let status = if *state.database_connected() {
            Paragraph::new("● Connected")
                .style(Style::default().fg(Color::Green))
        } else {
            Paragraph::new("○ Not connected")
                .style(Style::default().fg(Color::Red))
        };
        
        let status_block = Block::default().borders(Borders::ALL);
        frame.render_widget(status.block(status_block), chunks[1]);

        Ok(())
    }

    /// Renders the schema view.
    fn render_schema(&self, frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),    // Header
                Constraint::Min(5),       // Schema
            ])
            .split(area);

        // Header
        let selected_table = state.selected_database_table()
            .and_then(|idx| state.database_tables().get(idx));
        
        let table_name = selected_table
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "No table selected".to_string());

        let header = Paragraph::new(format!("Table: {}", table_name))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().add_modifier(Modifier::BOLD));
        
        frame.render_widget(header, chunks[0]);

        // Schema display
        let schema = state.database_schema();
        
        if schema.is_empty() {
            let empty = Paragraph::new("No schema loaded\n\nPress 't' to return to tables")
                .block(Block::default().title("Schema").borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            
            frame.render_widget(empty, chunks[1]);
        } else {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Column", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("        "),
                    Span::styled("Type", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("                "),
                    Span::styled("Nullable", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled("Default", Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from("─".repeat(60)),
            ];

            for col in schema {
                let nullable = if col.nullable { "YES" } else { "NO" };
                let default = col.default.as_deref().unwrap_or("-");

                lines.push(Line::from(format!(
                    "{:<15} {:<20} {:<8} {}",
                    col.name, col.data_type, nullable, default
                )));
            }

            let schema_text = Paragraph::new(lines)
                .block(Block::default().title("Schema (t=tables)").borders(Borders::ALL))
                .wrap(Wrap { trim: false });

            frame.render_widget(schema_text, chunks[1]);
        }

        Ok(())
    }

    /// Renders the content view.
    fn render_content(&self, frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Row list
                Constraint::Percentage(60), // Row detail
            ])
            .split(area);

        // Row list
        let content = state.database_content();
        
        if content.is_empty() {
            let empty = Paragraph::new("No content loaded\n\nPress 't' to return to tables")
                .block(Block::default().title("Content").borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            
            frame.render_widget(empty, chunks[0]);
        } else {
            let items: Vec<ListItem> = content
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let prefix = if *state.selected_database_content_row() == Some(i) {
                        "> "
                    } else {
                        "  "
                    };

                    // Try to extract ID or first field for display
                    let display = if let Some(id) = row.data.get("id") {
                        format!("Row {}: {}", i + 1, id)
                    } else {
                        format!("Row {}", i + 1)
                    };

                    let content = Line::from(vec![
                        Span::raw(prefix),
                        Span::raw(display),
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let filter = state.database_filter();
            let filter_text = if let Some(status) = &filter.review_status {
                format!("Content (filter: {}) (f=cycle filter)", status)
            } else {
                "Content (filter: all) (f=cycle filter)".to_string()
            };

            let list = List::new(items)
                .block(
                    Block::default()
                        .title(filter_text)
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::White));

            frame.render_widget(list, chunks[0]);

            // Row detail
            if let Some(row_idx) = state.selected_database_content_row() {
                if let Some(row) = content.get(*row_idx) {
                    self.render_row_detail(&row.data, frame, chunks[1])?;
                }
            } else {
                let empty = Paragraph::new("No row selected")
                    .block(Block::default().title("Row Detail").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                
                frame.render_widget(empty, chunks[1]);
            }
        }

        Ok(())
    }

    /// Renders detailed view of a single row.
    fn render_row_detail(&self, data: &JsonValue, frame: &mut Frame, area: ratatui::layout::Rect) -> TuiResult<()> {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

        let formatted = serde_json::to_string_pretty(data)
            .unwrap_or_else(|_| "Error formatting JSON".to_string());

        let lines: Vec<Line> = formatted
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect();

        let detail = Paragraph::new(lines)
            .block(Block::default().title("Row Detail").borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        frame.render_widget(detail, area);

        Ok(())
    }
}
