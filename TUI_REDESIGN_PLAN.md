# Botticelli TUI Redesign: Comprehensive Planning Document

**Status:** Phase 4 - Task 4.2 Complete (2024-12-14)

## Progress Summary

### Completed
- ✅ Phase 1: Foundation (Dependencies, Core Types)
- ✅ Phase 2: State Management (AppState with conversation and narrative support)
- ✅ Phase 3: UI Components (ChatView, NarrativeBrowserView, NarrativeEditorView)
- ✅ Phase 4.1: Event Loop (EventHandler, keyboard input)
- ✅ Phase 4.2: View Rendering (All three views have functional rendering)

### Current State
- New `botticelli_tui` crate with clean architecture
- Old TUI code removed from `botticelli_chat`
- Binary updated to use new TUI
- All views render with proper layouts
- Navigation commands implemented (up/down, select)
- Mode switching between Chat/Browser/Editor

### Next Steps
- Phase 5: Integration (Connect to actual data sources)
- Phase 6: Polish (Help system, error messages, keyboard shortcuts)
- Phase 7: Testing (Unit and integration tests)

## Executive Summary

**Problem:** Current TUI is a simple chat interface that hides most of Botticelli's capabilities. Users can't discover or access:
- Database content and queries
- Narrative library (50+ narrative files)
- Bot management
- Social media scheduling
- MCP tools (29 available)
- Settings and configuration
- File/project management

**Solution:** Multi-pane, discoverable TUI following familiar patterns while exposing Botticelli's unique features.

**Philosophy:** "Don't reinvent the wheel for common patterns; showcase what makes Botticelli special."

**Ecosystem Strategy:** Leverage proven ratatui libraries for UI mechanics, own our unique Botticelli features.

**See also:** [TUI_ECOSYSTEM_RESEARCH.md](./TUI_ECOSYSTEM_RESEARCH.md) for detailed ecosystem analysis

---

## Current State Analysis

### What We Have ✅

**TUI Implementation:**
- Single file: `crates/botticelli_chat/src/tui.rs` (277 lines)
- Simple chat interface (messages + input)
- Basic keyboard shortcuts (Ctrl+C, Ctrl+L, Up/Down)
- No menus, no panels, no discoverability

**Commands Available:**
```rust
enum Command {
    Narrative(NarrativeCommand),  // 11 subcommands
    Bot(BotCommand),              // 4 subcommands
    Social(SocialCommand),        // 3 subcommands
    Help,
    Exit,
}
```

**Services Available:**
- Database pool (PostgreSQL)
- MCP client
- Narrative repository
- LLM provider (Gemini/Claude)
- Content generation repo

**MCP Tools Available:** 29 tools including:
- Database queries
- Narrative creation/modification/execution
- Discord integration
- Bot commands
- Metrics/monitoring

**Narrative Library:** 50+ TOML files in `/narratives` directory

### What's Missing ❌

1. **No discoverability** - Users must know exact commands
2. **No visual feedback** - Can't see what's available
3. **No navigation** - Can't browse narratives, bots, database tables
4. **No settings UI** - Config is hidden in TOML files
5. **No file management** - Can't browse, open, save files visually
6. **No status display** - Can't see active bots, schedules, running processes
7. **No help system** - Limited guidance on features

---

## Design Principles

### 1. Familiarity First

**Use patterns from successful TUIs:**
- **htop** - Process viewer with panels and shortcuts
- **vim** - Mode-based interaction, status line
- **tmux** - Panes, status bar, keybindings
- **lazygit** - Panel-based navigation, clear actions
- **k9s** - Multi-pane K8s interface with context awareness

**Key patterns to adopt:**
- Status bar showing context
- Side panel for navigation
- Main content area
- Command palette/help overlay
- Consistent keyboard shortcuts

### 2. Progressive Disclosure

**Information hierarchy:**
1. **Always visible:** Status, current context, shortcuts
2. **One key away:** Main features (narratives, bots, db, chat)
3. **Two keys away:** Settings, help, advanced features

### 3. Context Awareness

**Show what matters:**
- Active narrative → show acts, execution state
- Browsing narratives → show metadata, quick actions
- Database view → show tables, recent queries
- Bot view → show assignments, schedules

---

## Proposed Architecture

### Layout System

```
┌──────────────────────────────────────────────────────────────┐
│ Botticelli │ [Tab] Narratives │ Bots │ Database │ Chat      │  ← Tab Bar
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────┐  ┌──────────────────────────────────────┐    │
│  │          │  │                                       │    │
│  │ Navigation│  │      Main Content Area               │    │  ← Main Area
│  │  Panel   │  │                                       │    │
│  │          │  │                                       │    │
│  │          │  │                                       │    │
│  │          │  │                                       │    │
│  │          │  │                                       │    │
│  │          │  │                                       │    │
│  └──────────┘  └──────────────────────────────────────┘    │
│                                                               │
├──────────────────────────────────────────────────────────────┤
│ Status: Ready │ Provider: Claude │ Narrative: showcase.toml  │  ← Status Bar
├──────────────────────────────────────────────────────────────┤
│ ? Help │ Ctrl+C Quit │ / Search │ : Command                 │  ← Shortcut Bar
└──────────────────────────────────────────────────────────────┘
```

### Tab Structure

#### Tab 1: Narratives 📖

**Purpose:** Browse, create, edit, execute narratives

**Navigation Panel:**
```
Narratives
├── Recent
│   ├── showcase.toml
│   └── discord/daily_showcase.toml
├── Categories
│   ├── Discord (40 files)
│   ├── Examples (10 files)
│   └── Tests (5 files)
└── Actions
    ├── Create New (N)
    ├── Import (I)
    └── Settings
```

