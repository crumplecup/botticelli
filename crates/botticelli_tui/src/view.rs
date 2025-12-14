use ratatui::Frame;

use crate::{AppState, Command, TuiResult};

/// Trait for TUI views.
pub trait View {
    /// Renders the view to the terminal frame.
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()>;

    /// Handles keyboard input and returns a command if applicable.
    fn handle_input(&self, key: crossterm::event::KeyEvent, state: &AppState)
        -> TuiResult<Option<Command>>;
}

/// Chat view implementation.
#[derive(Debug, Default)]
pub struct ChatView;

impl View for ChatView {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(frame.area());

        // Message area
        let messages = if let Some(conv_id) = state.current_conversation() {
            if let Some(msgs) = state.conversation_messages(&conv_id) {
                msgs.iter()
                    .map(|m| {
                        if m.is_user {
                            format!("You: {}", m.content)
                        } else {
                            format!("Bot: {}", m.content)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::from("No messages")
            }
        } else {
            String::from("No conversation selected")
        };

        let messages_widget = Paragraph::new(messages)
            .block(Block::default().title("Chat").borders(Borders::ALL));
        frame.render_widget(messages_widget, chunks[0]);

        // Input area
        let input_widget = Paragraph::new(state.input_buffer())
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
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Some(Command::Quit)),
            _ => Ok(None),
        }
    }
}

/// Narrative browser view stub.
#[derive(Debug, Default)]
pub struct NarrativeBrowserView;

impl View for NarrativeBrowserView {
    fn render(&self, _frame: &mut Frame, _state: &AppState) -> TuiResult<()> {
        // TODO: Implement in Phase 3
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
            _ => Ok(None),
        }
    }
}

/// Narrative editor view stub.
#[derive(Debug, Default)]
pub struct NarrativeEditorView;

impl View for NarrativeEditorView {
    fn render(&self, _frame: &mut Frame, _state: &AppState) -> TuiResult<()> {
        // TODO: Implement in Phase 3
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
            _ => Ok(None),
        }
    }
}
