# Botticelli TUI Redesign: Comprehensive Planning Document

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

pub struct NarrativesTabState {
    /// List of discovered narratives
    pub narratives: Vec<NarrativeEntry>,

    /// Currently selected narrative
    pub selected_index: usize,

    /// Navigation tree state
    pub tree: TreeState,

    /// Filter/search
    pub filter: Option<String>,

    /// View mode
    pub view_mode: NarrativeViewMode,
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

pub trait Widget {
    fn render(&self, area: Rect, buf: &mut Buffer, state: &AppState);
    fn handle_event(&mut self, event: &Event, state: &mut AppState) -> EventResult;
}

pub struct NavigationPanel<T> {
    pub items: Vec<NavItem<T>>,
    pub selected: usize,
    pub title: String,
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

### Phase 1: Foundation (Week 1)

**Goal:** Refactor TUI into modular architecture

#### Task 1.1: Create Module Structure
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
      ├── navigation_panel.rs
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

#### Task 1.3: Create Base Widgets
- **Files:** Widget trait and implementations
- **Action:** Navigation panel, status bar, tab bar
- **Acceptance:**
  - [ ] Widgets render correctly
  - [ ] Event handling works
  - [ ] Composable layout

### Phase 2: Narratives Tab (Week 2)

**Goal:** Full-featured narrative browser and manager

#### Task 2.1: Narrative Discovery
- **File:** `crates/botticelli_chat/src/tui/tabs/narratives.rs`
- **Action:** Scan narratives directory, extract metadata
  ```rust
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
  ```
- **Acceptance:**
  - [ ] Finds all .toml files in narratives/
  - [ ] Extracts name, model, act count
  - [ ] Categorizes by directory
  - [ ] Caches results

#### Task 2.2: Narrative List View
- **Action:** Render narrative list with metadata
- **Layout:**
  ```
  Name                          Model              Acts  Modified
  ──────────────────────────────────────────────────────────────
  showcase.toml                 claude-3-5-sonnet  3     2024-12-14
  discord/daily_showcase.toml   gemini-pro         5     2024-12-13
  ```
- **Acceptance:**
  - [ ] List renders
  - [ ] Selection works (up/down keys)
  - [ ] Shows metadata
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

### Phase 4: Chat Tab Enhancement (Week 4)

#### Task 4.1: Preserve Current Chat
- Keep existing chat interface
- Add to tab system
- Add conversation history

#### Task 4.2: Add Tool Call Visibility
- Show when LLM calls tools
- Display tool results
- Show thinking/reasoning

#### Task 4.3: Prompt Management
- Save frequently used prompts
- Load system prompts
- Quick prompt selection

### Phase 5: Polish & Features (Week 5)

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

**Ready to begin?** I can start with Phase 1, Task 1.1: Creating the module structure and base architecture.