**Main Content (when narrative selected):**
```
┌─ showcase.toml ────────────────────────────┐
│ Name: Showcase Narrative                   │
│ Model: claude-3-5-sonnet-20241022         │
│ Acts: 3                                    │
│                                            │
│ Table of Contents:                         │
│ 1. ✓ introduction                          │
│ 2. ✓ showcase_features                     │
│ 3. ⏸ conclusion                            │
│                                            │
│ [E] Edit  [X] Execute  [V] Validate        │
│ [D] Duplicate  [Del] Delete                │
└────────────────────────────────────────────┘
```

**Actions:**
- `Enter` - View/execute selected narrative
- `N` - Create new narrative (launches wizard)
- `E` - Edit in editor
- `V` - Validate TOML
- `X` - Execute narrative
- `/` - Search narratives

#### Tab 2: Bots 🤖

**Purpose:** Manage bots, assign narratives, view status

**Navigation Panel:**
```
Bots
├── Active (2)
│   ├── showcase_bot [Running]
│   └── welcome_bot [Idle]
├── Inactive (1)
│   └── test_bot
└── Actions
    ├── Create Bot (N)
    ├── Import Config (I)
    └── View Logs
```

**Main Content (when bot selected):**
```
┌─ showcase_bot ─────────────────────────────┐
│ Status: ● Running                          │
│ Narrative: showcase.toml                   │
│ Platform: Discord                          │
│ Last execution: 2024-12-14 08:30:15       │
│                                            │
│ Execution History (5 most recent):        │
│ ✓ 08:30:15 - Completed (3 acts)           │
│ ✓ 08:00:12 - Completed (3 acts)           │
│ ⚠ 07:30:08 - Failed (act 2)               │
│ ✓ 07:00:05 - Completed (3 acts)           │
│ ✓ 06:30:02 - Completed (3 acts)           │
│                                            │
│ [S] Start  [P] Pause  [L] Logs             │
│ [A] Assign Narrative  [E] Edit Config      │
└────────────────────────────────────────────┘
```

**Actions:**
- `Enter` - View bot details
- `N` - Create new bot
- `A` - Assign narrative to bot
- `S`/`P` - Start/pause bot
- `L` - View logs

#### Tab 3: Database 💾

**Purpose:** Browse tables, run queries, view content

**Navigation Panel:**
```
Database
├── Tables
│   ├── content (1,234 rows)
│   ├── narrative_executions (456)
│   ├── act_executions (1,890)
│   ├── model_responses (5,432)
│   └── content_generations (789)
├── Saved Queries
│   ├── Recent content
│   ├── Failed executions
│   └── Top performers
└── Actions
    ├── New Query (Q)
    ├── Export Data (E)
    └── Schema Browser
```

**Main Content (table view):**
```
┌─ content table ────────────────────────────┐
│ Showing 1-20 of 1,234 rows                 │
│                                            │
│ ID  │ Created    │ Type    │ Platform     │
│ ────┼────────────┼─────────┼──────────────│
│ 123 │ 2024-12-14 │ post    │ discord      │
│ 124 │ 2024-12-14 │ comment │ discord      │
│ 125 │ 2024-12-13 │ post    │ discord      │
│ ... │ ...        │ ...     │ ...          │
│                                            │
│ [Q] Query  [F] Filter  [→] Details         │
│ [E] Export  [R] Refresh                    │
└────────────────────────────────────────────┘
```

**Actions:**
- `Enter` - View row details
- `Q` - Open query editor
- `F` - Add filter
- `E` - Export results
- `/` - Search table

#### Tab 4: Chat 💬

**Purpose:** Conversational interface, LLM interaction

**Navigation Panel:**
```
Chat
├── Conversations
│   ├── Current Session
│   └── History (10 recent)
├── Prompts
│   ├── System prompts
│   └── Saved prompts
└── Settings
    ├── Model: claude-3-5-sonnet
    ├── Temperature: 0.7
    └── Max tokens: 4096
```

**Main Content:**
```
┌─ Chat ─────────────────────────────────────┐
│ System: Welcome! I can help you...        │
│                                            │
│ You: Create a narrative about space       │
│                                            │
│ Assistant: I'll help create a space       │
│ narrative. Let me guide you through...    │
│                                            │
│ [Shows thinking process, tool calls]      │
│                                            │
│ ─────────────────────────────────────────│
│ > _                                        │
│                                            │
│ [↑↓] History  [Ctrl+L] Clear               │
└────────────────────────────────────────────┘
```

#### Tab 5: Schedule 📅

**Purpose:** View and manage scheduled posts

**Navigation Panel:**
```
Schedule
├── Upcoming (5)
│   ├── Today (2)
│   ├── This Week (3)
│   └── This Month (12)
├── Past
│   └── Last 30 days
└── Actions
    ├── New Schedule (N)
    └── Bulk Import
```

**Main Content:**
```
┌─ Scheduled Posts ──────────────────────────┐
│ Today - Dec 14, 2024                       │
│                                            │
│ 10:00 AM - showcase_bot                   │
│   Platform: Discord                        │
│   Narrative: daily_showcase.toml           │
│   [E] Edit  [C] Cancel                     │
│                                            │
│ 2:00 PM - welcome_bot                      │
│   Platform: Discord                        │
│   Narrative: simple_welcome.toml           │
│   [E] Edit  [C] Cancel                     │
│                                            │
│ [N] New  [F] Filter by Bot/Platform        │
└────────────────────────────────────────────┘
```

#### Tab 6: Settings ⚙️

**Purpose:** Configuration, preferences, API keys

