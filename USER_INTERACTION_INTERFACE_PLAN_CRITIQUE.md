# Critique: User Interaction Interface Plan

**Date**: 2025-12-08  
**Reviewer**: Architecture Review (CLAUDE.md standards)  
**Document**: USER_INTERACTION_INTERFACE_PLAN.md  
**Status**: ⚠️ Needs Revision

---

## Executive Summary

The plan has **strong vision** but **violates several critical CLAUDE.md patterns**. Primary issues:

1. ❌ **Wrong crate location** - `botticelli_interface` already exists for LLM traits
2. ❌ **Trait naming collision** - Name conflicts with existing patterns
3. ❌ **Missing error strategy** - No `derive_more` error types defined
4. ❌ **No instrumentation** - Missing `#[instrument]` in trait methods
5. ❌ **Unclear module organization** - No `lib.rs` structure defined
6. ⚠️ **Builder pattern unclear** - Struct construction not specified

---

## Critical Issues (Must Fix)

### Issue 1: Crate Name Collision

**Problem:**
```rust
// Plan suggests: crates/botticelli_interface/src/interaction.rs
// But botticelli_interface already exists for LLM API traits!
```

**Current `botticelli_interface`:**
- Defines LLM capability traits (BotticelliDriver, Streaming, Vision, etc.)
- Contains bot_server traits (BotActor, BotServer)
- Contains narrative execution traits

**Violation:** Re-using an existing crate name for a completely different purpose breaks workspace organization.

**Fix Required:**
```
Create new crate: crates/botticelli_interaction/
               or: crates/botticelli_ui/
               or: crates/botticelli_chat/
```

Keep `botticelli_interface` for LLM/API traits only.

---

### Issue 2: Trait Naming Violations

**Problem:**
```rust
// Plan uses:
pub trait UserInterface { ... }
pub trait InteractionHandler { ... }
pub trait SessionManager { ... }
```

**Violations:**
1. **Generic names** - "UserInterface" is too vague
2. **Handler suffix** - Not consistent with codebase patterns
3. **Manager suffix** - Anti-pattern (everything is a "manager")

**Existing patterns in codebase:**
- `BotticelliDriver` - Describes what it drives
- `BotServer` - Describes what it serves
- `NarrativeExecution` - Describes what it executes
- `ContentRepository` - Describes what it repositories

**Fix Required:**
```rust
// Be specific about domain:
pub trait ChatInterface: Send + Sync { ... }
pub trait CommandExecutor: Send + Sync { ... }
pub trait ConversationState: Send + Sync { ... }

// Or even more specific:
pub trait TerminalInterface: Send + Sync { ... }
pub trait WebInterface: Send + Sync { ... }
pub trait CommandRouter: Send + Sync { ... }
```

---

### Issue 3: Missing Error Types

**Problem:** Plan shows no error handling design.

**CLAUDE.md Requirement:**
> Use `derive_more::Display` + `derive_more::Error` for all errors

**Plan has:**
```rust
async fn display_message(&mut self, message: Message) -> InterfaceResult<()>;
//                                                        ^^^^^^^^^^^^^^^^
// What is InterfaceResult? Where is InterfaceError defined?
```

**Fix Required:**
```rust
// Location: crates/botticelli_interaction/src/error.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum InteractionErrorKind {
    #[display("Failed to display message: {}", _0)]
    DisplayFailed(String),
    
    #[display("Invalid user input: {}", _0)]
    InvalidInput(String),
    
    #[display("Command execution failed: {}", _0)]
    CommandFailed(String),
    
    #[display("Session not found: {}", _0)]
    SessionNotFound(String),
}

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Interaction Error: {} at {}:{}", kind, file, line)]
pub struct InteractionError {
    pub kind: InteractionErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl InteractionError {
    #[track_caller]
    pub fn new(kind: InteractionErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

pub type InteractionResult<T> = Result<T, InteractionError>;
```

---

### Issue 4: Missing Instrumentation

**Problem:** No tracing/observability design.

**CLAUDE.md Requirement:**
> All public functions have `#[instrument]`

**Plan shows:**
```rust
pub trait ChatInterface: Send + Sync {
    async fn display_message(&mut self, message: Message) -> InteractionResult<()>;
    //                                                       ^^^^^^^^^^^^^^^^^^
    // Where is #[instrument]? How do we observe this?
}
```

**Challenge:** Traits can't have `#[instrument]` on signatures.

**Fix Required:**

