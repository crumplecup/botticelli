//! Application state and core TUI types.

/// Application mode determines which view is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppMode {
    /// List view - browse content items
    List,
    /// Detail view - view single content item
    Detail,
    /// Edit view - edit tags, rating, status
    Edit,
    /// Compare view - side-by-side comparison
    Compare,
    /// Export view - export options
    Export,
}

/// Content row representation for TUI display.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentRow {
    /// Row ID
    pub id: i64,
    /// Review status (pending, approved, rejected)
    pub review_status: String,
    /// User rating (1-5)
    pub rating: Option<i32>,
    /// Tags
    pub tags: Vec<String>,
    /// Content preview (first 50 chars)
    pub preview: String,
    /// Full content (for detail view)
    pub content: serde_json::Value,
    /// Source narrative
    pub source_narrative: Option<String>,
    /// Source act
    pub source_act: Option<String>,
}

/// Edit buffer for inline editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditBuffer {
    /// Tags being edited
    pub tags: String,
    /// Rating being edited (1-5)
    pub rating: Option<i32>,
    /// Status being edited
    pub status: String,
    /// Which field is currently focused
    pub focused_field: EditField,
}

/// Edit field focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditField {
    /// Tags field
    Tags,
    /// Rating field
    Rating,
    /// Status field
    Status,
}

/// Main application state.
pub struct App {
    /// Current mode
    pub mode: AppMode,
    /// Table name being viewed
    pub table_name: String,
    /// List of content items
    pub content_items: Vec<ContentRow>,
    /// Currently selected index in list
    pub selected_index: usize,
    /// Items selected for comparison
    pub compare_selection: Vec<usize>,
    /// Edit buffer (when in Edit mode)
    pub edit_buffer: Option<EditBuffer>,
    /// Status message to display
    pub status_message: String,
    /// Whether to quit the application
    pub should_quit: bool,
}

impl App {
    /// Create a new App instance with empty state.
    pub fn new(table_name: String) -> Self {
        Self {
            mode: AppMode::List,
            table_name,
            content_items: Vec::new(),
            selected_index: 0,
            compare_selection: Vec::new(),
            edit_buffer: None,
            status_message: String::from("Press ? for help"),
            should_quit: false,
        }
    }

    /// Set content items from external source.
    pub fn set_content(&mut self, items: Vec<ContentRow>) {
        self.content_items = items;
        if self.selected_index >= self.content_items.len() && !self.content_items.is_empty() {
            self.selected_index = self.content_items.len() - 1;
        }
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if !self.content_items.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.selected_index < self.content_items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Enter detail view for selected item.
    pub fn enter_detail(&mut self) {
        if !self.content_items.is_empty() {
            self.mode = AppMode::Detail;
        }
    }

    /// Return to list view.
    pub fn return_to_list(&mut self) {
        self.mode = AppMode::List;
        self.edit_buffer = None;
        self.compare_selection.clear();
    }

    /// Enter edit mode for selected item.
    pub fn enter_edit(&mut self) {
        if let Some(item) = self.content_items.get(self.selected_index) {
            self.edit_buffer = Some(EditBuffer {
                tags: item.tags.join(", "),
                rating: item.rating,
                status: item.review_status.clone(),
                focused_field: EditField::Tags,
            });
            self.mode = AppMode::Edit;
        }
    }

    /// Get current edit buffer data for saving.
    pub fn get_edit_data(&self) -> Option<(i64, Vec<String>, Option<i32>, String)> {
        if let Some(buffer) = &self.edit_buffer {
            let item_id = self.content_items[self.selected_index].id;
            let tags: Vec<String> = buffer
                .tags
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Some((item_id, tags, buffer.rating, buffer.status.clone()))
        } else {
            None
        }
    }

    /// Toggle item in comparison selection.
    pub fn toggle_compare(&mut self) {
        if let Some(pos) = self
            .compare_selection
            .iter()
            .position(|&i| i == self.selected_index)
        {
            self.compare_selection.remove(pos);
        } else {
            self.compare_selection.push(self.selected_index);
        }

        if self.compare_selection.len() >= 2 {
            self.mode = AppMode::Compare;
        }
    }

    /// Get selected item ID for deletion.
    pub fn get_selected_id(&self) -> Option<i64> {
        self.content_items.get(self.selected_index).map(|item| item.id)
    }

    /// Quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}