**Structure:**
```
┌─ Settings ─────────────────────────────────┐
│ General                                    │
│   UI Theme: [Dark ▼]                       │
│   Editor: [vim ▼]                          │
│                                            │
│ LLM Providers                              │
│   Default: [Claude ▼]                      │
│   ● Anthropic API Key: ••••••••           │
│   ○ Google API Key: [Not Set]             │
│   ○ OpenAI API Key: [Not Set]             │
│                                            │
│ Database                                   │
│   Host: localhost                          │
│   Port: 5432                               │
│   Database: botticelli                     │
│   ● Connected                              │
│                                            │
│ MCP                                        │
│   Server: localhost:8080                   │
│   ○ Not Connected                          │
│                                            │
│ [S] Save  [R] Reset  [T] Test Connection  │
└────────────────────────────────────────────┘
```

---

## Dependencies & Libraries

### Core Dependencies

```toml
[dependencies]
# Base TUI framework
ratatui = "0.29"
crossterm = "0.28"

# Widget Libraries (ecosystem)
tui-tree-widget = "0.22"      # Tree navigation for files/hierarchies
tui-textarea = "0.6"          # Rich multi-line text editor
tui-logger = "0.12"           # Real-time log viewing widget

# Optional (evaluate during implementation)
ratatui-explorer = "0.1"      # File browser widget
tui-popup = "0.1"             # Modal dialogs

# Utilities
directories = "5.0"           # XDG directory support

[dev-dependencies]
color_eyre = "0.6"            # Beautiful error displays
```

### Library Usage Strategy

**tui-tree-widget:**
- Navigation panels for narratives, bots, database tables
- Replaces custom tree implementation (saves 200+ lines)
- Built-in expansion, selection, keyboard nav

**tui-textarea:**
- Multi-line chat input with undo/redo
- Query editor for database tab
- Rich editing experience (search, copy/paste)

**tui-logger:**
- Debug/monitoring tab
- Bot execution logs
- Real-time log filtering

---

## Component Architecture

### 1. App State Management

```rust
// crates/botticelli_chat/src/tui/state.rs

pub struct AppState {
    /// Currently active tab
    pub active_tab: Tab,

    /// Tab-specific state
    pub narratives_state: NarrativesTabState,
    pub bots_state: BotsTabState,
    pub database_state: DatabaseTabState,
    pub chat_state: ChatTabState,
    pub schedule_state: ScheduleTabState,
    pub settings_state: SettingsTabState,

    /// Global state
    pub status: StatusInfo,
    pub modal: Option<Modal>,
    pub services: Arc<ServiceContainer>,
}

pub enum Tab {
    Narratives,
    Bots,
    Database,
    Chat,
    Schedule,
    Settings,
}

pub struct StatusInfo {
    pub current_narrative: Option<String>,
    pub active_provider: String,
    pub db_connected: bool,
    pub mcp_connected: bool,
}

pub enum Modal {
    Help,
    CreateNarrative(NarrativeWizard),
    Confirm { message: String, action: Box<dyn Fn()> },
    Error { message: String },
    Search { query: String, results: Vec<SearchResult> },
}
```

### 2. Tab State

```rust
// crates/botticelli_chat/src/tui/tabs/narratives.rs

use tui_tree_widget::{TreeItem, TreeState};

pub struct NarrativesTabState {
    /// List of discovered narratives
    pub narratives: Vec<NarrativeEntry>,

    /// Tree widget state (from tui-tree-widget)
    pub tree_state: TreeState<String>,

    /// Tree items built from narratives
    pub tree_items: Vec<TreeItem<'static, String>>,

    /// Filter/search
    pub filter: Option<String>,

    /// View mode
    pub view_mode: NarrativeViewMode,
}

impl NarrativesTabState {
    pub fn build_tree(&mut self) {
        // Group narratives by category
        let mut categories: HashMap<String, Vec<NarrativeEntry>> = HashMap::new();
        for narrative in &self.narratives {
            categories.entry(narrative.category.clone())
                .or_default()
                .push(narrative.clone());
        }

        // Build tree items
        self.tree_items = categories.into_iter()
            .map(|(category, items)| {
                let children: Vec<_> = items.into_iter()
                    .map(|n| TreeItem::new_leaf(n.name.clone()))
                    .collect();
                TreeItem::new(category, children).unwrap()
            })
            .collect();
    }

    pub fn render_tree(&mut self, area: Rect, buf: &mut Buffer) {
        use tui_tree_widget::Tree;

        let tree = Tree::new(&self.tree_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Narratives"))
            .highlight_style(Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD));

        tree.render(area, buf, &mut self.tree_state);
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Down => self.tree_state.key_down(),
            KeyCode::Up => self.tree_state.key_up(),
            KeyCode::Right => self.tree_state.toggle_selected(),
            KeyCode::Left => self.tree_state.close(),
            _ => {}
        }
    }
}

pub struct NarrativeEntry {
    pub path: PathBuf,
    pub name: String,
    pub metadata: NarrativeMetadata,
    pub last_modified: SystemTime,
    pub category: String,
}

pub struct NarrativeMetadata {
    pub name: String,
    pub model: Option<String>,
    pub act_count: usize,
    pub description: Option<String>,
}

pub enum NarrativeViewMode {
    List,          // Simple list
    Tree,          // Hierarchical by category
    Grid,          // Grid with previews
    Detail,        // Single narrative detail view
}
```

### 3. Widget System

