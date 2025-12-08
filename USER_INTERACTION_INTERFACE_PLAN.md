# User Interaction Interface Design

**Status**: In Progress (Step 11)  
**Created**: 2025-12-08  
**Updated**: 2025-12-08 18:08 UTC  
**Goal**: Define a unified trait-based interface for user-Botticelli interactions across multiple frontends (TUI, web, mobile)

---

## Implementation Tracker

| Step | Status | Description |
|------|--------|-------------|
| **Step 1** | ✅ Complete | Project scaffolding |
| **Step 2** | ✅ Complete | Error types and result aliases |
| **Step 3** | ✅ Complete | Core types (Message, UserInput, Command, Response) |
| **Step 4** | ✅ Complete | ChatInterface trait definition |
| **Step 5** | ✅ Complete | State management (ConversationState) |
| **Step 6** | ✅ Complete | Intent parser |
| **Step 7** | ✅ Complete | Command executor stub |
| **Step 8** | ✅ Complete | Unit tests for core types |
| **Step 9** | ✅ Complete | TUI implementation using ratatui |
| **Step 10** | ✅ Complete | Handler integration |
| **Step 11** | 📋 Planned | Example binary for TUI |
| **Step 12** | 📋 Planned | Integration tests |
| **Step 13** | 📋 Planned | Documentation |

**Current Step**: Step 11 - Example Binary  
**Blocked By**: None

**Next Actions**:
1. ⏭️ Create example binary
2. ⏭️ Add integration tests
3. ⏭️ Write user documentation

**Step 10 Completed**:
- Created `CommandExecutor` with async command handling
- Integrated executor with TUI for real-time command processing
- Added `NarrativeState` for tracking narrative construction
- Implemented handlers for narrative, bot, and social commands
- Connected parser → executor → TUI response flow

---

## Vision

### Problem Statement

Users need a natural, interactive way to:
- Generate narratives through conversation
- Assign narratives to bots
- Schedule social media posts
- Monitor bot activities
- Configure system behavior

Currently, these operations require:
- Direct file editing (TOML)
- CLI commands
- MCP tool invocation via external clients

### Solution

Create a **unified interaction interface** that:
1. **Abstracts the presentation layer** - Same logic works in terminal, browser, mobile
2. **Enables natural conversation** - Users describe what they want, Botticelli executes
3. **Provides real-time feedback** - Progress updates, errors, completions
4. **Maintains state** - Conversation context, user preferences, active tasks

---

## Architecture

### Core Traits

```rust
// Location: crates/botticelli_interface/src/interaction.rs

/// Core trait for user interaction interfaces.
pub trait UserInterface: Send + Sync {
    /// Display a message to the user.
    async fn display_message(&mut self, message: Message) -> InterfaceResult<()>;
    
    /// Get input from the user.
    async fn get_input(&mut self) -> InterfaceResult<UserInput>;
    
    /// Show progress indicator.
    async fn show_progress(&mut self, progress: Progress) -> InterfaceResult<()>;
    
    /// Display available actions.
    async fn display_actions(&mut self, actions: Vec<Action>) -> InterfaceResult<()>;
    
    /// Get the current interface state.
    fn state(&self) -> &InterfaceState;
}

/// Handles conversation and command interpretation.
pub trait InteractionHandler: Send + Sync {
    /// Process user input and determine action.
    async fn handle_input(&mut self, input: UserInput) -> InterfaceResult<Response>;
    
    /// Execute a specific command.
    async fn execute_command(&mut self, command: Command) -> InterfaceResult<CommandResult>;
    
    /// Get conversation history.
    fn history(&self) -> &[Interaction];
    
    /// Reset conversation state.
    fn reset(&mut self);
}

/// Manages session state and context.
pub trait SessionManager: Send + Sync {
    /// Start a new session.
    async fn start_session(&mut self) -> InterfaceResult<SessionId>;
    
    /// End current session.
    async fn end_session(&mut self, id: SessionId) -> InterfaceResult<()>;
    
    /// Get active session.
    fn active_session(&self) -> Option<&Session>;
    
    /// Store session data.
    async fn save_session(&mut self, session: &Session) -> InterfaceResult<()>;
}
```

### Message Types

