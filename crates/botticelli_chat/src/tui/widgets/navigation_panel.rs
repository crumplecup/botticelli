// Navigation panel widget using tui-tree-widget

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};

#[cfg(feature = "tui")]
use tui_tree_widget::{Tree, TreeItem, TreeState};

#[cfg(feature = "tui")]
use crossterm::event::KeyCode;

/// Navigation panel widget for hierarchical browsing
pub struct NavigationPanel {
    #[cfg(feature = "tui")]
    tree_state: TreeState<String>,
    #[cfg(feature = "tui")]
    tree_items: Vec<TreeItem<'static, String>>,
    title: String,
}

impl NavigationPanel {
    /// Creates a new navigation panel
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tui")]
            tree_state: TreeState::default(),
            #[cfg(feature = "tui")]
            tree_items: Vec::new(),
            title: title.into(),
        }
    }

    #[cfg(feature = "tui")]
    /// Sets the tree items to display
    pub fn set_items(&mut self, items: Vec<TreeItem<'static, String>>) {
        self.tree_items = items;
    }

    #[cfg(feature = "tui")]
    /// Renders the navigation panel
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::StatefulWidget;

        if let Ok(tree) = Tree::new(&self.tree_items) {
            let tree_widget = tree
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(self.title.as_str()),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            StatefulWidget::render(tree_widget, area, buf, &mut self.tree_state);
        }
    }

    #[cfg(feature = "tui")]
    /// Handles keyboard input for navigation
    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            // Arrow keys
            KeyCode::Down => {
                self.tree_state.key_down();
            }
            KeyCode::Up => {
                self.tree_state.key_up();
            }
            KeyCode::Right | KeyCode::Left | KeyCode::Enter | KeyCode::Char(' ') => {
                // Toggle expansion for selected item
                self.tree_state.toggle_selected();
            }

            // Vim keys
            KeyCode::Char('j') => {
                self.tree_state.key_down();
            }
            KeyCode::Char('k') => {
                self.tree_state.key_up();
            }
            KeyCode::Char('l') | KeyCode::Char('h') => {
                self.tree_state.toggle_selected();
            }

            _ => {}
        }
    }

    #[cfg(feature = "tui")]
    /// Gets the currently selected item path
    pub fn selected(&self) -> &[String] {
        self.tree_state.selected()
    }
}