```rust
// crates/botticelli_chat/src/tui/widgets/mod.rs

use tui_tree_widget::{Tree, TreeState};
use tui_textarea::TextArea;

pub trait Widget {
    fn render(&self, area: Rect, buf: &mut Buffer, state: &AppState);
    fn handle_event(&mut self, event: &Event, state: &mut AppState) -> EventResult;
}

/// Navigation panel using tui-tree-widget
pub struct NavigationPanel {
    pub tree_state: TreeState<String>,
    pub tree_items: Vec<TreeItem<'static, String>>,
    pub title: String,
}

impl NavigationPanel {
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let tree = Tree::new(&self.tree_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(&self.title))
            .highlight_style(Style::default().fg(Color::Yellow));

        tree.render(area, buf, &mut self.tree_state);
    }
}

/// Rich text input using tui-textarea
pub struct ChatInput {
    pub textarea: TextArea<'static>,
}

impl ChatInput {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default()
            .borders(Borders::ALL)
            .title("Chat Input (Ctrl+Enter to send)"));
        textarea.set_placeholder_text("Type your message...");
        Self { textarea }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        self.textarea.widget().render(area, buf);
    }

    pub fn handle_input(&mut self, input: crossterm::event::KeyEvent) -> bool {
        // Returns true if message should be sent
        if input.code == KeyCode::Enter && input.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }
        self.textarea.input(input);
        false
    }

    pub fn take_content(&mut self) -> String {
        let content = self.textarea.lines().join("\n");
        self.textarea.select_all();
        self.textarea.delete_str(0, content.len());
        content
    }
}

pub struct ContentArea {
    pub widget: Box<dyn Widget>,
}

pub struct StatusBar {
    pub items: Vec<StatusItem>,
}

pub enum EventResult {
    Handled,
    NotHandled,
    ShouldQuit,
}
```

### 4. Command System

```rust
// crates/botticelli_chat/src/tui/commands.rs

pub enum TuiCommand {
    /// Navigation
    SwitchTab(Tab),
    SelectNext,
    SelectPrevious,
    GoBack,

    /// Actions
    OpenModal(Modal),
    CloseModal,
    ExecuteAction(Action),

    /// File operations
    OpenFile(PathBuf),
    SaveFile { path: PathBuf, content: String },
    CreateFile { template: String },

    /// Search
    StartSearch,
    FilterBy(FilterCriteria),

    /// Quit
    Quit,
}

pub enum Action {
    CreateNarrative,
    EditNarrative(PathBuf),
    ExecuteNarrative(PathBuf),
    ValidateNarrative(PathBuf),

    CreateBot(String),
    AssignNarrative { bot_id: String, narrative: PathBuf },
    StartBot(String),
    StopBot(String),

    RunQuery(String),
    ExportData { table: String, format: ExportFormat },

    SchedulePost { bot_id: String, time: DateTime<Utc> },
}
```

---

## Implementation Roadmap

**Revised Timeline:** 3-4 weeks (down from 5 weeks by using ecosystem libraries)

### Phase 1: Foundation + Library Integration (Week 1)

**Goal:** Refactor TUI into modular architecture and integrate core libraries

#### Task 1.1: Add Dependencies
- **File:** `crates/botticelli_chat/Cargo.toml`
- **Action:** Add ratatui ecosystem dependencies
  ```toml
  [dependencies]
  tui-tree-widget = "0.22"
  tui-textarea = "0.6"
  tui-logger = "0.12"
  directories = "5.0"

  [dev-dependencies]
  color_eyre = "0.6"
  ```
- **Acceptance:**
  - [ ] Dependencies compile
  - [ ] No version conflicts
  - [ ] Examples from docs work

#### Task 1.2: Create Module Structure
- **Files:** Create directory structure
  ```
  crates/botticelli_chat/src/tui/
  ├── mod.rs           # Re-exports
  ├── app.rs           # Main app struct
  ├── state.rs         # AppState
  ├── events.rs        # Event handling
  ├── commands.rs      # Command system
  ├── tabs/
  │   ├── mod.rs
  │   ├── narratives.rs
  │   ├── bots.rs
  │   ├── database.rs
  │   ├── chat.rs
  │   ├── schedule.rs
  │   └── settings.rs
  └── widgets/
      ├── mod.rs
      ├── navigation_panel.rs  # Uses tui-tree-widget
      ├── chat_input.rs        # Uses tui-textarea
      ├── status_bar.rs
      ├── tab_bar.rs
      └── modal.rs
  ```
- **Acceptance:**
  - [ ] Modules compile
  - [ ] Old tui.rs code still works (backwards compatible)
  - [ ] Tests pass

#### Task 1.2: Implement AppState
- **File:** `crates/botticelli_chat/src/tui/state.rs`
- **Action:** Create AppState with tab states
- **Acceptance:**
  - [ ] AppState holds all UI state
  - [ ] Tab switching logic works
  - [ ] Service container integrated

#### Task 1.3: Create Navigation Panel Widget
- **File:** `crates/botticelli_chat/src/tui/widgets/navigation_panel.rs`
- **Action:** Wrapper around tui-tree-widget
  ```rust
  use tui_tree_widget::{Tree, TreeItem, TreeState};

  pub struct NavigationPanel {
      tree_state: TreeState<String>,
      tree_items: Vec<TreeItem<'static, String>>,
      title: String,
  }

  impl NavigationPanel {
      pub fn new(title: impl Into<String>) -> Self {
          Self {
              tree_state: TreeState::default(),
              tree_items: Vec::new(),
              title: title.into(),
          }
      }

      pub fn set_items(&mut self, items: Vec<TreeItem<'static, String>>) {
          self.tree_items = items;
      }

      pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
          let tree = Tree::new(&self.tree_items)
              .block(Block::default()
                  .borders(Borders::ALL)
                  .title(&self.title))
              .highlight_style(Style::default()
                  .fg(Color::Yellow)
                  .add_modifier(Modifier::BOLD));

          tree.render(area, buf, &mut self.tree_state);
      }

      pub fn handle_key(&mut self, key: KeyCode) {
          match key {
              KeyCode::Down | KeyCode::Char('j') => self.tree_state.key_down(),
              KeyCode::Up | KeyCode::Char('k') => self.tree_state.key_up(),
              KeyCode::Right | KeyCode::Char('l') => self.tree_state.open(),
              KeyCode::Left | KeyCode::Char('h') => self.tree_state.close(),
              KeyCode::Enter | KeyCode::Char(' ') => self.tree_state.toggle_selected(),
              _ => {}
          }
      }

      pub fn selected(&self) -> Option<&Vec<String>> {
          self.tree_state.selected()
      }
  }
  ```