```rust
/// A message displayed to the user.
pub enum Message {
    /// System message (status, info)
    System(String),
    
    /// Botticelli response
    Assistant(String),
    
    /// User message (echoed back)
    User(String),
    
    /// Error message
    Error(String),
    
    /// Success confirmation
    Success(String),
    
    /// Warning
    Warning(String),
    
    /// Structured data display
    Data(DataView),
}

/// User input types.
pub enum UserInput {
    /// Text command or message
    Text(String),
    
    /// Selection from menu
    Selection(usize),
    
    /// File upload
    File(PathBuf),
    
    /// Confirmation (yes/no)
    Confirm(bool),
    
    /// Cancel operation
    Cancel,
}

/// Response from interaction handler.
pub enum Response {
    /// Continue conversation
    Continue(Message),
    
    /// Execute command
    Execute(Command),
    
    /// Request clarification
    Clarify(Question),
    
    /// Show options
    Options(Vec<Action>),
    
    /// Task completed
    Complete(TaskResult),
}

/// Commands the system can execute.
pub enum Command {
    /// Narrative operations
    Narrative(NarrativeCommand),
    
    /// Bot operations
    Bot(BotCommand),
    
    /// Social media operations
    Social(SocialCommand),
    
    /// System operations
    System(SystemCommand),
}
```

### Workflow Model

```
User Input → Parser → Intent Recognition → Command Generation → Execution → Feedback
     ↑                                                                          ↓
     └──────────────────────── Clarification Loop ─────────────────────────────┘
```

---

## Phase 1: Core Trait Definition

### Goals

1. Define interaction traits that work across all UI paradigms
2. Create message and command enums
3. Design state management
4. Plan error handling

### Deliverables

- [ ] `interaction.rs` - Core UserInterface trait
- [ ] `handler.rs` - InteractionHandler implementation
- [ ] `session.rs` - SessionManager implementation
- [ ] `types.rs` - Message, Command, Response types
- [ ] `parser.rs` - Intent parsing and command generation
- [ ] Unit tests for trait implementations
- [ ] Documentation with examples

### Design Principles

1. **Trait-Based Abstraction** - UI implementations are interchangeable
2. **Async by Default** - All operations support async/await
3. **Type Safety** - Strong types for commands, messages, states
4. **Error Transparency** - Clear error messages with recovery options
5. **State Persistence** - Sessions can be saved/restored

### Key Patterns

**Conversation Context:**
```rust
pub struct ConversationContext {
    /// Active task (if any)
    pub active_task: Option<Task>,
    
    /// Recent interactions
    pub history: Vec<Interaction>,
    
    /// User preferences
    pub preferences: UserPreferences,
    
    /// Workspace state
    pub workspace: WorkspaceState,
}
```

**Command Routing:**
```rust
impl InteractionHandler {
    async fn handle_input(&mut self, input: UserInput) -> InterfaceResult<Response> {
        // Parse intent
        let intent = self.parser.parse(&input)?;
        
        // Route to appropriate handler
        match intent {
            Intent::CreateNarrative => self.handle_narrative_creation(input).await,
            Intent::AssignBot => self.handle_bot_assignment(input).await,
            Intent::SchedulePost => self.handle_scheduling(input).await,
            Intent::Query => self.handle_query(input).await,
        }
    }
}
```

---

## Phase 2: TUI Implementation

### Goals

1. Implement UserInterface trait for terminal
2. Create interactive chat-like experience
3. Support keyboard navigation and shortcuts
4. Display progress and structured data

### Components

**TUI UserInterface:**
```rust
pub struct TuiInterface {
    /// Terminal backend
    terminal: Terminal<CrosstermBackend<Stdout>>,
    
    /// Message buffer
    messages: Vec<RenderedMessage>,
    
    /// Input state
    input: InputBuffer,
    
    /// Current view mode
    mode: TuiMode,
}

pub enum TuiMode {
    /// Chat interaction
    Chat,
    
    /// Menu selection
    Menu(MenuState),
    
    /// Data viewing
    DataView(DataViewState),
    
    /// Progress monitoring
    Progress(ProgressState),
}
```

