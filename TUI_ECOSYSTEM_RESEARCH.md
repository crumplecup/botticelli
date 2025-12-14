# Ratatui Ecosystem Research: Strategic Assessment

**Date:** 2024-12-14
**Purpose:** Evaluate available ratatui libraries/frameworks to avoid reinventing wheels and leverage battle-tested solutions

---

## Executive Summary

The ratatui ecosystem has matured significantly in 2024, with:
- ✅ **20+ high-quality widget libraries** ready to use
- ✅ **Proven best practices** from community apps
- ✅ **Several MVC/application frameworks** available
- ✅ **Notable applications** we can learn from (Oatmeal, Codex)

**Key Finding:** We can build on existing components rather than creating everything from scratch, saving 2-3 weeks of development time.

---

## Available Widget Libraries

### Essential Widgets We Should Use

#### 1. **tui-tree-widget** ⭐ MUST USE
- **Purpose:** Tree data structure visualization
- **Perfect for:** Navigation panels showing hierarchical data
- **Use in Botticelli:**
  - Narrative directory tree
  - Database table/schema browser
  - Bot hierarchy
- **Link:** [crates.io/crates/tui-tree-widget](https://crates.io/crates/tui-tree-widget)
- **Status:** Mature, actively maintained

**Code Example:**
```rust
use tui_tree_widget::{Tree, TreeItem, TreeState};

// For narrative browser
let items = vec![
    TreeItem::new_leaf("showcase.toml"),
    TreeItem::new(
        "discord",
        vec![
            TreeItem::new_leaf("daily_showcase.toml"),
            TreeItem::new_leaf("welcome.toml"),
        ]
    ),
];

let tree = Tree::new(&items)?
    .block(Block::default().borders(Borders::ALL).title("Narratives"));

f.render_stateful_widget(tree, area, &mut tree_state);
```

#### 2. **tui-textarea** ⭐ MUST USE
- **Purpose:** Multi-line text editor widget (like HTML `<textarea>`)
- **Features:**
  - Undo/redo (Ctrl+Z, Ctrl+Y)
  - Search (Ctrl+F)
  - Copy/paste support
  - Vim-like modal editing (optional)
  - Line numbers
- **Perfect for:**
  - Chat input (multi-line messages)
  - Query editor (SQL/custom queries)
  - Quick file editing
  - Prompt templates
- **Link:** [github.com/rhysd/tui-textarea](https://github.com/rhysd/tui-textarea)
- **Status:** Very mature, 2024 updates for ratatui 0.29

**Code Example:**
```rust
use tui_textarea::TextArea;

let mut textarea = TextArea::default();
textarea.set_block(
    Block::default()
        .borders(Borders::ALL)
        .title("Chat Input")
);

// Handles all key events internally
match event.code {
    KeyCode::Char(c) => textarea.input(event),
    KeyCode::Enter => {
        let lines = textarea.lines();
        // Send message...
        textarea.delete_line_by_head();
    }
    _ => textarea.input(event),
}

f.render_widget(textarea.widget(), area);
```

#### 3. **ratatui-explorer** ⭐ HIGHLY RECOMMENDED
- **Purpose:** File explorer widget
- **Features:**
  - Browse directories
  - File selection
  - Configurable file filtering
  - Navigation with arrow keys
- **Perfect for:**
  - Narrative file browser
  - Settings file selection
  - Import/export file picker
- **Link:** [Third Party Widgets Showcase](https://ratatui.rs/showcase/third-party-widgets/)
- **Status:** New, promising

#### 4. **tui-popup** ⭐ RECOMMENDED
- **Purpose:** Modal/popup dialogs
- **Perfect for:**
  - Confirmation dialogs
  - Error messages
  - Help overlay
  - Create narrative wizard
- **Link:** Via awesome-ratatui
- **Alternative:** Can build simple popups ourselves (not complex)

#### 5. **tui-prompts**
- **Purpose:** Interactive command-line style prompts
- **Features:**
  - Text input with validation
  - Select from list
  - Confirm (yes/no)
  - Multi-select checkboxes
- **Perfect for:**
  - Narrative creation wizard
  - Settings configuration
  - Bot setup
- **Link:** [awesome-ratatui](https://github.com/ratatui/awesome-ratatui)

#### 6. **tui-logger** ⭐ RECOMMENDED
- **Purpose:** Logger widget with filtering
- **Perfect for:**
  - Debug/monitoring tab
  - Bot execution logs
  - Narrative execution traces
- **Integration:** Works with `log` crate
- **Link:** Via awesome-ratatui

### Specialized Widgets (Consider Later)

#### 7. **ratatui-image**
- **Purpose:** Display images (sixel, unicode halfblocks)
- **Use case:** Preview images from narratives, media content
- **Priority:** Low (nice-to-have)

#### 8. **ratatui-code-editor**
- **Purpose:** Full code editor with syntax highlighting (tree-sitter)
- **Use case:** Edit narrative TOML files inline
- **Priority:** Medium (can use $EDITOR for MVP)

#### 9. **tui-big-text**
- **Purpose:** Large ASCII art text
- **Use case:** Splash screen, branding
- **Priority:** Low (polish feature)

---

## Application Frameworks

### 1. **tui-realm**
- **Description:** React/Elm-inspired framework for ratatui
- **Philosophy:** Component-based, message-passing architecture
- **Pros:**
  - Well-structured patterns
  - Component reusability
  - Clear separation of concerns
- **Cons:**
  - Learning curve
  - More abstraction layers
  - Possibly overkill for our needs
- **Link:** [awesome-ratatui](https://github.com/ratatui/awesome-ratatui)
- **Recommendation:** ⚠️ **Skip** - Direct ratatui is simpler for our use case

### 2. **Direct Ratatui with MVC Pattern** ⭐ RECOMMENDED
- **Description:** Community best practice from GitHub discussions
- **Pattern:**
  ```rust
  struct App {
      state: AppState,      // Model
      // Views are render methods
      // Controller is event handler
  }
  ```
- **Pros:**
  - No extra dependencies
  - Maximum flexibility
  - Well-understood pattern
  - Direct control
- **Cons:**
  - Must structure ourselves
  - No framework magic
- **Link:** [Best practices discussion](https://github.com/ratatui/ratatui/discussions/220)
- **Recommendation:** ✅ **Use this** - Matches our current approach

---

## Notable Applications to Study

### 1. **Oatmeal** ⭐ MUST STUDY
- **Description:** Terminal UI chat application with LLM support
- **Relevance:** 🎯 **HIGHLY RELEVANT** - Almost exactly what we're building!
- **Features:**
  - Chat interface with LLM integration
  - Fancy chat bubbles
  - Terminal native
- **Link:** [App Showcase](https://ratatui.rs/showcase/apps/)
- **Learnings:**
  - How to structure LLM chat in TUI
  - Message rendering patterns
  - Real-time updates
- **Action:** ⚠️ **REVIEW THEIR SOURCE CODE**

### 2. **Codex**
- **Description:** Terminal-native coding agent
- **Relevance:** Similar AI/LLM integration patterns
- **Link:** [App Showcase](https://ratatui.rs/showcase/apps/)

### 3. **Yōzefu**
- **Description:** Interactive TUI for Kafka cluster data
- **Relevance:** Multi-tab data browser (similar to our database tab)
- **Learnings:**
  - Table rendering patterns
  - Data pagination
  - Navigation between related data

### 4. **Examples in ratatui repo**
- **JSON Editor Tutorial** - Sophisticated layouts
- **Counter App** - Immediate mode rendering
- **Table Example** - Sorting, filtering, selection
- **Link:** [examples/README.md](https://github.com/ratatui/ratatui/blob/main/examples/README.md)

---

## Best Practices from Community

### 1. Logging Strategy
**Recommendation:** Use `log4rs` for file-based logging

```rust
// Users can share logs when debugging
log4rs::init_file("config/log4rs.yaml", Default::default())?;

info!("App started");
debug!("Current state: {:?}", app.state);
error!("Failed to execute narrative: {}", e);

// Logs go to file, don't interfere with TUI
```

**Our implementation:**
- Already using `tracing` crate ✅
- Can output to file for debugging
- Keep current approach, maybe add `tui-logger` widget for visibility

### 2. Configuration Management
**Recommendation:** Use XDG directories

```rust
use directories::ProjectDirs;

if let Some(proj_dirs) = ProjectDirs::from("com", "botticelli", "Botticelli") {
    let config_dir = proj_dirs.config_dir();
    // ~/.config/botticelli/ on Linux
    // ~/Library/Application Support/botticelli/ on macOS
}
```

**Our implementation:**
- Currently using TOML files ✅
- Should move to XDG directories for user config
- Keep workspace config in repo

### 3. Event-Driven Architecture
**Recommendation:** Command/Message pattern

```rust
enum Message {
    Quit,
    SwitchTab(Tab),
    ExecuteNarrative(PathBuf),
    // etc.
}

impl App {
    fn update(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::ExecuteNarrative(path) => {
                self.state.executing = true;
                // spawn task...
            }
            // ...
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('x') if self.selected_narrative.is_some() => {
                Some(Message::ExecuteNarrative(self.selected_narrative.clone().unwrap()))
            }
            _ => None
        }
    }
}
```

**Our implementation:**
- Planned `TuiCommand` enum already matches this ✅
- Should formalize message passing

### 4. Error Handling
**Recommendation:** Use `color_eyre` for better error displays

```rust
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    // If any error occurs, get beautiful traceback even after TUI exits
}
```

**Our implementation:**
- Using `botticelli_error` with derive_more ✅
- Can add `color_eyre` for development/debugging
- Keep current error types for production

---

## Strategic Recommendations

### Immediate Actions (Week 1-2)

#### 1. ✅ **Add Dependencies**
```toml
[dependencies]
tui-tree-widget = "0.22"
tui-textarea = "0.6"
ratatui-explorer = "0.1"  # If stable
tui-logger = "0.12"

[dev-dependencies]
color_eyre = "0.6"
```

#### 2. ✅ **Study Oatmeal Source Code**
- Find their repo (likely on GitHub)
- Study chat rendering
- Study LLM integration patterns
- Adapt patterns to our needs

#### 3. ✅ **Use Tree Widget for Navigation**
- Replace our planned custom tree with `tui-tree-widget`
- Saves 2-3 days of development
- Battle-tested, feature-complete

#### 4. ✅ **Use TextArea for Inputs**
- Multi-line chat input
- Query editor
- Better than rolling our own

### Architecture Decisions

#### 1. ✅ **Stick with Direct Ratatui + MVC**
- Don't use tui-realm framework
- Follow community MVC pattern
- Maximum control, minimum abstraction

#### 2. ✅ **Adopt Event-Driven Message Pattern**
- Formalize our `TuiCommand` as messages
- Clear update cycle
- Testable state transitions

#### 3. ✅ **Use XDG Directories**
- Move user config to standard locations
- Better cross-platform support
- Matches user expectations

### What NOT to Use (Avoid Bloat)

#### ❌ **Skip These:**
- `tui-realm` - Too much abstraction
- `ratatui-image` - Not critical for MVP
- `tui-big-text` - Polish feature only
- `ratatui-code-editor` - Can use $EDITOR

---

## Updated Implementation Plan

### Week 1: Foundation + Widget Integration

**Original Plan:** Build custom widgets
**Updated Plan:** Integrate existing widgets

```rust
// Before (custom implementation)
struct TreeWidget {
    items: Vec<TreeNode>,
    selected: usize,
    // 200+ lines of tree logic
}

// After (use library)
use tui_tree_widget::{Tree, TreeState};

struct NavigationPanel {
    tree_state: TreeState<String>,
    items: Vec<TreeItem<String>>,
}
// 30 lines of integration code
```

**Time Saved:** 2-3 days per major widget

### Widget Mapping

| Our Need | Library | Status |
|----------|---------|--------|
| Navigation tree | `tui-tree-widget` | ✅ Use |
| Multi-line input | `tui-textarea` | ✅ Use |
| File browser | `ratatui-explorer` | ✅ Use if stable |
| Popups/modals | `tui-popup` or custom | ⚠️ Evaluate |
| Logger view | `tui-logger` | ✅ Use |
| Prompts | `tui-prompts` | ⚠️ Evaluate |

### Revised Timeline

**Original:** 5 weeks
**With Libraries:** 3-4 weeks

- **Week 1:** Foundation + integrate tree/textarea widgets
- **Week 2:** Narratives tab (using tree widget)
- **Week 3:** Other tabs (reusing patterns)
- **Week 4:** Polish + features

**Savings:** 1-2 weeks by using proven libraries

---

## Code Examples: Before & After

### Navigation Tree

**Before (custom):**
```rust
// 200+ lines of custom tree implementation
struct CustomTree {
    nodes: Vec<TreeNode>,
    expanded: HashSet<usize>,
    selected: usize,
    // ... lots of logic
}

impl CustomTree {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Complex rendering logic
        // Handle expansion
        // Handle selection
        // Handle scrolling
    }

    fn handle_key(&mut self, key: KeyCode) {
        // Navigation logic
    }
}
```

**After (using library):**
```rust
use tui_tree_widget::{Tree, TreeItem, TreeState};

struct NarrativeTree {
    items: Vec<TreeItem<'static, String>>,
    state: TreeState<String>,
}

impl NarrativeTree {
    fn new(narratives: Vec<NarrativeEntry>) -> Self {
        let items = build_tree_items(narratives);
        Self {
            items,
            state: TreeState::default(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let tree = Tree::new(&self.items)
            .block(Block::default().borders(Borders::ALL).title("Narratives"))
            .highlight_style(Style::default().fg(Color::Yellow));

        tree.render(area, buf, &mut self.state);
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Down => self.state.key_down(),
            KeyCode::Up => self.state.key_up(),
            KeyCode::Right => self.state.toggle_selected(),
            _ => {}
        }
    }
}

fn build_tree_items(narratives: Vec<NarrativeEntry>) -> Vec<TreeItem<'static, String>> {
    // Group by category
    let mut categories: HashMap<String, Vec<NarrativeEntry>> = HashMap::new();
    for n in narratives {
        categories.entry(n.category.clone()).or_default().push(n);
    }

    categories.into_iter()
        .map(|(cat, items)| {
            let children = items.into_iter()
                .map(|n| TreeItem::new_leaf(n.name.clone()))
                .collect();
            TreeItem::new(cat, children).unwrap()
        })
        .collect()
}
```

**Lines of code:** 200+ → 30
**Features gained:** Keyboard nav, expansion, scrolling (all built-in)

### Chat Input

**Before (basic input):**
```rust
// Current simple input
struct InputBuffer {
    text: String,
    cursor: usize,
}

// Single line only, no editing features
```

**After (rich textarea):**
```rust
use tui_textarea::TextArea;

let mut textarea = TextArea::default();
textarea.set_block(Block::default().title("Chat"));
textarea.set_placeholder_text("Type your message...");

// Built-in features:
// - Multi-line editing
// - Undo/redo (Ctrl+Z/Y)
// - Copy/paste
// - Search
// - Line numbers (optional)
```

**Functionality gained:**
- Multi-line messages
- Undo/redo
- Professional editing experience

---

## Learning Resources

### Official Docs
- [Ratatui Documentation](https://docs.rs/ratatui/latest/ratatui/)
- [Ratatui.rs](https://ratatui.rs/)
- [Best Practices Discussion](https://github.com/ratatui/ratatui/discussions/220)

### Tutorials (2024)
- FOSDEM 2024 talk - Introduction and hello world
- EuroRust 2024 talk - Common concepts and capabilities
- [Ratatui Hello World Tutorial](https://ratatui.rs/)
- JSON Editor tutorial - Sophisticated layouts

### Community
- [awesome-ratatui](https://github.com/ratatui/awesome-ratatui) - Curated list of apps/libraries
- [Third Party Widgets Showcase](https://ratatui.rs/showcase/third-party-widgets/)
- [App Showcase](https://ratatui.rs/showcase/apps/)

---

## Action Items

### Immediate (This Week)
1. ✅ Add widget dependencies to Cargo.toml
2. ✅ Find and study Oatmeal source code
3. ✅ Create spike/prototype with tui-tree-widget
4. ✅ Test tui-textarea integration

### Short Term (Next 2 Weeks)
1. ✅ Integrate tree widget into narratives tab
2. ✅ Replace input buffer with textarea
3. ✅ Add tui-logger for debug tab
4. ✅ Study example apps for patterns

### Medium Term
1. Consider ratatui-explorer for file operations
2. Evaluate tui-prompts for wizards
3. Add polish widgets (image, big-text) if valuable

---

## Summary: What We're Gaining

### Time Savings
- **Tree widget:** 2-3 days saved
- **Textarea widget:** 1-2 days saved
- **Logger widget:** 1 day saved
- **Best practices:** Avoid pitfalls, faster development
- **Total saved:** ~1-2 weeks

### Quality Improvements
- ✅ Battle-tested components
- ✅ Keyboard navigation standards
- ✅ Accessibility features built-in
- ✅ Community-validated patterns
- ✅ Reduced bugs from custom code

### What We Keep
- ✅ Our domain logic (narratives, bots, database)
- ✅ Our command system
- ✅ Our service container
- ✅ Our error handling
- ✅ Our business logic

**Philosophy:** Use libraries for UI mechanics, own our unique features.

---

## Sources

- [Ratatui GitHub](https://github.com/ratatui/ratatui)
- [awesome-ratatui](https://github.com/ratatui/awesome-ratatui)
- [Third Party Widgets](https://ratatui.rs/showcase/third-party-widgets/)
- [tui-textarea](https://github.com/rhysd/tui-textarea)
- [Best Practices Discussion](https://github.com/ratatui/ratatui/discussions/220)
- [App Showcase](https://ratatui.rs/showcase/apps/)
- [Ratatui References](https://ratatui.rs/references/)