- **Acceptance:**
  - [ ] Widget compiles
  - [ ] Renders tree correctly
  - [ ] Keyboard navigation works (vim keys + arrows)
  - [ ] Can get selected item

#### Task 1.4: Create Chat Input Widget
- **File:** `crates/botticelli_chat/src/tui/widgets/chat_input.rs`
- **Action:** Wrapper around tui-textarea
  ```rust
  use tui_textarea::TextArea;

  pub struct ChatInput {
      textarea: TextArea<'static>,
  }

  impl ChatInput {
      pub fn new() -> Self {
          let mut textarea = TextArea::default();
          textarea.set_block(Block::default()
              .borders(Borders::ALL)
              .title("Input (Ctrl+Enter to send, Ctrl+Z to undo)"));
          textarea.set_placeholder_text("Type your message...");

          Self { textarea }
      }

      pub fn render(&self, area: Rect, buf: &mut Buffer) {
          self.textarea.widget().render(area, buf);
      }

      pub fn handle_input(&mut self, event: KeyEvent) -> InputResult {
          // Ctrl+Enter sends message
          if event.code == KeyCode::Enter
             && event.modifiers.contains(KeyModifiers::CONTROL) {
              return InputResult::SendMessage;
          }

          // Regular input
          self.textarea.input(event);
          InputResult::Continue
      }

      pub fn take_content(&mut self) -> String {
          let content = self.textarea.lines().join("\n");
          self.textarea.select_all();
          self.textarea.cut();
          content
      }

      pub fn is_empty(&self) -> bool {
          self.textarea.lines().iter().all(|l| l.trim().is_empty())
      }
  }

  pub enum InputResult {
      SendMessage,
      Continue,
  }
  ```
- **Acceptance:**
  - [ ] Widget compiles
  - [ ] Multi-line editing works
  - [ ] Undo/redo works (Ctrl+Z/Y)
  - [ ] Ctrl+Enter sends message
  - [ ] Content can be extracted

#### Task 1.5: Create Base Widgets
- **Files:** Status bar, tab bar, modal
- **Action:** Custom widgets for app-specific UI
- **Acceptance:**
  - [ ] Widgets render correctly
  - [ ] Event handling works
  - [ ] Composable layout

### Phase 2: Narratives Tab (Week 2)

**Goal:** Full-featured narrative browser and manager using tree widget

#### Task 2.1: Narrative Discovery
- **File:** `crates/botticelli_chat/src/tui/tabs/narratives.rs`
- **Action:** Scan narratives directory, extract metadata
  ```rust
  use tui_tree_widget::TreeItem;

  async fn discover_narratives(path: &Path) -> Result<Vec<NarrativeEntry>> {
      let mut narratives = Vec::new();
      for entry in walkdir::WalkDir::new(path) {
          let entry = entry?;
          if entry.path().extension() == Some("toml".as_ref()) {
              let metadata = extract_metadata(entry.path()).await?;
              narratives.push(NarrativeEntry {
                  path: entry.path().to_path_buf(),
                  name: entry.file_name().to_string_lossy().to_string(),
                  metadata,
                  last_modified: entry.metadata()?.modified()?,
                  category: infer_category(entry.path()),
              });
          }
      }
      Ok(narratives)
  }

  fn build_tree_items(narratives: &[NarrativeEntry]) -> Vec<TreeItem<'static, String>> {
      // Group by category
      let mut categories: HashMap<String, Vec<&NarrativeEntry>> = HashMap::new();
      for narrative in narratives {
          categories.entry(narrative.category.clone())
              .or_default()
              .push(narrative);
      }

      // Build tree structure
      let mut items = Vec::new();

      // Add "Recent" category first
      let recent: Vec<_> = narratives.iter()
          .sorted_by_key(|n| n.last_modified)
          .rev()
          .take(10)
          .map(|n| TreeItem::new_leaf(format!("{} ({})", n.name, n.metadata.model.as_deref().unwrap_or("default"))))
          .collect();

      if !recent.is_empty() {
          items.push(TreeItem::new("📌 Recent", recent).unwrap());
      }

      // Add category groups
      for (category, cat_narratives) in categories {
          let children: Vec<_> = cat_narratives.iter()
              .map(|n| TreeItem::new_leaf(n.name.clone()))
              .collect();

          let label = format!("📁 {} ({})", category, children.len());
          items.push(TreeItem::new(label, children).unwrap());
      }

      items
  }
  ```
- **Acceptance:**
  - [ ] Finds all .toml files in narratives/
  - [ ] Extracts name, model, act count
  - [ ] Categorizes by directory
  - [ ] Builds tree structure
  - [ ] "Recent" section shows 10 most recent

#### Task 2.2: Narrative Tree View with Navigation Panel
- **Action:** Use NavigationPanel widget with narrative tree
- **Layout:**
  ```
  ┌─ Narratives ────────────────┐
  │ ▼ 📌 Recent                 │
  │   ├─ showcase.toml (claude) │
  │   └─ daily.toml (gemini)    │
  │ ▼ 📁 Discord (40)           │
  │   ├─ daily_showcase.toml    │
  │   ├─ welcome.toml           │
  │   └─ ...                    │
  │ ▶ 📁 Examples (10)          │
  │ ▶ 📁 Tests (5)              │
  └─────────────────────────────┘
  ```