**Features:**
- Scrollable message history
- Syntax highlighting for TOML/code
- Progress bars and spinners
- Keyboard shortcuts (Ctrl+C cancel, Ctrl+L clear, etc.)
- Multi-line input support
- Tab completion

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ Botticelli Interactive Console                    [v0.1.0] │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ System: Welcome! I can help you:                            │
│  • Generate narratives                                       │
│  • Assign narratives to bots                                │
│  • Schedule social media posts                              │
│                                                              │
│ You: Create a narrative for Discord stats analysis          │
│                                                              │
│ Assistant: I'll create a narrative for Discord stats.       │
│ What should the narrative name be?                          │
│                                                              │
│ You: discord_stats                                           │
│                                                              │
│ Assistant: ✓ Created narrative 'discord_stats'              │
│ Would you like to assign it to a bot? (y/n)                 │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ > _                                                          │
└─────────────────────────────────────────────────────────────┘
```

### TUI-Specific Features

- **Split panes** - Messages on top, input at bottom
- **Color coding** - Different message types have colors
- **Status bar** - Current mode, task status, shortcuts
- **Notifications** - Important updates highlighted
- **History navigation** - Arrow keys to replay commands

---

## Phase 3: Web Implementation (Leptos)

### Goals

1. Implement UserInterface trait for web browser
2. Create responsive chat interface
3. Support real-time updates via WebSockets
4. Enable rich media display

### Technology

**Leptos Framework:**
- Reactive UI components
- Server-side rendering
- WebAssembly support
- Type-safe signals

**Architecture:**
```rust
#[component]
pub fn ChatInterface() -> impl IntoView {
    let (messages, set_messages) = create_signal(Vec::<Message>::new());
    let (input, set_input) = create_signal(String::new());
    
    // WebSocket connection
    let ws = use_websocket("/api/interact");
    
    view! {
        <div class="chat-container">
            <MessageList messages=messages />
            <InputBox 
                value=input 
                on_send=move |text| {
                    ws.send(text);
                }
            />
        </div>
    }
}
```

### Web-Specific Features

- **Rich formatting** - Markdown rendering, code highlighting
- **File uploads** - Drag-and-drop TOML files
- **Media preview** - Images, videos inline
- **Mobile responsive** - Works on tablets and phones
- **Notifications** - Browser notifications for completions
- **Bookmarkable states** - URLs for specific conversations

---

## Phase 4: Mobile Considerations

### Goals

1. Ensure traits work on mobile platforms
2. Plan native app architecture (future)
3. Define mobile-specific UI patterns

### Challenges

- Smaller screens
- Touch interactions
- Background operation limits
- Offline mode support

### Strategy

**Progressive Web App (PWA):**
- Leptos web app works on mobile
- Install as PWA
- Offline caching
- Push notifications

**Future Native:**
- Use same trait interface
- Platform-specific UI (Swift/Kotlin)
- Rust core logic via FFI

---

## User Workflows

### Workflow 1: Create and Deploy Narrative

```
User: "Create a narrative that analyzes Discord activity"

Botticelli: "I'll help you create that narrative. 
             What should we name it?"

User: "discord_activity"

Botticelli: "Great! Which model should we use?
             1. Gemini (fast, good for analysis)
             2. Claude (detailed, thoughtful)
             3. GPT-4 (balanced)"

User: "1"

Botticelli: ✓ Created narrative 'discord_activity'
            ✓ Using Gemini 2.0 Flash
            ✓ 2 acts detected: fetch data, analyze
            
            Would you like to:
            1. Preview the TOML
            2. Assign to a bot
            3. Edit the narrative
            4. Save and exit

User: "2"

Botticelli: "Which bot should run this narrative?
             Available bots:
             1. stats_bot (Discord)
             2. reporter_bot (Discord)
             
             Or type a new bot name"

User: "stats_bot"

Botticelli: ✓ Assigned 'discord_activity' to stats_bot
            
            When should it run?
            1. Now (one-time)
            2. Schedule (recurring)
            3. Manual trigger

User: "2"

Botticelli: "How often? (examples: daily, hourly, weekly)"

User: "daily at 9am"

Botticelli: ✓ Scheduled stats_bot to run 'discord_activity' daily at 9:00 AM
            
            Summary:
            • Narrative: discord_activity
            • Bot: stats_bot
            • Schedule: Daily 9:00 AM
            • Next run: Tomorrow 9:00 AM
            
            All set! Type 'help' for more options.
```

### Workflow 2: Monitor Bot Activity

```
User: "Show me what stats_bot is doing"

Botticelli: stats_bot Status:
            
            Last Run: Today 9:03 AM (3 minutes ago)
            Status: ✓ Completed
            Narrative: discord_activity
            
            Results:
            • Fetched 1,247 messages
            • Analyzed activity patterns
            • Posted summary to #stats
            
            Next Run: Tomorrow 9:00 AM
            
            Options:
            1. View detailed logs
            2. See full output
            3. Modify schedule
            4. Back to main menu

User: "1"

Botticelli: [Shows detailed execution logs]
```

### Workflow 3: Quick Commands

```
User: "/create narrative discord_stats"
Botticelli: ✓ Created narrative 'discord_stats'

User: "/assign discord_stats to stats_bot"
Botticelli: ✓ Assigned

User: "/schedule stats_bot daily 9am"
Botticelli: ✓ Scheduled

User: "/status stats_bot"
Botticelli: [Shows status]