**Option 1: Require implementations to instrument**
```rust
/// Core trait for chat-based user interaction.
/// 
/// # Implementation Requirements
/// 
/// All implementations **must** use `#[instrument]` on all methods.
pub trait ChatInterface: Send + Sync {
    /// Display a message to the user.
    /// 
    /// # Instrumentation
    /// 
    /// Implementations must use:
    /// ```ignore
    /// #[instrument(skip(self), fields(message_type))]
    /// ```
    async fn display_message(&mut self, message: Message) -> InteractionResult<()>;
}

// Implementation:
impl ChatInterface for TuiInterface {
    #[instrument(skip(self), fields(message_type = %message.kind()))]
    async fn display_message(&mut self, message: Message) -> InteractionResult<()> {
        debug!("Displaying message");
        // ...
    }
}
```

**Option 2: Concrete wrapper with instrumentation**
```rust
/// Instrumented wrapper for any ChatInterface implementation.
pub struct InstrumentedChat<T: ChatInterface> {
    inner: T,
}

impl<T: ChatInterface> InstrumentedChat<T> {
    #[instrument(skip(self, message), fields(message_type = %message.kind()))]
    pub async fn display_message(&mut self, message: Message) -> InteractionResult<()> {
        debug!("Displaying message");
        let result = self.inner.display_message(message).await;
        if let Err(e) = &result {
            error!(error = ?e, "Failed to display message");
        }
        result
    }
}
```

**Recommendation:** Use Option 1 (documented requirement) + provide testing utilities.

---

### Issue 5: Module Organization Unclear

**Problem:** No `lib.rs` structure specified.

**CLAUDE.md Requirement:**
> lib.rs: Only `mod` and `pub use` statements

**Plan shows files:**
- `interaction.rs`
- `handler.rs`
- `session.rs`
- `types.rs`
- `parser.rs`

**But doesn't show:**
- What goes in `lib.rs`?
- What gets exported at crate level?
- Module visibility (`pub mod` vs private `mod`)?

**Fix Required:**
```rust
// crates/botticelli_interaction/src/lib.rs
#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! User interaction traits and types for Botticelli.
//!
//! Provides trait-based abstraction for chat interfaces across
//! terminal (TUI), web (Leptos), and mobile platforms.

mod chat;
mod command;
mod error;
mod parser;
mod session;
mod types;

// Error types (always exported first)
pub use error::{InteractionError, InteractionErrorKind, InteractionResult};

// Core traits
pub use chat::ChatInterface;
pub use command::{CommandExecutor, CommandRouter};
pub use session::ConversationState;

// Message and command types
pub use types::{
    Action, Command, Message, Progress, Question, Response, 
    TaskResult, UserInput, DataView,
};

// Command variants
pub use command::{
    BotCommand, NarrativeCommand, SocialCommand, SystemCommand,
};

// Parser
pub use parser::{IntentParser, Intent};
```

---

### Issue 6: Builder Pattern Unclear

**Problem:** Types shown without construction strategy.

**CLAUDE.md Requirement:**
> Always use builders, never struct literals

**Plan shows:**
```rust
pub enum Message {
    System(String),
    Assistant(String),
    Error(String),
    // ...
}

pub struct Session {
    pub id: SessionId,
    pub user_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub context: ConversationContext,
    pub active_tasks: Vec<Task>,
}
```

**Questions:**
1. How do I construct a `Message`? Builder or direct enum variant?
2. How do I construct a `Session`? Builder required?
3. What about `ConversationContext`?

**Fix Required:**

**For enums (simple):** Direct construction OK
```rust
// Enums with data are constructed directly
let msg = Message::System("Hello".to_string());
let input = UserInput::Text("create narrative".to_string());
```

**For structs:** Always builders
```rust
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct Session {
    pub id: SessionId,
    
    #[builder(default)]
    pub user_id: Option<String>,
    
    #[builder(default = DateTime::now_utc())]
    pub started_at: DateTime<Utc>,
    
    #[builder(default)]
    pub context: ConversationContext,
    
    #[builder(default)]
    pub active_tasks: Vec<Task>,
}

// Usage:
let session = Session::builder()
    .id(SessionId::new())
    .user_id(Some("user123".to_string()))
    .build();
```

**Document this clearly in plan.**

---

## Design Issues (Should Fix)

### Issue 7: Over-Abstraction Risk

**Problem:** Three separate traits may be too much.

**Current plan:**
```rust
pub trait UserInterface { ... }       // Display/input
pub trait InteractionHandler { ... }  // Command processing
pub trait SessionManager { ... }      // State management
```

**Risk:** Most implementations need all three → code duplication.

**Alternative Design:**
```rust
/// Complete chat interface with all capabilities.
pub trait ChatInterface: Send + Sync {
    // Display
    async fn display_message(&mut self, message: Message) -> InteractionResult<()>;
    async fn get_input(&mut self) -> InteractionResult<UserInput>;
    