- **Code:**
  ```rust
  impl NarrativesTab {
      fn render_navigation(&mut self, area: Rect, buf: &mut Buffer) {
          // Build tree items on first render or when narratives change
          if self.tree_items.is_empty() {
              self.tree_items = build_tree_items(&self.narratives);
          }

          self.nav_panel.set_items(self.tree_items.clone());
          self.nav_panel.render(area, buf);
      }

      fn handle_navigation_key(&mut self, key: KeyCode) {
          self.nav_panel.handle_key(key);

          // Update selected narrative based on tree selection
          if let Some(path) = self.nav_panel.selected() {
              self.selected_narrative = self.find_narrative_by_path(path);
          }
      }
  }
  ```
- **Acceptance:**
  - [ ] Tree renders with categories
  - [ ] Selection works (arrows, vim keys)
  - [ ] Expansion/collapse works (Enter, Space)
  - [ ] Selected narrative updates
  - [ ] Filtering works

#### Task 2.3: Narrative Detail View
- **Action:** Show full narrative info when selected
- **Acceptance:**
  - [ ] Shows all acts in TOC
  - [ ] Shows model/temperature settings
  - [ ] Shows file path
  - [ ] Actions available (Edit, Execute, Validate)

#### Task 2.4: Quick Actions
- **Action:** Implement keyboard shortcuts
  - `E` - Edit in $EDITOR
  - `X` - Execute narrative
  - `V` - Validate TOML
  - `N` - Create new (wizard)
  - `D` - Duplicate
  - `/` - Search/filter
- **Acceptance:**
  - [ ] All shortcuts work
  - [ ] Opens editor correctly
  - [ ] Executes and shows results
  - [ ] Validation shows errors

### Phase 3: Other Tabs (Week 3)

#### Task 3.1: Bots Tab
- List bots from database
- Show status (active/inactive)
- Show assigned narratives
- Allow start/stop/assign actions

#### Task 3.2: Database Tab
- List tables with row counts
- Show table schema
- Simple query interface
- Results pagination

#### Task 3.3: Schedule Tab
- List upcoming scheduled posts
- Show past executions
- Allow create/edit/cancel

#### Task 3.4: Settings Tab
- Load from ChatAppConfig
- Allow editing
- Save to TOML
- Test connections (DB, MCP)

### Phase 4: Chat Tab Enhancement (Week 3-4)

#### Task 4.1: Integrate Rich Chat Input ✅ COMPLETE
- **File:** `crates/botticelli_chat/src/tui/tabs/chat.rs`
- **Status:** Implemented with ChatInput widget integration
- **Implementation:**
  - ChatMessage struct with role, content, timestamp
  - MessageRole enum (System, User, Assistant) with styling
  - Multi-line input using ChatInput widget
  - Scrolling support (Ctrl+Up/Down)
  - Clear chat (Ctrl+L)
  - Message history with timestamps
  - Auto-scroll to latest message
- **Acceptance:**
  - [x] Multi-line input works
  - [x] Undo/redo works (provided by tui-textarea)
  - [x] Ctrl+Enter sends
  - [x] Message history preserved
  - [x] Scrolling with scrollbar
  - [x] Clear chat command

#### Task 4.2: Add Tool Call Visibility
- **Action:** Show tool calls in chat messages
  ```rust
  enum ChatMessage {
      User { content: String },
      Assistant { content: String },
      ToolCall {
          tool_name: String,
          input: serde_json::Value
      },
      ToolResult {
          tool_name: String,
          output: serde_json::Value,
          is_error: bool,
      },
      Thinking { content: String },
  }
  ```
- **Rendering:**
  ```
  You: Create a narrative about space

  Assistant: I'll help create that narrative...

  🔧 Tool: create_narrative_session
     Input: {"description": "space narrative"}

  ✅ Result: {"session_id": "123", "status": "created"}

  Assistant: I've created a session. What should...
  ```
- **Acceptance:**
  - [ ] Tool calls visible
  - [ ] Results shown
  - [ ] Errors highlighted
  - [ ] Thinking/reasoning shown

#### Task 4.3: Add Conversation History
- **Action:** Save conversations to XDG data directory
  ```rust
  use directories::ProjectDirs;

  fn save_conversation(messages: &[ChatMessage]) -> Result<()> {
      if let Some(proj_dirs) = ProjectDirs::from("com", "botticelli", "Botticelli") {
          let data_dir = proj_dirs.data_dir();
          let convo_file = data_dir.join("conversations")
              .join(format!("{}.json", Utc::now().timestamp()));

          std::fs::create_dir_all(convo_file.parent().unwrap())?;
          let json = serde_json::to_string_pretty(&messages)?;
          std::fs::write(convo_file, json)?;
      }
      Ok(())
  }
  ```
- **Acceptance:**
  - [ ] Conversations saved to XDG data dir
  - [ ] Can load previous conversations
  - [ ] History browsable in UI

### Phase 5: Monitoring & Polish (Week 4)

