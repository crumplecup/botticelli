# Botticelli Interaction Interface - Implementation Plan

**Status**: Ready for Implementation  
**Created**: 2025-12-08  
**Updated**: 2025-12-08 17:40 UTC  
**Crate**: `botticelli_chat`  
**Goal**: Trait-based chat interface for TUI, web, and mobile platforms

---

## Quick Navigation

**Status Tracker**: [Current Phase](#current-phase) | [All Steps](#implementation-steps)

### Implementation Steps

1. [✅ Step 1: Project Setup](#step-1-project-setup)
2. [⏭️ Step 2: Error Types](#step-2-error-types)
3. [⏭️ Step 3: Core Types](#step-3-core-types)
4. [⏭️ Step 4: ChatInterface Trait](#step-4-chatinterface-trait)
5. [⏭️ Step 5: Command Types](#step-5-command-types)
6. [⏭️ Step 6: Intent Parser](#step-6-intent-parser)
7. [⏭️ Step 7: State Management](#step-7-state-management)
8. [⏭️ Step 8: Tests](#step-8-tests)
9. [⏭️ Step 9: TUI Implementation](#step-9-tui-implementation)
10. [⏭️ Step 10: Integration](#step-10-integration)
11. [✅ Step 11: Container Infrastructure](#step-11-container-infrastructure)

---

## Current Phase

**Active Step**: Step 1 - Project Setup  
**Blocked By**: None  
**Next Action**: Create crate structure

---

## Vision

### Problem Statement

Users need an interactive way to:
- Create narratives through conversation
- Assign narratives to bots
- Schedule social media posts
- Monitor bot activities

Currently requires direct TOML editing or external MCP clients.

### Solution

A **trait-based chat interface** that:
- Abstracts UI implementation (TUI/web/mobile)
- Provides conversational workflows
- Integrates with existing MCP tools
- Maintains session state

### Key Design Decisions

1. **Crate Name**: `botticelli_chat` (avoids collision with `botticelli_interface`)
2. **Trait Strategy**: One primary trait (`ChatInterface`), split only if needed
3. **Error Handling**: `derive_more::Display` + `derive_more::Error`
4. **Instrumentation**: Required in implementations, documented in trait docs
5. **Builders**: Structs use `typed_builder`, enums use direct construction

---

## Architecture Overview

```
botticelli_chat/
├── src/
│   ├── lib.rs              # mod + pub use only
│   ├── error.rs            # ChatError with derive_more
│   ├── chat.rs             # ChatInterface trait
│   ├── command.rs          # Command types and execution
│   ├── message.rs          # Message and Response types
│   ├── input.rs            # UserInput types
│   ├── parser.rs           # Intent parsing
│   └── state.rs            # ConversationState
└── tests/
    ├── chat_test.rs
    ├── command_test.rs
    ├── parser_test.rs
    └── fixtures/
        └── mock_chat.rs    # Mock implementation
```

### Core Trait

```rust
/// Chat interface for conversational user interaction.
/// 
/// # Implementation Requirements
/// 
/// All implementations **must** use `#[instrument]` on all methods
/// for observability.
pub trait ChatInterface: Send + Sync {
    /// Display a message to the user.
    fn display_message(&mut self, message: Message) -> ChatResult<()>;
    
    /// Get input from the user (async - waits for user).
    async fn get_input(&mut self) -> ChatResult<UserInput>;
    
    /// Execute a command (async - calls MCP tools).
    async fn execute_command(&mut self, command: Command) -> ChatResult<Response>;
    
    /// Get conversation state.
    fn state(&self) -> &ConversationState;
    
    /// Get mutable conversation state.
    fn state_mut(&mut self) -> &mut ConversationState;
}
```

---

## Implementation Steps

### Step 1: Project Setup

**Goal**: Create crate structure and dependencies

**Actions**:
1. Create `crates/botticelli_chat/` directory
2. Create `Cargo.toml` with dependencies
3. Create `src/lib.rs` skeleton
4. Add to workspace `Cargo.toml`
5. Run `cargo check -p botticelli_chat`

**Files Created**:
- `crates/botticelli_chat/Cargo.toml`
- `crates/botticelli_chat/src/lib.rs`

**Dependencies**:
```toml
[dependencies]
derive_more = { version = "1.0", features = ["display", "error", "from"] }
typed-builder = "0.20"
derive-getters = "0.5"
tokio = { version = "1.41", features = ["full"] }
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Success Criteria**:
- [ ] Crate compiles
- [ ] Added to workspace
- [ ] No warnings

**Time Estimate**: 15 minutes

---

### Step 2: Error Types

**Goal**: Define error types following CLAUDE.md patterns

**Actions**:
1. Create `src/error.rs`
2. Define `ChatErrorKind` enum
3. Define `ChatError` wrapper
4. Define `ChatResult<T>` type alias
5. Export from `lib.rs`
6. Run `cargo check -p botticelli_chat`

**Code Template**:

```rust
// src/error.rs

use derive_more::{Display, Error};

/// Specific error conditions for interaction operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum ChatErrorKind {
    #[display("Failed to display message: {}", _0)]
    DisplayFailed(String),
    
    #[display("Invalid user input: {}", _0)]
    InvalidInput(String),
    
    #[display("Command execution failed: {}", _0)]
    CommandFailed(String),
    
    #[display("Session not found: {}", _0)]
    SessionNotFound(String),
    
    #[display("Parse error: {}", _0)]
    ParseError(String),
    
    #[display("State error: {}", _0)]
    StateError(String),
}

/// Interaction error with location tracking.
#[derive(Debug, Clone, Display, Error)]
#[display("Interaction: {} at {}:{}", kind, file, line)]
pub struct ChatError {
    /// Error kind
    pub kind: ChatErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: &'static str,
}

impl ChatError {
    /// Create a new error with caller location.
    #[track_caller]
    pub fn new(kind: ChatErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

/// Result type for interaction operations.
pub type ChatResult<T> = Result<T, ChatError>;
```

**lib.rs Updates**:
```rust
mod error;

pub use error::{ChatError, ChatErrorKind, ChatResult};
```

**Success Criteria**:
- [ ] All error variants compile
- [ ] `#[track_caller]` on constructor
- [ ] `derive_more` derives used
- [ ] No manual `impl Display` or `impl Error`
- [ ] Exported from crate root

**Time Estimate**: 20 minutes

---

### Step 3: Core Types

**Goal**: Define message and input types

**Actions**:
1. Create `src/message.rs`
2. Create `src/input.rs`
3. Define all type variants
4. Export from `lib.rs`
5. Run `cargo check -p botticelli_chat`

**Code Templates**:

```rust
// src/message.rs

/// A message displayed to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// System notification
    System(String),
    
    /// Assistant response
    Assistant(String),
    
    /// User message (echoed back)
    User(String),
    
    /// Error notification
    Error(String),
    
    /// Success confirmation
    Success(String),
    
    /// Warning
    Warning(String),
}

impl Message {
    /// Get message kind as string.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::System(_) => "system",
            Self::Assistant(_) => "assistant",
            Self::User(_) => "user",
            Self::Error(_) => "error",
            Self::Success(_) => "success",
            Self::Warning(_) => "warning",
        }
    }
    
    /// Get message content.
    pub fn content(&self) -> &str {
        match self {
            Self::System(s) | Self::Assistant(s) | Self::User(s) 
            | Self::Error(s) | Self::Success(s) | Self::Warning(s) => s,
        }
    }
}

/// Response from command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Continue conversation with message
    Continue(Message),
    
    /// Task completed successfully
    Complete(String),
    
    /// Request clarification
    Clarify(String),
    
    /// Show error
    Error(String),
}
```

```rust
// src/input.rs

use std::path::PathBuf;

/// User input types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserInput {
    /// Text command or message
    Text(String),
    
    /// Selection from menu
    Selection(usize),
    
    /// File path
    File(PathBuf),
    
    /// Confirmation
    Confirm(bool),
    
    /// Cancel operation
    Cancel,
}

impl UserInput {
    /// Get input kind as string.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Selection(_) => "selection",
            Self::File(_) => "file",
            Self::Confirm(_) => "confirm",
            Self::Cancel => "cancel",
        }
    }
}
```

**lib.rs Updates**:
```rust
mod input;
mod message;

pub use input::UserInput;
pub use message::{Message, Response};
```

**Success Criteria**:
- [ ] All enum variants compile
- [ ] Helper methods implemented
- [ ] No warnings
- [ ] Exported from crate root

**Time Estimate**: 20 minutes

---

### Step 4: ChatInterface Trait

**Goal**: Define core trait with documentation

**Actions**:
1. Create `src/chat.rs`
2. Define `ChatInterface` trait
3. Document instrumentation requirements
4. Export from `lib.rs`
5. Run `cargo check -p botticelli_chat`

**Code Template**:

```rust
// src/chat.rs

use crate::{ConversationState, ChatResult, Message, Response, UserInput, Command};

/// Chat interface for conversational user interaction.
/// 
/// Provides a trait-based abstraction for implementing chat UIs
/// across different platforms (terminal, web, mobile).
/// 
/// # Implementation Requirements
/// 
/// ## Instrumentation
/// 
/// All implementations **must** use `#[instrument]` on all trait methods
/// for observability and debugging. Example:
/// 
/// ```ignore
/// #[instrument(skip(self), fields(message_type = %message.kind()))]
/// fn display_message(&mut self, message: Message) -> ChatResult<()> {
///     debug!("Displaying message");
///     // implementation
/// }
/// ```
/// 
/// ## Error Handling
/// 
/// Methods return `ChatResult` which includes location tracking.
/// Use `ChatError::new()` to preserve caller location.
/// 
/// # Example Implementation
/// 
/// ```ignore
/// use botticelli_chat::{ChatInterface, Message, UserInput, Response, Command};
/// 
/// struct MyChatInterface {
///     state: ConversationState,
/// }
/// 
/// impl ChatInterface for MyChatInterface {
///     #[instrument(skip(self))]
///     fn display_message(&mut self, message: Message) -> ChatResult<()> {
///         println!("{}: {}", message.kind(), message.content());
///         Ok(())
///     }
///     
///     // ... other methods
/// }
/// ```
pub trait ChatInterface: Send + Sync {
    /// Display a message to the user.
    /// 
    /// This is synchronous as most UI frameworks can display immediately.
    fn display_message(&mut self, message: Message) -> ChatResult<()>;
    
    /// Get input from the user.
    /// 
    /// This is async as it may wait for user interaction.
    async fn get_input(&mut self) -> ChatResult<UserInput>;
    
    /// Execute a command.
    /// 
    /// This is async as it may invoke MCP tools or other async operations.
    async fn execute_command(&mut self, command: Command) -> ChatResult<Response>;
    
    /// Get conversation state.
    fn state(&self) -> &ConversationState;
    
    /// Get mutable conversation state.
    fn state_mut(&mut self) -> &mut ConversationState;
}
```

**lib.rs Updates**:
```rust
mod chat;

pub use chat::ChatInterface;
```

**Success Criteria**:
- [ ] Trait compiles
- [ ] Documentation complete
- [ ] Instrumentation requirements documented
- [ ] Example provided
- [ ] Exported from crate root

**Time Estimate**: 30 minutes

---

### Step 5: Command Types

**Goal**: Define command enum and variants

**Actions**:
1. Create `src/command.rs`
2. Define `Command` enum
3. Define command variant types
4. Export from `lib.rs`
5. Run `cargo check -p botticelli_chat`

**Code Template**:

```rust
// src/command.rs

/// Commands the system can execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Narrative operations
    Narrative(NarrativeCommand),
    
    /// Bot operations
    Bot(BotCommand),
    
    /// System operations
    System(SystemCommand),
}

/// Narrative-related commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrativeCommand {
    /// Create a new narrative
    Create {
        /// Narrative name
        name: String,
        /// Natural language description
        description: String,
    },
    
    /// Load existing narrative
    Load {
        /// Path to narrative file
        path: String,
    },
    
    /// Update narrative field
    Update {
        /// Narrative name
        name: String,
        /// Field to update
        field: String,
        /// New value
        value: String,
    },
    
    /// Validate narrative
    Validate {
        /// Narrative name
        name: String,
    },
    
    /// Save narrative
    Save {
        /// Narrative name
        name: String,
        /// Optional custom path
        path: Option<String>,
    },
}

/// Bot-related commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand {
    /// List available bots
    List,
    
    /// Get bot status
    Status {
        /// Bot name
        name: String,
    },
    
    /// Assign narrative to bot
    Assign {
        /// Bot name
        bot_name: String,
        /// Narrative name
        narrative_name: String,
    },
    
    /// Start bot
    Start {
        /// Bot name
        name: String,
    },
    
    /// Stop bot
    Stop {
        /// Bot name
        name: String,
    },
}

/// System-related commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemCommand {
    /// Show help
    Help,
    
    /// Clear conversation
    Clear,
    
    /// Exit interface
    Exit,
    
    /// Show version
    Version,
}
```

**lib.rs Updates**:
```rust
mod command;

pub use command::{BotCommand, Command, NarrativeCommand, SystemCommand};
```

**Success Criteria**:
- [ ] All command variants compile
- [ ] Documentation on each variant
- [ ] No warnings
- [ ] Exported from crate root

**Time Estimate**: 25 minutes

---

### Step 6: Intent Parser

**Goal**: Parse user input into commands

**Actions**:
1. Create `src/parser.rs`
2. Define `Intent` enum
3. Implement basic parsing logic
4. Export from `lib.rs`
5. Run `cargo check -p botticelli_chat`

**Code Template**:

```rust
// src/parser.rs

use crate::{Command, ChatError, ChatErrorKind, ChatResult};
use crate::{NarrativeCommand, BotCommand, SystemCommand};

/// User intent parsed from input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Create narrative
    CreateNarrative,
    
    /// Load narrative
    LoadNarrative,
    
    /// Update narrative
    UpdateNarrative,
    
    /// Assign bot
    AssignBot,
    
    /// Get status
    Status,
    
    /// Help request
    Help,
    
    /// Exit
    Exit,
    
    /// Unknown/ambiguous
    Unknown,
}

/// Parse user input text into intent.
pub struct IntentParser;

impl IntentParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self
    }
    
    /// Parse text into intent.
    pub fn parse_intent(&self, text: &str) -> Intent {
        let lower = text.to_lowercase();
        
        // Check for command prefixes
        if lower.starts_with("/help") || lower.starts_with("help") {
            return Intent::Help;
        }
        
        if lower.starts_with("/exit") || lower.starts_with("exit") || lower.starts_with("quit") {
            return Intent::Exit;
        }
        
        // Check for narrative operations
        if lower.contains("create") && lower.contains("narrative") {
            return Intent::CreateNarrative;
        }
        
        if lower.contains("load") && lower.contains("narrative") {
            return Intent::LoadNarrative;
        }
        
        if lower.contains("update") && lower.contains("narrative") {
            return Intent::UpdateNarrative;
        }
        
        // Check for bot operations
        if lower.contains("assign") {
            return Intent::AssignBot;
        }
        
        if lower.contains("status") {
            return Intent::Status;
        }
        
        Intent::Unknown
    }
    
    /// Parse text into command (with extracted parameters).
    /// 
    /// This is a simple implementation. A full implementation would
    /// use NLP or structured parsing to extract parameters.
    pub fn parse_command(&self, text: &str) -> ChatResult<Command> {
        let intent = self.parse_intent(text);
        
        match intent {
            Intent::Help => Ok(Command::System(SystemCommand::Help)),
            Intent::Exit => Ok(Command::System(SystemCommand::Exit)),
            Intent::CreateNarrative => {
                // Simple extraction: look for quoted name
                // Full implementation would be more sophisticated
                Err(ChatError::new(
                    ChatErrorKind::ParseError(
                        "Command parsing needs more context".to_string()
                    )
                ))
            }
            _ => Err(ChatError::new(
                ChatErrorKind::ParseError(
                    format!("Cannot parse intent: {:?}", intent)
                )
            )),
        }
    }
}

impl Default for IntentParser {
    fn default() -> Self {
        Self::new()
    }
}
```

**lib.rs Updates**:
```rust
mod parser;

pub use parser::{Intent, IntentParser};
```

**Success Criteria**:
- [ ] Parser compiles
- [ ] Basic intent recognition works
- [ ] Returns `ChatResult`
- [ ] Documented as simple implementation
- [ ] Exported from crate root

**Time Estimate**: 30 minutes

---

### Step 7: State Management

**Goal**: Define conversation state

**Actions**:
1. Create `src/state.rs`
2. Define `ConversationState` struct
3. Use `typed_builder` for construction
4. Export from `lib.rs`
5. Run `cargo check -p botticelli_chat`

**Code Template**:

```rust
// src/state.rs

use chrono::{DateTime, Utc};
use typed_builder::TypedBuilder;
use crate::{Message, UserInput};

/// Conversation state tracking.
#[derive(Debug, Clone, TypedBuilder)]
pub struct ConversationState {
    /// Session ID
    pub session_id: String,
    
    /// Session start time
    #[builder(default = Utc::now())]
    pub started_at: DateTime<Utc>,
    
    /// Conversation history
    #[builder(default)]
    pub history: Vec<Interaction>,
    
    /// Active narrative (if any)
    #[builder(default)]
    pub active_narrative: Option<String>,
    
    /// Active bot (if any)
    #[builder(default)]
    pub active_bot: Option<String>,
}

impl ConversationState {
    /// Add interaction to history.
    pub fn add_interaction(&mut self, interaction: Interaction) {
        self.history.push(interaction);
    }
    
    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
    
    /// Get last N interactions.
    pub fn recent_interactions(&self, n: usize) -> &[Interaction] {
        let start = self.history.len().saturating_sub(n);
        &self.history[start..]
    }
}

/// A single interaction in conversation.
#[derive(Debug, Clone, TypedBuilder)]
pub struct Interaction {
    /// Timestamp
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,
    
    /// User input
    pub input: UserInput,
    
    /// System response
    pub response: Message,
}
```

**lib.rs Updates**:
```rust
mod state;

pub use state::{ConversationState, Interaction};
```

**Success Criteria**:
- [ ] State struct compiles
- [ ] Uses `typed_builder`
- [ ] Helper methods implemented
- [ ] Documentation complete
- [ ] Exported from crate root

**Time Estimate**: 25 minutes

---

### Step 8: Tests

**Goal**: Create test infrastructure

**Actions**:
1. Create `tests/fixtures/mock_chat.rs`
2. Create `tests/chat_test.rs`
3. Create `tests/command_test.rs`
4. Create `tests/parser_test.rs`
5. Run `cargo test -p botticelli_chat`

**Code Templates**:

```rust
// tests/fixtures/mock_chat.rs

use botticelli_chat::{
    ChatInterface, Command, ConversationState, ChatResult,
    Message, Response, UserInput,
};

/// Mock chat interface for testing.
pub struct MockChatInterface {
    pub state: ConversationState,
    pub messages: Vec<Message>,
    pub inputs: Vec<UserInput>,
}

impl MockChatInterface {
    /// Create a new mock interface.
    pub fn new(session_id: String) -> Self {
        Self {
            state: ConversationState::builder()
                .session_id(session_id)
                .build(),
            messages: Vec::new(),
            inputs: Vec::new(),
        }
    }
    
    /// Add input to be returned by get_input.
    pub fn add_input(&mut self, input: UserInput) {
        self.inputs.push(input);
    }
}

impl ChatInterface for MockChatInterface {
    fn display_message(&mut self, message: Message) -> ChatResult<()> {
        self.messages.push(message);
        Ok(())
    }
    
    async fn get_input(&mut self) -> ChatResult<UserInput> {
        self.inputs.pop()
            .ok_or_else(|| botticelli_chat::ChatError::new(
                botticelli_chat::ChatErrorKind::InvalidInput(
                    "No input available".to_string()
                )
            ))
    }
    
    async fn execute_command(&mut self, _command: Command) -> ChatResult<Response> {
        Ok(Response::Complete("Mock execution".to_string()))
    }
    
    fn state(&self) -> &ConversationState {
        &self.state
    }
    
    fn state_mut(&mut self) -> &mut ConversationState {
        &mut self.state
    }
}
```

```rust
// tests/chat_test.rs

use botticelli_chat::{ChatInterface, Message};

mod fixtures;
use fixtures::mock_chat::MockChatInterface;

#[tokio::test]
async fn test_display_system_message() {
    let mut chat = MockChatInterface::new("test-session".to_string());
    let msg = Message::System("Test message".to_string());
    
    chat.display_message(msg.clone()).expect("Should display");
    
    assert_eq!(chat.messages.len(), 1);
    assert_eq!(chat.messages[0], msg);
}

#[tokio::test]
async fn test_display_multiple_messages() {
    let mut chat = MockChatInterface::new("test-session".to_string());
    
    chat.display_message(Message::System("First".to_string())).unwrap();
    chat.display_message(Message::Assistant("Second".to_string())).unwrap();
    chat.display_message(Message::User("Third".to_string())).unwrap();
    
    assert_eq!(chat.messages.len(), 3);
}

#[tokio::test]
async fn test_get_input() {
    let mut chat = MockChatInterface::new("test-session".to_string());
    chat.add_input(botticelli_chat::UserInput::Text("hello".to_string()));
    
    let input = chat.get_input().await.expect("Should get input");
    
    assert!(matches!(input, botticelli_chat::UserInput::Text(_)));
}
```

```rust
// tests/command_test.rs

use botticelli_chat::{Command, NarrativeCommand, SystemCommand};

#[test]
fn test_narrative_create_command() {
    let cmd = Command::Narrative(NarrativeCommand::Create {
        name: "test".to_string(),
        description: "Test narrative".to_string(),
    });
    
    assert!(matches!(cmd, Command::Narrative(_)));
}

#[test]
fn test_system_help_command() {
    let cmd = Command::System(SystemCommand::Help);
    
    assert!(matches!(cmd, Command::System(SystemCommand::Help)));
}
```

```rust
// tests/parser_test.rs

use botticelli_chat::{Intent, IntentParser};

#[test]
fn test_parse_help_intent() {
    let parser = IntentParser::new();
    
    let intent = parser.parse_intent("help");
    assert_eq!(intent, Intent::Help);
    
    let intent = parser.parse_intent("/help");
    assert_eq!(intent, Intent::Help);
}

#[test]
fn test_parse_create_narrative_intent() {
    let parser = IntentParser::new();
    
    let intent = parser.parse_intent("create a narrative");
    assert_eq!(intent, Intent::CreateNarrative);
    
    let intent = parser.parse_intent("I want to create a narrative");
    assert_eq!(intent, Intent::CreateNarrative);
}

#[test]
fn test_parse_unknown_intent() {
    let parser = IntentParser::new();
    
    let intent = parser.parse_intent("random text");
    assert_eq!(intent, Intent::Unknown);
}
```

**Success Criteria**:
- [ ] All tests compile
- [ ] Mock implementation works
- [ ] Tests pass
- [ ] No warnings

**Time Estimate**: 45 minutes

---

### Step 9: TUI Implementation

**Goal**: Implement ChatInterface for TUI

**Location**: `crates/botticelli_tui/`

**Actions**:
1. Add `botticelli_chat` dependency to `botticelli_tui`
2. Create `src/chat_interface.rs`
3. Implement `ChatInterface` trait
4. Add instrumentation
5. Run `cargo test -p botticelli_tui`

**Code Template**:

```rust
// crates/botticelli_tui/src/chat_interface.rs

use botticelli_chat::{
    ChatInterface, Command, ConversationState, ChatError,
    ChatErrorKind, ChatResult, Message, Response, UserInput,
};
use tracing::{debug, error, instrument};

/// TUI implementation of ChatInterface.
pub struct TuiChatInterface {
    state: ConversationState,
    message_buffer: Vec<Message>,
}

impl TuiChatInterface {
    /// Create a new TUI chat interface.
    pub fn new(session_id: String) -> Self {
        Self {
            state: ConversationState::builder()
                .session_id(session_id)
                .build(),
            message_buffer: Vec::new(),
        }
    }
    
    /// Get message buffer for rendering.
    pub fn messages(&self) -> &[Message] {
        &self.message_buffer
    }
}

impl ChatInterface for TuiChatInterface {
    #[instrument(skip(self), fields(message_type = %message.kind()))]
    fn display_message(&mut self, message: Message) -> ChatResult<()> {
        debug!(content = %message.content(), "Displaying message");
        self.message_buffer.push(message);
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn get_input(&mut self) -> ChatResult<UserInput> {
        // TODO: Implement actual input reading from TUI
        // For now, return error
        Err(ChatError::new(
            ChatErrorKind::InvalidInput("Not implemented".to_string())
        ))
    }
    
    #[instrument(skip(self, command))]
    async fn execute_command(&mut self, command: Command) -> ChatResult<Response> {
        debug!(command = ?command, "Executing command");
        
        // TODO: Integrate with MCP tools
        // For now, return simple response
        Ok(Response::Complete("Command executed".to_string()))
    }
    
    fn state(&self) -> &ConversationState {
        &self.state
    }
    
    fn state_mut(&mut self) -> &mut ConversationState {
        &mut self.state
    }
}
```

**botticelli_tui lib.rs Updates**:
```rust
mod chat_interface;

pub use chat_interface::TuiChatInterface;
```

**Success Criteria**:
- [ ] Implementation compiles
- [ ] All trait methods instrumented
- [ ] Basic functionality works
- [ ] Tests pass

**Time Estimate**: 1 hour

---

### Step 10: Integration

**Goal**: Connect to MCP tools

**Actions**:
1. Add `botticelli_mcp` dependency
2. Implement MCP tool invocation in `execute_command`
3. Add error conversion from MCP errors
4. Add integration tests
5. Run full test suite

**Code Template**:

```rust
// In execute_command implementation

use botticelli_mcp::{CreateNarrativeTool, McpTool};

async fn execute_command(&mut self, command: Command) -> ChatResult<Response> {
    match command {
        Command::Narrative(NarrativeCommand::Create { name, description }) => {
            debug!(name = %name, "Creating narrative");
            
            let tool = CreateNarrativeTool;
            let params = serde_json::json!({
                "description": description,
                "name": name,
            });
            
            // Execute tool and convert error
            let result = tool.execute(params).await
                .map_err(|e| ChatError::new(
                    ChatErrorKind::CommandFailed(
                        format!("Failed to create narrative: {}", e)
                    )
                ))?;
            
            // Extract response data
            let toml = result.get("toml")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ChatError::new(
                    ChatErrorKind::CommandFailed(
                        "No TOML in response".to_string()
                    )
                ))?;
            
            let msg = format!("✓ Created narrative '{}'\n\n{}", name, toml);
            Ok(Response::Complete(msg))
        }
        
        // Other command handlers...
        _ => {
            Err(ChatError::new(
                ChatErrorKind::CommandFailed(
                    "Command not implemented".to_string()
                )
            ))
        }
    }
}
```

**Success Criteria**:
- [ ] MCP tools invoked correctly
- [ ] Error conversion works
- [ ] Integration tests pass
- [ ] Full workflow functional

**Time Estimate**: 1.5 hours

---

## Complete lib.rs Structure

```rust
// crates/botticelli_chat/src/lib.rs

#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! User interaction traits and types for Botticelli.
//!
//! Provides trait-based abstraction for chat interfaces across
//! terminal (TUI), web (Leptos), and mobile platforms.
//!
//! # Example
//!
//! ```ignore
//! use botticelli_chat::{ChatInterface, Message};
//!
//! async fn example(chat: &mut impl ChatInterface) {
//!     chat.display_message(Message::System("Hello!".to_string())).unwrap();
//!     let input = chat.get_input().await.unwrap();
//! }
//! ```

mod chat;
mod command;
mod error;
mod input;
mod message;
mod parser;
mod state;

// Export error types first
pub use error::{ChatError, ChatErrorKind, ChatResult};

// Core trait
pub use chat::ChatInterface;

// Command types
pub use command::{BotCommand, Command, NarrativeCommand, SystemCommand};

// Message and response types
pub use message::{Message, Response};

// Input types
pub use input::UserInput;

// Parser
pub use parser::{Intent, IntentParser};

// State management
pub use state::{ConversationState, Interaction};
```

---

## Testing Strategy

### Unit Tests (in `tests/`)

1. **chat_test.rs** - Test trait contract
2. **command_test.rs** - Test command construction
3. **parser_test.rs** - Test intent parsing
4. **state_test.rs** - Test state management

### Integration Tests

1. **tui_integration_test.rs** - TUI implementation
2. **mcp_integration_test.rs** - MCP tool invocation

### Test Coverage Goals

- [ ] All error variants triggered
- [ ] All command variants tested
- [ ] All intent parsing cases covered
- [ ] State transitions validated
- [ ] Mock implementation complete

---

## Validation Checklist

### CLAUDE.md Compliance

- [ ] No `#[cfg(test)]` in source files
- [ ] All errors use `derive_more::Display` + `derive_more::Error`
- [ ] All public functions documented
- [ ] Trait methods document instrumentation requirements
- [ ] Structs use `typed_builder`
- [ ] Enums use direct construction
- [ ] `lib.rs` only has `mod` and `pub use`
- [ ] All imports use `use crate::{Type}`
- [ ] No `#[allow]` attributes

### Build Verification

```bash
# After each step
cargo check -p botticelli_chat

# Before committing
just check botticelli_chat
just test-package botticelli_chat

# Final verification
just check-all botticelli_chat
```

---

## Timeline

| Step | Description | Time | Cumulative |
|------|-------------|------|------------|
| 1 | Project Setup | 15m | 15m |
| 2 | Error Types | 20m | 35m |
| 3 | Core Types | 20m | 55m |
| 4 | ChatInterface Trait | 30m | 1h 25m |
| 5 | Command Types | 25m | 1h 50m |
| 6 | Intent Parser | 30m | 2h 20m |
| 7 | State Management | 25m | 2h 45m |
| 8 | Tests | 45m | 3h 30m |
| 9 | TUI Implementation | 1h | 4h 30m |
| 10 | Integration | 1.5h | 6h |

**Total Estimated Time**: 6 hours (one full workday)

---

## Success Criteria

### Phase 1 Complete When:

- [ ] All 10 steps completed
- [ ] `cargo check -p botticelli_chat` passes
- [ ] `cargo test -p botticelli_chat` passes
- [ ] All CLAUDE.md compliance checks pass
- [ ] TUI implementation functional
- [ ] MCP integration working
- [ ] Documentation complete
- [ ] Zero warnings

### Ready for Phase 2 (Web) When:

- [ ] Phase 1 complete
- [ ] User workflow tested end-to-end
- [ ] Performance acceptable (< 500ms response)
- [ ] Error messages clear and helpful
- [ ] Code reviewed and approved

---

### Step 11: Container Infrastructure

**Goal**: Provide containerized deployment for chat interface

**Status**: ✅ Complete

**Actions Taken**:
1. Created `Containerfile.chat` - Multi-stage build for chat binary
2. Created `docker-compose.chat.yml` - Full stack orchestration
3. Updated `justfile` with chat commands

**Deliverables**:

**Container Files**:
- `Containerfile.chat` - Optimized multi-stage build with cargo-chef caching
- `docker-compose.chat.yml` - Stack includes:
  - PostgreSQL (port 5433)
  - MCP Server (internal)
  - Chat TUI (interactive)
  - Optional Jaeger (debug profile)

**Justfile Commands**:
- `just chat-build` - Build chat container
- `just chat-up` - Start complete stack (interactive)
- `just chat-up-bg` - Start in background
- `just chat-up-debug` - Start with Jaeger tracing
- `just chat-down` - Stop all services
- `just chat-logs [service]` - View logs
- `just chat-rebuild` - Rebuild and restart
- `just chat-local` - Run locally (requires separate postgres/mcp)
- `just chat-setup` - Complete setup from scratch

**Success Criteria**:
- [✅] Multi-stage Containerfile with dependency caching
- [✅] Docker Compose orchestration
- [✅] PostgreSQL with health checks
- [✅] MCP Server integration
- [✅] Justfile recipes for all operations
- [✅] Non-root container user (security)
- [✅] Volume mounts for narratives directory
- [✅] Environment variable configuration
- [✅] Optional Jaeger debugging support

**Time Taken**: 15 minutes

---

## Future Phases

### Phase 2: Web Interface (Leptos)

**Timeline**: 1-2 weeks

**Deliverables**:
- Leptos component implementing `ChatInterface`
- WebSocket server for real-time updates
- Responsive CSS styling
- Deployment configuration

### Phase 3: Advanced Features

**Timeline**: Ongoing

**Features**:
- Session persistence to database
- Multi-user support
- Advanced intent parsing (NLP)
- Workflow recording/replay
- Voice interface

---

## References

- [CLAUDE.md](./CLAUDE.md) - Project coding standards
- [USER_INTERACTION_INTERFACE_PLAN.md](./USER_INTERACTION_INTERFACE_PLAN.md) - Original vision
- [USER_INTERACTION_INTERFACE_PLAN_CRITIQUE.md](./USER_INTERACTION_INTERFACE_PLAN_CRITIQUE.md) - Critique and fixes
- [botticelli_interface](./crates/botticelli_interface/) - Existing LLM trait patterns
- [botticelli_mcp](./crates/botticelli_mcp/) - MCP tools to integrate

---

**Status**: Ready to begin Step 1  
**Next Action**: Create crate structure  
**Estimated Completion**: 6 hours from start