    // Command execution
    async fn execute_command(&mut self, command: Command) -> InteractionResult<Response>;
    
    // State
    fn conversation_state(&self) -> &ConversationState;
    fn conversation_state_mut(&mut self) -> &mut ConversationState;
}

// Separate trait ONLY if implementations commonly split:
pub trait ConversationPersistence: Send + Sync {
    async fn save_state(&mut self, state: &ConversationState) -> InteractionResult<()>;
    async fn load_state(&mut self, session_id: SessionId) -> InteractionResult<ConversationState>;
}
```

**Decision:** Start with one trait. Split only when clear need emerges.

---

### Issue 8: Async Everywhere

**Problem:** All trait methods are async.

**Question:** Do we need async for all operations?

**Examples:**
```rust
async fn display_message(&mut self, message: Message) -> InteractionResult<()>;
//    ^^^^^ Why async for in-memory display?

fn state(&self) -> &InterfaceState;
// This one is sync - good!
```

**Consideration:**
- **TUI display** - Sync rendering
- **Web display** - Could be sync (Leptos signals)
- **Database save** - Needs async
- **Command execution** - Needs async (calls MCP tools)

**Fix:**
```rust
pub trait ChatInterface: Send + Sync {
    /// Display a message (may be sync for some implementations).
    fn display_message(&mut self, message: Message) -> InteractionResult<()>;
    
    /// Get input (async - may wait for user).
    async fn get_input(&mut self) -> InteractionResult<UserInput>;
    
    /// Execute command (async - invokes tools).
    async fn execute_command(&mut self, command: Command) -> InteractionResult<Response>;
}
```

**Or use blanket impl:**
```rust
// Sync version
pub trait ChatInterfaceSync: Send + Sync {
    fn display_message(&mut self, message: Message) -> InteractionResult<()>;
}

// Async version automatically available
#[async_trait]
impl<T: ChatInterfaceSync> ChatInterface for T {
    async fn display_message(&mut self, message: Message) -> InteractionResult<()> {
        self.display_message(message)
    }
}
```

---

### Issue 9: Testing Not Specified

**Problem:** Plan says "Unit tests for trait implementations" but doesn't show how.

**CLAUDE.md Requirement:**
> All tests go in `tests/` directory

**Questions:**
1. How do you test a trait?
2. What's the test structure?
3. What are the fixtures?

**Fix Required:**
```
crates/botticelli_interaction/tests/
├── chat_interface_test.rs       # Test trait contract
├── command_executor_test.rs     # Test command routing
├── parser_test.rs               # Test intent parsing
└── fixtures/
    └── mock_interface.rs        # Mock implementation for testing
```

**Example test structure:**
```rust
// tests/chat_interface_test.rs

use botticelli_interaction::{ChatInterface, Message, UserInput, InteractionResult};

/// Mock implementation for testing.
struct MockChatInterface {
    messages: Vec<Message>,
    inputs: Vec<UserInput>,
}

impl ChatInterface for MockChatInterface {
    fn display_message(&mut self, message: Message) -> InteractionResult<()> {
        self.messages.push(message);
        Ok(())
    }
    
    async fn get_input(&mut self) -> InteractionResult<UserInput> {
        self.inputs.pop()
            .ok_or_else(|| InteractionError::new(
                InteractionErrorKind::InvalidInput("No input available".to_string())
            ))
    }
}

#[tokio::test]
async fn test_display_system_message() {
    let mut interface = MockChatInterface::default();
    let msg = Message::System("Test".to_string());
    
    interface.display_message(msg.clone()).expect("Should display");
    
    assert_eq!(interface.messages.len(), 1);
    assert!(matches!(interface.messages[0], Message::System(_)));
}
```

**Document this pattern in plan.**

---

### Issue 10: Integration Strategy Vague

**Problem:** Shows using MCP tools but not how.

**Plan shows:**
```rust
impl InteractionHandler {
    async fn handle_narrative_creation(&mut self, input: UserInput) -> InterfaceResult<Response> {
        let tool = CreateNarrativeTool;
        let params = json!({ ... });
        let result = tool.execute(params).await?;
        Ok(Response::Success(format_narrative_result(&result)))
    }
}
```

**Issues:**
1. **No error conversion** - `tool.execute()` returns `ToolResult`, not `InteractionResult`
2. **No instrumentation** - Missing `#[instrument]`
3. **JSON construction** - How is `extract_description()` implemented?
4. **Response formatting** - What is `format_narrative_result()`?