#### Task 5.0: Add Logger Tab
- **File:** `crates/botticelli_chat/src/tui/tabs/logs.rs`
- **Action:** Integrate tui-logger widget
  ```rust
  use tui_logger::{TuiLoggerWidget, TuiLoggerLevelOutput};

  pub struct LogsTab {
      // tui-logger manages state internally
  }

  impl LogsTab {
      pub fn new() -> Self {
          // Initialize tui-logger
          tui_logger::init_logger(log::LevelFilter::Debug).unwrap();
          tui_logger::set_default_level(log::LevelFilter::Info);

          Self {}
      }

      pub fn render(&self, area: Rect, buf: &mut Buffer) {
          let widget = TuiLoggerWidget::default()
              .block(Block::default()
                  .title("Logs")
                  .borders(Borders::ALL))
              .output_separator(':')
              .output_timestamp(Some("%H:%M:%S".to_string()))
              .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
              .output_target(true)
              .output_file(false);

          widget.render(area, buf);
      }
  }
  ```
- **Acceptance:**
  - [ ] Real-time log viewing
  - [ ] Log level filtering
  - [ ] Scrollable log history
  - [ ] Integrated with existing tracing

### Phase 6: Polish & Features (Ongoing)

#### Task 5.1: Help System
- Comprehensive help modal
- Context-sensitive help
- Keyboard shortcut reference

#### Task 5.2: Search & Filter
- Global search (Ctrl+P style)
- Per-tab filtering
- Fuzzy matching

#### Task 5.3: Themes & Customization
- Color scheme support
- User preferences
- Keybinding customization

---

## Technical Considerations

### 1. Performance

**Lazy Loading:**
- Don't load all narratives at startup
- Load on-demand when tab opens
- Cache parsed TOML

**Pagination:**
- Database results paginated
- Large lists virtualized
- Smooth scrolling

**Async Operations:**
- All I/O async
- Show loading states
- Non-blocking UI updates

### 2. State Management

**Separation of Concerns:**
```rust
pub struct App {
    state: AppState,         // UI state
    services: ServiceContainer,  // Business logic
}

impl App {
    // UI concerns
    fn render(&self, frame: &mut Frame) { }
    fn handle_event(&mut self, event: Event) { }

    // Delegate to services
    async fn execute_narrative(&mut self, path: &Path) {
        self.services.narrative_repository()
            .execute(path)
            .await
    }
}
```

**State Updates:**
- Immutable where possible
- Clear update paths
- Event-driven changes

### 3. Error Handling

**User-Friendly Errors:**
```rust
match result {
    Ok(narrative) => {
        self.state.modal = Some(Modal::Success {
            message: format!("Executed {} successfully", narrative.name),
        });
    }
    Err(e) => {
        self.state.modal = Some(Modal::Error {
            title: "Execution Failed",
            message: format!("Error: {}\n\nPress Enter to continue", e),
            details: Some(format!("{:?}", e)),  // Debug info
        });
    }
}
```

### 4. Testing

**Unit Tests:**
- State transitions
- Widget rendering
- Command handling

**Integration Tests:**
- Tab switching
- File operations
- Service integration

**Snapshot Tests:**
- UI layout tests
- Rendering verification

---

## Migration Strategy

### Backwards Compatibility

1. **Keep old TUI working** until new one is ready
2. **Feature flag:** `tui_v2` for new implementation
3. **Gradual rollout:** Start with narratives tab, add others
4. **User choice:** Allow switching between old/new

### Rollout Plan

**Week 1-2:**
- New code alongside old
- Feature flag off by default
- Internal testing

**Week 3-4:**
- Feature flag on by default
- Old version available with `--legacy-ui`
- Gather feedback

**Week 5+:**
- Remove old implementation
- New TUI is default
- Documentation updated

---

## Learning from Ecosystem

### Apps to Study

**Oatmeal** - Terminal chat with LLM support
- Highly relevant to our chat tab
- Study their message rendering
- Learn LLM integration patterns
- **Action:** Find and review source code

**Yōzefu** - Kafka cluster browser
- Multi-tab data navigation
- Similar to our database tab
- Table rendering patterns

### Best Practices Applied

**Architecture:**
- ✅ MVC pattern (App → State → Render)
- ✅ Event-driven message passing
- ✅ Component-based organization

**Configuration:**
- ✅ XDG directories for user config
- ✅ Workspace config in repo
- ✅ Environment variables for secrets

**Logging:**
- ✅ tui-logger widget for real-time viewing
- ✅ File-based logs for debugging
- ✅ Don't interfere with TUI

**Error Handling:**
- ✅ color_eyre for beautiful error displays (dev)
- ✅ Keep botticelli_error for production
- ✅ Modal dialogs for user-facing errors

---

## Success Metrics

### User Experience

1. **Discoverability:**
   - ✅ Can find all 50+ narratives without knowing filenames
   - ✅ Can see what bots are active
   - ✅ Can browse database tables
   - ✅ Can access all 29 MCP tools

2. **Efficiency:**
   - ✅ Create narrative: 2 actions (Tab to Narratives, N for new)
   - ✅ Execute narrative: 3 actions (Tab, arrow to select, X)
   - ✅ View bot status: 1 action (Tab to Bots)

3. **Familiarity:**
   - ✅ Follows conventions from htop/lazygit/k9s
   - ✅ Standard shortcuts (?, Ctrl+C, Ctrl+L)
   - ✅ Clear visual hierarchy

### Technical Quality

1. **Performance:**
   - ✅ Startup < 1 second
   - ✅ Tab switching < 100ms
   - ✅ No UI blocking on I/O

2. **Maintainability:**
   - ✅ Modular architecture
   - ✅ Clear separation of concerns
   - ✅ Comprehensive tests

3. **Extensibility:**
   - ✅ Easy to add new tabs
   - ✅ Widget system supports composition
   - ✅ Command system is extensible

---

## Next Steps

1. **Review this plan** - Any changes or concerns?
2. **Prioritize features** - Which tabs are most important?
3. **Start Phase 1** - Create modular foundation
4. **Iterate** - Build tab by tab, test with users

---

## Time Savings from Ecosystem

