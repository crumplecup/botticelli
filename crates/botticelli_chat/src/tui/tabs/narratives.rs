// Narratives tab implementation

mod discovery;
mod tree_builder;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(feature = "tui")]
use ratatui::{buffer::Buffer, layout::Rect};

#[cfg(feature = "tui")]
use tui_tree_widget::{TreeItem, TreeState};

pub use discovery::{discover_narratives, DiscoveryError};
pub use tree_builder::{build_tree_items, find_narrative_by_path};

/// Actions that can be performed on narratives
#[derive(Debug, Clone)]
pub enum NarrativeAction {
    /// Edit narrative in external editor
    Edit(PathBuf),
    /// Execute narrative
    Execute(PathBuf),
    /// Validate TOML syntax
    Validate(PathBuf),
    /// Delete narrative file
    Delete(PathBuf),
    /// Create new narrative
    CreateNew,
    /// Refresh narrative list from disk
    Refresh,
}

/// State for the Narratives tab
#[derive(Debug)]
pub struct NarrativesTab {
    /// List of discovered narratives
    pub narratives: Vec<NarrativeEntry>,

    /// Tree widget state (from tui-tree-widget)
    #[cfg(feature = "tui")]
    pub tree_state: TreeState<String>,

    /// Tree items built from narratives
    #[cfg(feature = "tui")]
    pub tree_items: Vec<TreeItem<'static, String>>,

    /// Currently selected narrative
    pub selected: Option<usize>,

    /// Filter/search query
    pub filter: Option<String>,

    /// View mode
    pub view_mode: NarrativeViewMode,
}

/// Narrative entry with metadata
#[derive(Debug, Clone)]
pub struct NarrativeEntry {
    pub path: PathBuf,
    pub name: String,
    pub metadata: NarrativeMetadata,
    pub category: String,
    pub last_modified: SystemTime,
}

/// Narrative metadata extracted from TOML
#[derive(Debug, Clone)]
pub struct NarrativeMetadata {
    pub name: String,
    pub model: Option<String>,
    pub act_count: usize,
    pub description: Option<String>,
}

/// View modes for narratives
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NarrativeViewMode {
    /// Simple list
    List,
    /// Hierarchical tree by category
    Tree,
    /// Grid with previews
    Grid,
    /// Single narrative detail view
    Detail,
}

impl NarrativesTab {
    /// Creates a new narratives tab
    pub fn new() -> Self {
        Self {
            narratives: Vec::new(),
            #[cfg(feature = "tui")]
            tree_state: TreeState::default(),
            #[cfg(feature = "tui")]
            tree_items: Vec::new(),
            selected: None,
            filter: None,
            view_mode: NarrativeViewMode::Tree,
        }
    }

    /// Refreshes narratives from the filesystem
    pub async fn refresh_narratives<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), DiscoveryError> {
        self.narratives = discover_narratives(path).await?;

        #[cfg(feature = "tui")]
        {
            self.rebuild_tree();
        }

        Ok(())
    }

    #[cfg(feature = "tui")]
    /// Rebuilds the tree items from current narratives
    pub fn rebuild_tree(&mut self) {
        self.tree_items = build_tree_items(&self.narratives);
    }

    #[cfg(feature = "tui")]
    /// Gets the currently selected narrative
    pub fn selected_narrative(&self) -> Option<&NarrativeEntry> {
        let path_components = self.tree_state.selected();
        find_narrative_by_path(&self.narratives, path_components)
    }

    #[cfg(feature = "tui")]
    /// Handles keyboard input for quick actions
    pub fn handle_action_key(&self, key: char) -> Option<NarrativeAction> {
        let narrative = self.selected_narrative()?;

        match key {
            'e' | 'E' => Some(NarrativeAction::Edit(narrative.path.clone())),
            'x' | 'X' => Some(NarrativeAction::Execute(narrative.path.clone())),
            'v' | 'V' => Some(NarrativeAction::Validate(narrative.path.clone())),
            'd' | 'D' => Some(NarrativeAction::Delete(narrative.path.clone())),
            'n' | 'N' => Some(NarrativeAction::CreateNew),
            'r' | 'R' => Some(NarrativeAction::Refresh),
            _ => None,
        }
    }

    #[cfg(feature = "tui")]
    /// Handles tree navigation keys
    pub fn handle_nav_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                self.tree_state.key_down();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.tree_state.key_up();
            }
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Right | KeyCode::Char('l') => {
                self.tree_state.toggle_selected();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // For now, just toggle (we could implement close-only logic)
                self.tree_state.toggle_selected();
            }
            _ => {}
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the narratives tab
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use crate::tui::widgets::NavigationPanel;
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        // Create layout: navigation panel on left, content on right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Navigation panel
                Constraint::Percentage(70), // Content area
            ])
            .split(area);

        // Render navigation panel with tree
        let mut nav_panel = NavigationPanel::new("Narratives");
        nav_panel.set_items(self.tree_items.clone());
        nav_panel.render(chunks[0], buf);

        // Update our tree state from the panel
        // (In a real implementation, we'd need to synchronize state)

        // Render content area based on selection
        if let Some(narrative) = self.selected_narrative() {
            self.render_narrative_detail(narrative, chunks[1], buf);
        } else {
            self.render_empty_state(chunks[1], buf);
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the detail view for a selected narrative
    fn render_narrative_detail(&self, narrative: &NarrativeEntry, area: Rect, buf: &mut Buffer) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let content = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&narrative.metadata.name),
            ]),
            Line::from(vec![
                Span::styled("Model: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(narrative.metadata.model.as_deref().unwrap_or("default")),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(narrative.path.to_string_lossy()),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(
                narrative
                    .metadata
                    .description
                    .as_deref()
                    .unwrap_or("No description available"),
            ),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from("[E] Edit  [X] Execute  [V] Validate  [D] Delete"),
        ];

        use ratatui::widgets::Widget;

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Narrative Details"),
            )
            .wrap(Wrap { trim: true });

        Widget::render(paragraph, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Renders empty state when no narrative is selected
    fn render_empty_state(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::Line,
            widgets::{Block, Borders, Paragraph, Widget},
        };

        let content = vec![
            Line::from(""),
            Line::from("No narrative selected"),
            Line::from(""),
            Line::from("Use arrow keys or vim keys (j/k) to navigate"),
            Line::from("Press Enter to expand/collapse categories"),
        ];

        let paragraph = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .style(Style::default().fg(Color::DarkGray));

        Widget::render(paragraph, area, buf);
    }
}

impl Default for NarrativesTab {
    fn default() -> Self {
        Self::new()
    }
}