**Fix Required:**
```rust
impl CommandExecutor for DefaultCommandExecutor {
    #[instrument(skip(self), fields(command_type = "narrative_create"))]
    async fn execute_narrative_command(
        &mut self,
        cmd: NarrativeCommand,
    ) -> InteractionResult<Response> {
        match cmd {
            NarrativeCommand::Create { description, name } => {
                debug!(name = %name, "Creating narrative");
                
                let tool = CreateNarrativeTool;
                let params = json!({
                    "description": description,
                    "name": name,
                });
                
                // Convert tool error to interaction error
                let result = tool.execute(params).await
                    .map_err(|e| InteractionError::new(
                        InteractionErrorKind::CommandFailed(format!("Tool execution failed: {}", e))
                    ))?;
                
                // Extract key info from result
                let toml = result.get("toml")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| InteractionError::new(
                        InteractionErrorKind::CommandFailed("No TOML in result".to_string())
                    ))?;
                
                let validation = result.get("validation")
                    .ok_or_else(|| InteractionError::new(
                        InteractionErrorKind::CommandFailed("No validation in result".to_string())
                    ))?;
                
                // Format user-friendly response
                let response = if validation.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
                    format!("✓ Created narrative '{}'\n\nPreview:\n{}", name, toml)
                } else {
                    format!("⚠ Created narrative '{}' but there are validation warnings", name)
                };
                
                Ok(Response::Continue(Message::Success(response)))
            }
        }
    }
}
```

**Document this error conversion pattern.**

---

## Recommendations

### Immediate Actions

1. **Rename crate** → `botticelli_interaction` (not `botticelli_interface`)
2. **Define error types** → Add complete error.rs with derive_more
3. **Clarify trait names** → More specific, domain-focused names
4. **Add instrumentation requirements** → Document in trait docs
5. **Define lib.rs structure** → Show exact module organization
6. **Specify builder patterns** → Which types need builders?

### Architecture Decisions Needed

1. **One trait or three?** → Start with one `ChatInterface`, split later if needed
2. **Sync or async?** → Make display sync, execution async
3. **Error conversion** → Define pattern for tool errors → interaction errors
4. **Testing strategy** → Document mock implementations and test structure

### Phase 1 Revised Deliverables

```
crates/botticelli_interaction/
├── src/
│   ├── lib.rs                 # mod + pub use only
│   ├── error.rs               # InteractionError with derive_more
│   ├── chat.rs                # ChatInterface trait
│   ├── command.rs             # Command types and execution
│   ├── types.rs               # Message, Response, etc.
│   ├── parser.rs              # Intent parsing
│   └── state.rs               # ConversationState
├── tests/
│   ├── chat_interface_test.rs
│   ├── command_test.rs
│   ├── parser_test.rs
│   └── fixtures/
│       └── mock_chat.rs
└── Cargo.toml
```

### Documentation Additions

Add to plan:
1. **Error Handling Section** - Complete error type definitions
2. **Instrumentation Section** - Requirements for implementations
3. **Testing Section** - Mock implementation pattern
4. **Integration Section** - Error conversion from MCP tools
5. **Module Organization Section** - Exact lib.rs structure

---

## Summary

**Strong Points:**
- ✅ Clear vision and problem statement
- ✅ Good workflow examples
- ✅ Phased approach (TUI → Web → Mobile)
- ✅ Integration with existing MCP tools

**Critical Gaps:**
- ❌ Wrong crate name (collision with existing)
- ❌ No error handling design
- ❌ No instrumentation strategy
- ❌ Missing module organization
- ❌ Vague builder pattern usage

**Verdict:** Revise plan to address CLAUDE.md compliance before Phase 1 implementation.

---

## Revised Timeline

**Week 0 (Now):**
- Fix planning document
- Define error types
- Clarify trait structure
- Document test strategy

**Week 1 (Phase 1):**
- Create `botticelli_interaction` crate
- Implement traits and types
- Write tests
- Verify against CLAUDE.md

**Weeks 2-4:** Proceed with original Phase 2-4 plan

---

**Status**: Plan needs revision before implementation can begin.  
**Priority**: Fix critical issues (crate name, errors, instrumentation) first.  
**Blocker**: Cannot proceed until crate naming conflict resolved.