### Original Estimate
- Phase 1: 1 week (foundation)
- Phase 2: 1 week (narratives)
- Phase 3: 1 week (other tabs)
- Phase 4: 1 week (chat)
- Phase 5: 1 week (polish)
- **Total: 5 weeks**

### Revised with Libraries
- Phase 1: 1 week (foundation + library integration)
- Phase 2: 1 week (narratives using tree widget)
- Phase 3: 1 week (other tabs, logger widget)
- Phase 4: Concurrent with Phase 3
- **Total: 3-4 weeks**

### Savings Breakdown
- **tui-tree-widget:** 2-3 days saved (200+ lines not written)
- **tui-textarea:** 1-2 days saved (rich editing for free)
- **tui-logger:** 1 day saved (monitoring tab ready)
- **Best practices:** Fewer bugs, faster development
- **Total saved: 1-2 weeks**

### What We're NOT Building
- ❌ Custom tree navigation logic
- ❌ Text editing with undo/redo
- ❌ Log filtering and display
- ❌ File browser (if we use ratatui-explorer)

### What We ARE Building
- ✅ Botticelli domain logic
- ✅ Narrative management
- ✅ Bot orchestration
- ✅ Database integration
- ✅ LLM sampling integration
- ✅ Our unique features

---

## Next Steps

1. **Review updated plan** - Any changes needed?
2. **Study Oatmeal** - Learn from similar app
3. **Start Phase 1** - Add dependencies, create modules
4. **Iterate quickly** - Libraries accelerate development

**Ready to begin?** I can start with Phase 1, Task 1.1: Adding dependencies and creating the module structure.

---

## Implementation Tracking

**Started:** 2024-12-14

### Phase 1: Foundation ✅ COMPLETE
**Status:** Complete  
**Date:** 2024-12-14

- ✅ Task 1.1: Add dependencies (uuid) to Cargo.toml
- ✅ Task 1.2: Create module structure (app, commands, error, events, state, view)
- ✅ Task 1.3: Define core types (AppState, ViewMode, ChatMessage, ConversationId, NarrativeId)

### Phase 2: Core Types & Event Loop ✅ COMPLETE
**Status:** Complete  
**Date:** 2024-12-14

- ✅ Task 2.1: Implement AppState with mode tracking, conversation management, input buffer
- ✅ Task 2.2: Create Command enum for TUI actions
- ✅ Task 2.3: Implement EventHandler with keyboard/resize/tick events
- ✅ Task 2.4: Create App coordinator with event loop, terminal setup/restore, command handling
- ✅ Task 2.5: Implement View trait with ChatView (basic rendering)
- ✅ Task 2.6: Add stub views (NarrativeBrowserView, NarrativeEditorView)

**Files Refactored:**
- Deleted: app.rs, backend.rs, database_backend.rs, events.rs, ui.rs, views.rs, runner.rs (old architecture)
- Created: app.rs, commands.rs, events.rs, state.rs, view.rs (new architecture)
- Updated: lib.rs (new module structure), Cargo.toml (uuid dependency)

**Architecture Changes:**
- Moved from monolithic tui.rs to modular architecture
- Separated concerns: state management, event handling, view rendering, command processing
- Created clean View trait for extensibility

### Phase 3: UI Components ✅ COMPLETE
**Status:** Complete  
**Date:** 2024-12-14

- ✅ Task 3.1: Implement ChatView with message display and input buffer
- ✅ Task 3.2: Implement NarrativeBrowserView with list navigation
- ✅ Task 3.3: Implement NarrativeEditorView with content display

**Files Created:**
- All views implemented in view.rs with basic rendering

### Phase 4: Event Handling & State Updates ✅ COMPLETE
**Status:** Complete  
**Date:** 2024-12-14

- ✅ Task 4.1: Input handling per view (keyboard shortcuts)
- ✅ Task 4.2: Rendering logic for all views
- ✅ Task 4.3: State update logic (navigation, input buffer, mode switching)

**Features Implemented:**
- Global keyboard shortcuts (Alt+1/2/3 for tab switching, Ctrl+Q to quit)
- Chat input handling (typing, backspace, Enter to send)
- Narrative browser navigation (j/k/arrow keys, Enter to edit)
- Narrative editor controls (Ctrl+S to save, Esc to go back)
- Input buffer management with character append and delete

**Files Updated:**
- commands.rs: Added AppendChar, DeleteChar commands
- app.rs: Added handle_global_input() method, enhanced command handling
- view.rs: Enhanced input handling for ChatView with typing support

### Phase 5: Integration (NOT STARTED)
**Next:** Wire up actual backend services

- ⬜ Task 5.1: Add ConversationSession to App
- ⬜ Task 5.2: Implement SendMessage command with actual LLM integration
- ⬜ Task 5.3: Add message history rendering with scrolling
- ⬜ Task 5.4: Add conversation loading/saving
- ⬜ Task 5.5: Narrative loading/saving integration
- ⬜ Task 5.6: Add cursor movement in input

### Phase 6: Narrative Views Enhancement (NOT STARTED)
- ⬜ Advanced NarrativeBrowserView features (search, filter)
- ⬜ Enhanced NarrativeEditorView (syntax highlighting, validation)
- ⬜ Narrative TOML parsing display

### Phase 5: Settings View (NOT STARTED)
- ⬜ Settings view implementation
- ⬜ Configuration display/editing

### Phase 6: Polish (NOT STARTED)
- ⬜ Better error display
- ⬜ Loading indicators
- ⬜ Help overlay
- ⬜ Keyboard shortcut hints

**Current Focus:** Basic TUI refactor complete with event handling and state updates. Ready for Phase 5 backend integration or Phase 6 polish.
