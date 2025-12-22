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
            if let Some(msgs) = state.conversation_messages(&conv_id) {
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
