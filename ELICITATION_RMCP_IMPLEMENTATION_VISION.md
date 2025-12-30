# Elicitation + RMCP: Implementation Vision and Roadmap

**Date:** 2025-12-30  
**Status:** Vision Document - Implementation Strategy  
**Cross-ref:** ELICITATION_RMCP_SYNERGY.md, RMCP_MIGRATION_VISION.md, RMCP_TOOL_MIGRATION_PLAN.md

## Executive Summary

This document presents a comprehensive vision for integrating the `elicitation` and `rmcp` ecosystems to create a uniquely powerful paradigm for MCP development in Rust. It provides both the strategic vision (why this matters) and concrete implementation steps (how to achieve it).

**Core Thesis:** The combination of type-safe input elicitation (elicitation) and type-safe tool registration (rmcp) creates a bidirectional type bridge that makes Rust the ideal language for building reliable, performant AI tooling.

## Table of Contents

1. [Vision: Why This Matters](#vision-why-this-matters)
2. [Current Architecture](#current-architecture)
3. [The Synergy](#the-synergy)
4. [Implementation Phases](#implementation-phases)
5. [Design Principles](#design-principles)
6. [Success Metrics](#success-metrics)
7. [Comparison with Alternatives](#comparison-with-alternatives)

---

## Vision: Why This Matters

### The Core Insight

**Elicitation** provides type-safe *input parsing* (LLM → Rust types)  
**RMCP** provides type-safe *tool registration* (Rust types → MCP schema)

Together, they create a **bidirectional type bridge**:

```
LLM Conversation ←→ JSON ←→ Rust Types ←→ Application Logic
                   ↑         ↑
              elicitation  rmcp
```

### Why Rust for MCP Development

Traditional MCP development (Python, TypeScript):
- Runtime schema validation
- String-based parameter parsing
- Manual type coercion
- Tests required to catch type mismatches
- Dynamic flexibility at the cost of reliability

Rust with elicitation + rmcp:
- **Schema generation from types** (rmcp derives)
- **Automatic parsing with validation** (elicitation traits)
- **Compile-time guarantees** (type system enforces correctness)
- **Tests verify behavior, not types**
- **Zero-cost abstractions** (no runtime overhead)

### Self-Documenting APIs

The same type declaration:

```rust
#[derive(Tool, Elicit, Builder, Getters, Serialize, Deserialize)]
#[tool(name = "search", description = "Search the database")]
pub struct SearchRequest {
    #[tool(description = "Search query")]
    #[prompt("What would you like to search for?")]
    query: String,
    
    #[tool(description = "Maximum results (1-100)")]
    #[prompt("How many results? (1-100)")]
    #[builder(default = "10")]
    limit: RangedInt<1, 100>,
    
    #[tool(description = "Filter by category")]
    #[prompt("Select a category:")]
    category: Option<Category>,
}
```

**Benefits from ONE type definition:**
1. ✅ Generates MCP tool schema (rmcp)
2. ✅ Implements conversational elicitation (elicitation)
3. ✅ Enforces validation rules (type system + RangedInt)
4. ✅ Documents the API (attributes)
5. ✅ Provides builder pattern (derive_builder)
6. ✅ Exposes getters (derive_getters)

**One source of truth, six benefits.**

### Token Economy

**Problem**: Token-burning debugging without type safety

```
User: "Search isn't working"
AI: "What parameters did you pass?"
User: [sends JSON]
AI: "The limit field should be a number, not a string"
User: [fixes JSON]
AI: "Now the category is invalid"
User: [fixes again]
```

Each round-trip costs tokens and time.

**Solution**: Compiler-enforced correctness

```rust
// If it compiles, the types are correct
let request = SearchRequest::elicit(client).await?;
// Type system guarantees:
//   - query is a String (not null, not a number)
//   - limit is in valid range (1-100)
//   - category is from enum or None
//   - all required fields present

execute_search(request).await?;
```

**Result**: Fewer tokens spent on type debugging, more on actual problem-solving.

### Performance Enables New Use Cases

Zero-cost abstractions make Rust MCP servers viable for:
- **High-throughput scenarios** (1000+ req/s)
- **Resource-constrained environments** (embedded, edge)
- **Real-time interactive systems** (streaming, low latency)
- **Embedded AI assistants** (desktop apps, CLI tools)

---

## Current Architecture

### Elicitation Crate (`/home/erik/repos/elicitation`)

**Purpose**: Type-safe value elicitation from LLM conversations

**Current Status** (v0.1.0):
- ✅ Core traits: `Elicitation`, `Prompt`, `Select`, `Affirm`, `Survey`
- ✅ Primitive implementations: `bool`, integers, floats, `String`, `Duration`, `PathBuf`, network types
- ✅ Container implementations: `Option<T>`, `Vec<T>`, `Result<T,E>`, `Box<T>`, `Arc<T>`, etc.
- ✅ Derive macro for enums (`#[derive(Elicit)]` → `Select` pattern)
- ✅ Uses pmcp for MCP client interaction
- 🚧 Struct derivation (`Survey` pattern) - Phase 5 planned

**Design Principles**:
1. **Trait-based**: Generic over transport via `pmcp::Client<T>`
2. **Paradigm-oriented**: Different interaction patterns for different types
   - `Select` - Choose from finite options (enums)
   - `Affirm` - Yes/no confirmation (booleans)
   - `Survey` - Multi-field elicitation (structs)
3. **Compositional**: Complex types built from primitive implementations
4. **Zero-cost**: All abstractions compile away

**Key Files**:
- `src/traits.rs` - Core `Elicitation` and `Prompt` traits
- `src/paradigm.rs` - Interaction patterns (`Select`, `Affirm`, `Survey`)
- `src/primitives/` - Built-in type implementations (integers, strings, etc.)
- `src/containers/` - Generic container implementations (Option, Vec, etc.)
- `src/mcp/tools.rs` - MCP tool parameter builders
- `crates/elicitation_derive/` - Proc macro for `#[derive(Elicit)]`

### Botticelli (`/home/erik/repos/botticelli`)

**Purpose**: MCP server implementation with narrative generation capabilities

**Current Status** (rmcp branch):
- ✅ rmcp dependencies added (0.12.0)
- ✅ Error types with `derive_more` integration
- ✅ Basic tool infrastructure (`EchoTool`, `ServerInfoTool`)
- ✅ Builder patterns for all types
- ✅ Full instrumentation with tracing
- ✅ Private fields with derive_getters
- 🚧 Migration from pmcp-based tools (in progress)
- 🚧 Database tools conversion
- 🚧 LLM integration tools conversion
- 🚧 Narrative tools conversion

**Design Principles** (per CLAUDE.md):
1. **rmcp-native**: Use `#[derive(Tool)]` for automatic schema generation
2. **Builder-centric**: All types use builders (derive_builder, derive_new)
3. **Private fields**: Use derive_getters/derive_setters for encapsulation
4. **Error propagation**: Unified error types with `derive_more::Display` + `derive_more::Error`
5. **Observable**: `#[instrument]` on all public functions
6. **No pub fields**: Always use getters/setters
7. **Test organization**: All tests in `tests/` directory, never `#[cfg(test)]` in source

---

## The Synergy

### Complementary Strengths

The elicitation and rmcp systems operate at different layers:

```
┌─────────────────────────────────────────────────────────────┐
│                   MCP Client (Claude)                       │
└───────────────────────────┬─────────────────────────────────┘
                            │ JSON-RPC (MCP Protocol)
┌───────────────────────────▼─────────────────────────────────┐
│              RMCP Layer (Tool Protocol)                     │
│  - Tool discovery                                           │
│  - Parameter deserialization                                │
│  - Handler routing                                          │
│                                                             │
│  #[tool_router]                                             │
│  #[tool] async fn search(params: SearchRequest)            │
└───────────────────────────┬─────────────────────────────────┘
                            │ Type-safe Rust types
┌───────────────────────────▼─────────────────────────────────┐
│         Elicitation Layer (User Interaction)                │
│  - Conversational prompts                                   │
│  - Type-specific validation                                 │
│  - Re-prompting on error                                    │
│                                                             │
│  #[derive(Elicit)]                                          │
│  struct SearchRequest { ... }                               │
└─────────────────────────────────────────────────────────────┘
```

### Pattern 1: Elicitation → RMCP (Input Path)

**Scenario**: Tool needs complex configuration from user conversation

```rust
// 1. Define the request type
#[derive(Tool, Elicit, Builder, Getters, Serialize, Deserialize)]
#[tool(name = "search_database", description = "Search database with filters")]
#[setters(prefix = "with_")]
pub struct SearchRequest {
    #[tool(description = "SQL query pattern")]
    #[prompt("Enter your search query:")]
    query: String,
    
    #[tool(description = "Maximum results (1-100)")]
    #[prompt("How many results? (1-100)")]
    #[builder(default = "10")]
    limit: u32,
    
    #[tool(description = "Sort order")]
    #[prompt("Choose sort order:")]
    #[builder(default = "SortOrder::Relevance")]
    sort: SortOrder,
}

// 2. Elicit from user conversation
let request = SearchRequest::elicit(client).await?;

// 3. Use in tool implementation
#[tool_handler]
#[instrument(skip(db), fields(query = %request.query(), limit = request.limit()))]
async fn search_database(
    db: &Database,
    request: SearchRequest,
) -> Result<SearchResponse, ToolError> {
    // Type-safe access via getters
    let query = request.query();
    let limit = request.limit();
    let sort = request.sort();
    
    debug!("Executing search");
    let results = db.search(query, *limit, *sort).await?;
    info!(count = results.len(), "Search complete");
    
    Ok(SearchResponse::new(results))
}
```

**Benefits**:
- ✅ Single type definition for both layers
- ✅ Compile-time type safety end-to-end
- ✅ Automatic schema generation (rmcp)
- ✅ Automatic validation (elicitation + type system)
- ✅ No manual JSON parsing
- ✅ Self-documenting API

### Pattern 2: RMCP → Elicitation (Tool Composition)

**Scenario**: One tool's output feeds another tool's input

```rust
// Tool 1: List available options (produces typed output)
#[derive(Tool, Serialize, Deserialize)]
#[tool(name = "list_categories", description = "List all categories")]
pub struct ListCategoriesResponse {
    categories: Vec<Category>,
    total: usize,
}

#[tool_handler]
async fn list_categories(db: &Database) -> Result<ListCategoriesResponse, ToolError> {
    let categories = db.list_categories().await?;
    let total = categories.len();
    Ok(ListCategoriesResponse { categories, total })
}

// Tool 2: Filter selection (uses elicitation for choice)
#[derive(Tool, Elicit, Serialize, Deserialize)]
#[tool(name = "filter_by_category", description = "Filter items by category")]
pub struct FilterRequest {
    #[prompt("Which category should we filter by?")]
    category: Category,  // Uses Select pattern (enum)
}

#[tool_handler]
async fn filter_by_category(
    db: &Database,
    request: FilterRequest,
) -> Result<FilterResponse, ToolError> {
    let items = db.filter_by_category(request.category()).await?;
    Ok(FilterResponse { items })
}

// Workflow composition
async fn multi_step_search(
    client: &Client,
    db: &Database,
) -> Result<Vec<Item>, ToolError> {
    // Step 1: List available categories (rmcp tool)
    let categories_resp = list_categories(db).await?;
    info!(count = categories_resp.total, "Listed categories");
    
    // Step 2: Elicit user's choice (elicitation)
    let choice = Category::elicit(client).await?;
    debug!(category = ?choice, "User selected category");
    
    // Step 3: Execute filtered search (rmcp tool)
    let request = FilterRequest::new(choice);
    let response = filter_by_category(db, request).await?;
    
    Ok(response.items)
}
```

**Benefits**:
- ✅ Tools produce structured outputs (rmcp serialization)
- ✅ Those outputs feed elicitation prompts (type composition)
- ✅ LLM sees consistent schema across tool boundaries
- ✅ State machines emerge from type composition
- ✅ Observable via tracing at each step

### Pattern 3: Self-Healing Validation

**Scenario**: Type constraints automatically enforce re-prompting

```rust
// Define a ranged type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port(u16);

impl Port {
    pub fn new(value: u16) -> Result<Self, ValidationError> {
        if (1..=65535).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ValidationError::OutOfRange {
                value: value as i64,
                min: 1,
                max: 65535,
            })
        }
    }
}

impl Prompt for Port {
    fn prompt() -> Option<&'static str> {
        Some("Enter port number (1-65535):")
    }
}

impl Elicitation for Port {
    #[instrument(skip(client))]
    async fn elicit<T: rmcp::transport::Transport>(
        client: &rmcp::Client<T>,
    ) -> ElicitResult<Self> {
        loop {
            let value: u16 = u16::elicit(client).await?;
            
            match Self::new(value) {
                Ok(port) => {
                    debug!(port = value, "Valid port selected");
                    return Ok(port);
                }
                Err(e) => {
                    warn!(error = %e, value, "Invalid port, re-prompting");
                    // Re-prompt automatically with error context
                    continue;
                }
            }
        }
    }
}

// Usage in tool
#[derive(Tool, Elicit)]
pub struct ServerConfig {
    #[prompt("Server listening port:")]
    port: Port,  // Automatically validates and re-prompts
}
```

**Benefits**:
- ✅ Validation logic in one place (Port::new)
- ✅ Automatic re-prompting on invalid input
- ✅ User-friendly error messages
- ✅ No manual validation in tool handlers
- ✅ Compile-time guarantee of valid values

---

## Implementation Phases

### Phase 1: Establish RMCP Foundation ✅ (Current)

**Goal**: Replace pmcp with rmcp in botticelli, establish patterns

**Status**: In progress on `rmcp` branch

**Completed**:
- ✅ Add rmcp dependencies (rmcp 0.12.0, rmcp-macros 0.12.0)
- ✅ Create error types with `derive_more::Display` + `derive_more::Error`
- ✅ Implement basic tools (Echo, ServerInfo) with `#[derive(Tool)]`
- ✅ Establish builder patterns (derive_builder, derive_new)
- ✅ Add full instrumentation with `#[instrument]`
- ✅ Use private fields with derive_getters

**Remaining** (per RMCP_TOOL_MIGRATION_PLAN.md):
- 🎯 Migrate ValidateNarrative tool
- 🎯 Migrate Execute tools (ExecuteActs, ExecuteSteps, ContinueNarrative)
- 🎯 Migrate LLM integration (unified backend pattern)
- 🎯 Migrate Database tools
- 🎯 Remove all pmcp dependencies
- 🎯 Update tests to match new patterns

**Acceptance Criteria**:
- All tools use `#[derive(Tool)]` from rmcp
- Zero pmcp dependencies in botticelli
- All compilation errors resolved
- All tests passing with new patterns
- Full tracing coverage

### Phase 2: Migrate Elicitation to RMCP 🎯

**Goal**: Replace pmcp with rmcp in the elicitation crate

**Why**: Align elicitation with botticelli's transport layer

**Changes Required**:

1. **Update elicitation dependencies** (`elicitation/Cargo.toml`):
```toml
[workspace.dependencies]
# Replace pmcp with rmcp
rmcp = "0.12"
rmcp-macros = "0.12"

# Keep existing
derive_more = { version = "1", features = ["display", "error", "from"] }
derive-getters = "0.5"
tracing = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

2. **Update trait signatures** (`elicitation/src/traits.rs`):
```rust
pub trait Elicitation: Sized + Prompt {
    /// Elicit a value via RMCP client.
    fn elicit<T: rmcp::transport::Transport>(
        client: &rmcp::Client<T>,
    ) -> impl std::future::Future<Output = ElicitResult<Self>> + Send;
}
```

3. **Update primitive implementations** (`elicitation/src/primitives/*.rs`):
```rust
impl Elicitation for bool {
    #[instrument(skip(client))]
    async fn elicit<T: rmcp::transport::Transport>(
        client: &rmcp::Client<T>,
    ) -> ElicitResult<Self> {
        let prompt = Self::prompt().unwrap_or("Please confirm (yes/no):");
        debug!("Eliciting boolean");
        
        let params = mcp::bool_params(prompt);
        let result = client.call_tool("elicit_bool", params).await?;
        
        let value = mcp::extract_value(result)?;
        mcp::parse_bool(value)
    }
}
```

4. **Update container implementations** (Option, Vec, Result, etc.):
- Change `pmcp::Client<T>` to `rmcp::Client<T>`
- Keep all logic unchanged (transport-agnostic)

5. **Update derive macro** (`elicitation_derive/src/*.rs`):
- Generate code using `rmcp::Client<T>` instead of `pmcp::Client<T>`
- No other changes (macro logic is transport-agnostic)

6. **Update error types** (`elicitation/src/error.rs`):
```rust
use derive_more::{Display, Error, From};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum ElicitErrorKind {
    #[display("JSON error: {}", _0)]
    Json(String),
    
    #[display("RMCP error: {}", _0)]
    Rmcp(String),
    
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),
    
    #[display("User cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Display, Error)]
#[display("Elicit error: {} at {}:{}", kind, file, line)]
pub struct ElicitError {
    pub kind: ElicitErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl ElicitError {
    #[track_caller]
    pub fn new(kind: ElicitErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

pub type ElicitResult<T> = Result<T, ElicitError>;

// Convert from rmcp errors
impl From<rmcp::Error> for ElicitError {
    fn from(e: rmcp::Error) -> Self {
        Self::new(ElicitErrorKind::Rmcp(e.to_string()))
    }
}
```

7. **Update re-exports** (`elicitation/src/lib.rs`):
```rust
// Replace pmcp re-export
pub use rmcp;
```

**Testing**:
- Run existing elicitation tests with rmcp
- Verify all primitive types work
- Verify container types work
- Test derive macro with rmcp

**Acceptance Criteria**:
- Zero pmcp dependencies in elicitation
- All existing tests pass
- Published to crates.io as elicitation v0.2.0
- Documentation updated with rmcp examples

### Phase 3: Unify Tool Definitions 🎯

**Goal**: Define elicitation's internal tools using rmcp derives

**Current** (elicitation internal tools):
```rust
// Simple JSON builders
pub fn bool_params(prompt: &str) -> serde_json::Value {
    json!({ "prompt": prompt })
}

pub fn number_params(prompt: &str, min: i64, max: i64) -> serde_json::Value {
    json!({
        "prompt": prompt,
        "min": min,
        "max": max,
    })
}
```

**Future** (rmcp-based tool definitions):
```rust
#[derive(Tool, Serialize, Deserialize, Getters)]
#[tool(name = "elicit_bool", description = "Elicit a boolean value")]
pub struct ElicitBoolRequest {
    #[tool(description = "The question to ask")]
    prompt: String,
}

#[derive(Serialize, Deserialize, Getters)]
pub struct ElicitBoolResponse {
    value: bool,
}

#[derive(Tool, Serialize, Deserialize, Getters)]
#[tool(name = "elicit_number", description = "Elicit a numeric value")]
pub struct ElicitNumberRequest {
    #[tool(description = "The question to ask")]
    prompt: String,
    
    #[tool(description = "Minimum valid value")]
    min: i64,
    
    #[tool(description = "Maximum valid value")]
    max: i64,
}

#[derive(Serialize, Deserialize, Getters)]
pub struct ElicitNumberResponse {
    value: i64,
}
```

**Benefits**:
- ✅ Elicitation tools are "just tools" with rmcp schemas
- ✅ Can be tested/mocked independently
- ✅ Schema generation is automatic
- ✅ Type-safe on both sides
- ✅ Self-documenting

**Changes**:
1. Create `elicitation/src/mcp/tool_types.rs` with rmcp-based tool definitions
2. Update implementations to use structured types instead of JSON builders
3. Keep `tool_names` module for string constants
4. Add tool handlers that serve these tools (for standalone elicitation server)

**Acceptance Criteria**:
- All elicitation tools defined with `#[derive(Tool)]`
- Tool schemas auto-generated
- Backward compatible with existing usage
- Tests verify tool schema correctness

### Phase 4: Enable Dual-Derive Pattern 🚀

**Goal**: Support `#[derive(Tool, Elicit)]` on the same type

**Use Case**: Request types that are both MCP tools AND elicitable values

```rust
// A type that works both ways
#[derive(Tool, Elicit, Builder, Getters, Setters, Serialize, Deserialize)]
#[tool(name = "search_config", description = "Search configuration")]
#[setters(prefix = "with_")]
pub struct SearchConfig {
    #[tool(description = "Search query pattern")]
    #[prompt("Enter search query:")]
    query: String,
    
    #[tool(description = "Maximum results (1-100)")]
    #[prompt("How many results?")]
    #[builder(default = "10")]
    limit: u32,
    
    #[tool(description = "Sort order")]
    #[prompt("Choose sort order:")]
    #[builder(default = "SortOrder::Relevance")]
    sort: SortOrder,
}

// Can be used as an MCP tool parameter (rmcp)
#[tool_handler]
#[instrument(skip(db))]
async fn execute_search(
    db: &Database,
    config: SearchConfig,
) -> Result<SearchResults, ToolError> {
    debug!(query = %config.query(), "Executing search");
    db.search(config.query(), *config.limit(), *config.sort()).await
}

// Can also be elicited from conversation (elicitation)
let config = SearchConfig::elicit(client).await?;
```

**Implementation**:
1. Ensure `#[derive(Tool)]` and `#[derive(Elicit)]` don't conflict
2. Both generate different trait implementations (no overlap)
3. May need unified attribute syntax for prompts/descriptions
4. Consider `#[mcp(description = "...", prompt = "...")]` that both macros understand

**Challenges**:
- Attribute overlap: `#[tool(...)]` vs `#[prompt(...)]`
- May need attribute coordination between macros
- Documentation clarity (which derive does what)

**Solutions**:
1. **Option A**: Keep separate attributes, accept some duplication
   ```rust
   #[tool(description = "For LLM schema")]
   #[prompt("For user interaction")]
   field: Type,
   ```

2. **Option B**: Unified attribute namespace
   ```rust
   #[mcp(description = "For schema", prompt = "For user")]
   field: Type,
   ```

**Acceptance Criteria**:
- Types can derive both Tool and Elicit
- No macro conflicts or compilation errors
- Clear documentation on attribute usage
- Examples demonstrating dual-derive pattern

### Phase 5: Advanced Elicitation Patterns 🔮

**Goal**: Implement Survey and Authorize patterns

#### Survey Pattern (Multi-Field Structs)

**Current Status**: Planned, not implemented

**Goal**: Sequential field elicitation for structs

```rust
#[derive(Tool, Elicit, Builder, Getters)]
#[tool(name = "database_config")]
pub struct DatabaseConfig {
    #[prompt("Database host:")]
    host: String,
    
    #[prompt("Port number:")]
    port: u16,
    
    #[prompt("Enable SSL?")]
    ssl: bool,
    
    #[prompt("Select authentication method:")]
    auth: AuthMethod,  // Nested Select
}

// Generated state machine for sequential elicitation
impl Elicitation for DatabaseConfig {
    async fn elicit<T: rmcp::transport::Transport>(
        client: &rmcp::Client<T>,
    ) -> ElicitResult<Self> {
        // Elicit each field in order
        let host = String::elicit(client).await?;
        let port = u16::elicit(client).await?;
        let ssl = bool::elicit(client).await?;
        let auth = AuthMethod::elicit(client).await?;
        
        Ok(DatabaseConfig { host, port, ssl, auth })
    }
}
```

**Implementation**:
1. Extend `#[derive(Elicit)]` to handle structs
2. Generate sequential field elicitation
3. Support field attributes: `#[prompt("...")]`, `#[skip]`, `#[default]`
4. Handle nested elicitation (struct fields that are themselves Elicit)

#### Authorize Pattern (Permission Policies)

**Future**: Permission and confirmation policies

```rust
#[derive(Tool, Authorize, Elicit)]
pub struct DangerousOperation {
    #[require_confirm("This will delete all data. Are you sure?")]
    #[require_role("admin")]
    target: DatabaseName,
}

// Generates automatic authorization checks
impl Authorize for DangerousOperation {
    async fn authorize<T: rmcp::transport::Transport>(
        client: &rmcp::Client<T>,
        context: &SecurityContext,
    ) -> AuthorizeResult<Self> {
        // Check role
        if !context.has_role("admin") {
            return Err(AuthorizeError::InsufficientPermissions);
        }
        
        // Elicit target
        let target = DatabaseName::elicit(client).await?;
        
        // Require confirmation
        let prompt = "This will delete all data. Are you sure?";
        if !bool::elicit_with_prompt(client, prompt).await? {
            return Err(AuthorizeError::UserCancelled);
        }
        
        Ok(Self { target })
    }
}
```

**Acceptance Criteria**:
- Survey pattern works for multi-field structs
- Authorize pattern enforces permission policies
- Clear documentation on advanced patterns
- Examples demonstrating real-world use cases

---

## Design Principles

### 1. Types as Documentation

**Principle**: The type definition IS the documentation

```rust
// The type tells you everything
#[derive(Tool, Elicit, Builder, Getters)]
#[tool(name = "read_file", description = "Read file contents")]
pub struct FileReadRequest {
    #[tool(description = "Path to file (must exist)")]
    #[prompt("Which file should I read?")]
    path: ExistingPath,  // Custom validated type
    
    #[tool(description = "Maximum bytes to read")]
    #[prompt("How many bytes? (default: all)")]
    #[builder(default = "None")]
    max_bytes: Option<NonZeroUsize>,
}
```

**Benefits**:
- No separate schema files
- No manual validation code
- No synchronization problems
- Single source of truth

### 2. Trait Sandwich / Trait Accordion

**Principle**: Force separation of concerns via trait boundaries

```
┌─────────────────┐
│  Tool Handler   │  Business logic (domain-specific)
├─────────────────┤
│  impl Tool      │  Protocol layer (rmcp - schema generation)
├─────────────────┤
│  Request Type   │  Domain model (shared vocabulary)
├─────────────────┤
│  impl Elicit    │  Interaction layer (elicitation - parsing)
├─────────────────┤
│  LLM Response   │  Transport layer (JSON)
└─────────────────┘
```

**Benefits**:
- Each layer depends only on traits
- Easy to test layers independently
- Refactoring within layers doesn't break boundaries
- Clear separation of concerns

### 3. Builder-Everywhere

**Principle**: Never use struct literals, always builders

```rust
// ❌ BAD: Struct literal (breaks on field additions)
let request = SearchRequest {
    query: "rust async".to_string(),
    limit: 50,
    sort: SortOrder::Relevance,
};

// ✅ GOOD: Builder pattern (future-proof)
let request = SearchRequestBuilder::default()
    .query("rust async")
    .limit(50)
    .sort(SortOrder::Relevance)
    .build()?;

// ✅ ALSO GOOD: Elicited from conversation
let request = SearchRequest::elicit(client).await?;
```

**Benefits**:
- Self-documenting construction
- Optional fields with defaults
- Validation at build time
- IDE autocomplete support
- Future-proof against field additions

### 4. Private Fields + Derived Access

**Principle**: Never expose fields directly, always use getters/setters

```rust
#[derive(Tool, Elicit, Builder, Getters, Setters)]
#[setters(prefix = "with_")]
pub struct Config {
    #[tool(description = "API key for authentication")]
    #[prompt("Enter your API key:")]
    api_key: String,  // Private!
    
    #[tool(description = "Request timeout in seconds")]
    #[prompt("Timeout (seconds):")]
    #[builder(default = "30")]
    timeout: u64,  // Private!
}

// Usage
let key = config.api_key();              // Getter
let config = config.with_api_key(new_key);  // Setter (builder style)
```

**Benefits**:
- Encapsulation (can change internal representation)
- Immutable by default (setters return new instance)
- Type safety (can't directly assign wrong type)
- Refactoring safety (compiler catches usage)

### 5. Observable by Default

**Principle**: Every public function has `#[instrument]`

```rust
#[tool_handler]
#[instrument(skip(db), fields(query = %request.query(), limit = request.limit()))]
async fn search(
    db: &Database,
    request: SearchRequest,
) -> Result<SearchResponse, ToolError> {
    debug!("Starting search");
    
    let results = db.query(request.query()).await?;
    debug!(count = results.len(), "Query complete");
    
    let filtered = results.into_iter()
        .take(*request.limit() as usize)
        .collect();
    
    info!(returned = filtered.len(), "Search successful");
    Ok(SearchResponse::new(filtered))
}
```

**Benefits**:
- Automatic span creation
- Structured logging
- Performance tracking
- Error context
- Debugging without debugger

### 6. Error Handling with derive_more

**Principle**: Never write manual Display or Error impls

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ToolErrorKind {
    #[display("Database error: {}", _0)]
    Database(String),
    
    #[display("Invalid parameter: {}", _0)]
    InvalidParameter(String),
    
    #[display("Operation cancelled by user")]
    Cancelled,
}

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Tool error: {} at {}:{}", kind, file, line)]
pub struct ToolError {
    pub kind: ToolErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl ToolError {
    #[track_caller]
    pub fn new(kind: ToolErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}
```

**Benefits**:
- Automatic Display implementation
- Automatic Error implementation
- Location tracking for debugging
- Less boilerplate

---

## Success Metrics

### 1. Compilation Success Rate

**Metric**: Percentage of compilations that result in working tools

**Target**: ≥95% (if it compiles, it works)

**Current baseline** (dynamic languages): ~60% first-attempt success

**How to measure**:
- Track compilation attempts vs working tools
- Exclude intentional refactoring
- Focus on new tool development

### 2. Debugging Token Cost

**Metric**: Tokens spent on type-related debugging / total conversation tokens

**Target**: <10% of conversation tokens on type issues

**How to measure**:
- Tag conversations by topic (type debugging vs logic debugging)
- Measure token usage per topic
- Compare with baseline (Python/TypeScript MCP)

### 3. Refactoring Safety

**Metric**: Runtime type errors after refactoring

**Target**: Zero runtime type errors from API changes

**How to measure**:
- Track errors after refactoring
- Categorize: compile-time caught vs runtime discovered
- Goal: 100% caught at compile time

### 4. Time to Correct Tool

**Metric**: Time from idea to working implementation

**Baseline**:
- Initial implementation: Slower (Rust overhead)
- Iteration: Faster (compiler catches issues)
- Break-even: ~3 iterations

**How to measure**:
- Track time for new tool development
- Measure iterations required
- Compare with equivalent Python/TypeScript

### 5. Performance Benchmarks

**Metric**: Requests per second, latency, memory usage

**Targets**:
- Throughput: ≥1000 req/s
- P99 latency: <10ms
- Memory: <50MB resident

**How to measure**:
- Benchmark harness with realistic workloads
- Compare with Python/TypeScript equivalents
- Profile hotspots

---

## Comparison with Alternatives

### Python MCP

**Pros**:
- Fastest prototyping
- Largest ecosystem
- Easy to learn
- Dynamic flexibility

**Cons**:
- Runtime type errors
- Manual validation boilerplate
- Schema/code can diverge
- Performance limitations
- GIL for concurrency

**When to choose Python**:
- Quick prototypes
- Scripts and one-offs
- Teams with Python expertise
- Non-performance-critical tools

### TypeScript MCP

**Pros**:
- Good type system
- JavaScript ecosystem
- Faster than Python
- Good tooling

**Cons**:
- Still runtime validation
- Type erasure (no runtime types)
- Less control over performance
- Memory overhead (V8)

**When to choose TypeScript**:
- Web integration
- Node.js ecosystem
- Teams with TS expertise
- Moderate performance needs

### Rust + Elicitation + RMCP

**Pros**:
- Compile-time correctness
- Zero-cost abstractions
- Self-documenting types
- Schema generation automatic
- Refactoring safety
- Maximum performance
- Memory safety

**Cons**:
- Steeper learning curve
- Longer compile times
- More verbose (sometimes)
- Smaller ecosystem (for MCP)

**When to choose Rust**:
- Production-critical tools
- Performance requirements
- Long-term maintenance
- Type safety critical
- Resource constraints

### The Trade-Off

**Invest time upfront** (learning Rust, defining types, waiting for compilation)  
**Save time later** (fewer bugs, easier refactoring, faster execution, lower costs)

**Break-even point**: After ~3 major iterations, Rust's safety pays for itself

---

## Next Steps

### Immediate (rmcp Branch)

1. ✅ Complete rmcp integration in botticelli
2. 🎯 Migrate all existing tools to rmcp patterns
3. 🎯 Establish testing patterns for rmcp tools
4. 🎯 Update documentation with examples
5. 🎯 Fix all compilation errors
6. 🎯 Pass all tests

**Timeline**: 1-2 weeks

### Short Term (Q1 2025)

1. Migrate elicitation crate to rmcp (Phase 2)
2. Publish elicitation v0.2.0 with rmcp support
3. Create dual-derive examples (Tool + Elicit) (Phase 4)
4. Write "Why Rust for MCP" blog post
5. Update PLANNING_INDEX.md

**Timeline**: 1-2 months

### Medium Term (Q2 2025)

1. Implement Survey pattern (struct elicitation) (Phase 5)
2. Add Authorize pattern (permission policies) (Phase 5)
3. Build tool composition examples
4. Create performance benchmarks vs Python/TypeScript
5. Publish case studies

**Timeline**: 2-3 months

### Long Term (2025+)

1. Propose elicitation patterns to MCP spec
2. Build rmcp + elicitation starter template
3. Create IDE plugins for tool development
4. Establish Rust MCP ecosystem
5. Publish comprehensive guide

**Timeline**: 6-12 months

---

## Conclusion

The integration of `elicitation` and `rmcp` represents a **paradigm shift** in AI tool development:

### The Vision

**Type-safe end-to-end**: LLM conversation → JSON → Rust types → Application logic, with compile-time guarantees at every step.

**Self-documenting**: Types are the source of truth for schemas, validation, and documentation.

**Compiler-verified**: Catch errors before runtime, reducing debugging time and token costs.

**Zero-cost**: Abstractions compile away, enabling high-performance use cases.

**Composable**: Trait-based architecture enables flexible tool composition.

### Why This Matters

1. **Compiler does heavy lifting**: Type checking, validation, schema generation happen automatically
2. **Tokens are expensive**: Fewer tokens spent debugging types = lower AI costs
3. **Reliability matters**: Production AI tools need to be trustworthy
4. **Performance enables new use cases**: Real-time, embedded, high-throughput scenarios

### The Argument for Rust

We're not just building an MCP server—we're **proving that Rust's strengths** (safety, performance, expressiveness) **map directly to AI tooling needs**.

**The future of AI infrastructure is type-safe, and Rust is uniquely positioned to deliver it.**

---

## References

- **CLAUDE.md** - Coding standards and patterns
- **ELICITATION_RMCP_SYNERGY.md** - Architectural complementarity analysis
- **RMCP_MIGRATION_VISION.md** - High-level migration strategy
- **RMCP_TOOL_MIGRATION_PLAN.md** - Concrete migration steps
- **Elicitation crate** - `/home/erik/repos/elicitation`
- **RMCP crate** - https://docs.rs/rmcp/latest/rmcp/

---

*Document created: 2025-12-30*  
*Status: Vision Document - Implementation Strategy*  
*Cross-ref: See references section*