User: "/help"
Botticelli: [Shows command reference]
```

---

## Integration with Existing Systems

### MCP Tools Integration

The interaction handler can invoke MCP tools:

```rust
impl InteractionHandler {
    async fn handle_narrative_creation(&mut self, input: UserInput) -> InterfaceResult<Response> {
        // Use existing CreateNarrativeTool
        let tool = CreateNarrativeTool;
        
        let params = json!({
            "description": extract_description(&input),
            "name": extract_name(&input),
        });
        
        let result = tool.execute(params).await?;
        
        // Convert tool result to user-friendly message
        Ok(Response::Success(format_narrative_result(&result)))
    }
}
```

### Database Integration

Session persistence:

```rust
impl SessionManager {
    async fn save_session(&mut self, session: &Session) -> InterfaceResult<()> {
        // Use existing database crate
        let conn = self.pool.get()?;
        
        diesel::insert_into(sessions::table)
            .values(session.to_db_row())
            .execute(&mut conn)?;
            
        Ok(())
    }
}
```

### Narrative Execution

Trigger narrative execution:

```rust
impl BotCommandHandler {
    async fn run_narrative(&mut self, bot: &str, narrative: &str) -> InterfaceResult<ExecutionResult> {
        // Use existing ExecuteNarrativeTool
        let tool = ExecuteNarrativeTool;
        
        tool.execute(json!({
            "bot_id": bot,
            "narrative_path": format!("{}.toml", narrative),
        })).await?;
        
        Ok(ExecutionResult::Started)
    }
}
```

---

## Technical Considerations

### State Management

**Session State:**
```rust
pub struct Session {
    pub id: SessionId,
    pub user_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub context: ConversationContext,
    pub active_tasks: Vec<Task>,
}
```

**Persistence:**
- Sessions saved to database
- Can resume interrupted conversations
- History available across sessions

### Error Handling

**Error Categories:**
1. **User Errors** - Invalid input, missing info
2. **System Errors** - Database failures, network issues
3. **Execution Errors** - Narrative failures, bot errors

**Recovery:**
- Clear error messages
- Suggested fixes
- Retry options
- Rollback capabilities

### Security

**Considerations:**
- User authentication (future)
- Command authorization
- Rate limiting
- Input validation
- SQL injection prevention
- XSS protection (web)

---

## Testing Strategy

### Unit Tests

- Trait implementations
- Message parsing
- Command routing
- State management

### Integration Tests

- TUI rendering
- Web components
- Database operations
- MCP tool invocation

### User Testing

- Usability studies
- Workflow validation
- Performance testing
- Accessibility testing

---

## Future Enhancements

### Phase 5+

1. **Voice Interface** - Voice commands and responses
2. **Multi-User** - Collaborative sessions
3. **Plugins** - Extensible command system
4. **AI Assistance** - LLM-powered help
5. **Automation** - Workflow recording and replay
6. **Analytics** - Usage tracking and insights

---

## Implementation Timeline

### Phase 1: Core Traits (Week 1)
- Define traits and types
- Implement base handlers
- Create parser
- Write tests

### Phase 2: TUI (Week 2)
- Implement TUI interface
- Create chat components
- Add keyboard shortcuts
- Test workflows

### Phase 3: Web (Week 3-4)
- Setup Leptos project
- Implement web interface
- Add WebSocket support
- Deploy demo

### Phase 4: Polish (Week 5)
- Mobile responsive
- Documentation
- Examples
- User testing

---

## Success Criteria

### Phase 1
- [ ] All core traits defined
- [ ] Message types implemented
- [ ] Command routing works
- [ ] Tests passing

### Phase 2
- [ ] TUI displays messages correctly
- [ ] Input handling works
- [ ] Workflows complete successfully
- [ ] Keyboard shortcuts functional

### Phase 3
- [ ] Web interface renders
- [ ] Real-time updates work
- [ ] Mobile responsive
- [ ] Deployed and accessible

### Overall Success
- [ ] Users can create narratives conversationally
- [ ] Bot assignment is intuitive
- [ ] Scheduling works reliably
- [ ] Error messages are clear
- [ ] Performance is acceptable (< 500ms response)

---

## References

- [botticelli_interface](./crates/botticelli_interface/) - Existing trait patterns
- [botticelli_tui](./crates/botticelli_tui/) - TUI implementation
- [botticelli_mcp](./crates/botticelli_mcp/) - MCP tools to integrate
- [NARRATIVE_GENERATION_MCP_PLAN.md](./NARRATIVE_GENERATION_MCP_PLAN.md) - Related narrative work
- [Leptos Framework](https://leptos.dev/) - Web framework
- [Ratatui](https://ratatui.rs/) - TUI library

---

**Status**: Ready for Phase 1 implementation  
**Priority**: High - Enables user-facing workflows  
**Dependencies**: MCP tools (complete), Database (existing)
