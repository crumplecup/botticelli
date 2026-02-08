# Proposal: `#[elicit_trait_tools]` Macro for Trait-Based Tool Generation

## Executive Summary

This proposal introduces `#[elicit_trait_tools]` to the `elicitation` crate, enabling automatic MCP tool generation from trait definitions. This eliminates boilerplate wrapper code and enables a "tool everything" architecture where entire APIs can be exposed as MCP tools with minimal ceremony.

**Impact**: Reduces tool wrapper code by ~80-90% for trait-heavy APIs, enabling comprehensive MCP tool coverage for large Rust libraries.

## Problem Statement

### Current State: Manual Wrapper Boilerplate

The `elicitation` crate currently provides excellent support for generating MCP tools from **types** via `#[elicit_tools(...)]`. However, when exposing **trait methods** as tools, developers must write manual wrapper functions, leading to significant boilerplate.

**Example**: For a trait with 10 methods, you need ~150 lines of boilerplate just for wrappers.

```rust
// Your domain trait (clean, focused on business logic)
pub trait EventHandler {
    async fn process_guild_create(&self, guild: &Guild) -> Result<()>;
    async fn process_message(&self, msg: &Message) -> Result<()>;
    async fn process_reaction(&self, reaction: &Reaction) -> Result<()>;
    // ... 7 more methods
}

// Implementation (also clean)
impl EventHandler for DiscordHandler {
    async fn process_guild_create(&self, guild: &Guild) -> Result<()> {
        // Business logic
    }
    // ... other methods
}

// ❌ PROBLEM: Required manual wrapper for EVERY method
#[tool_router(router = event_handler_tools, vis = "pub")]
impl BotticelliServer {
    /// Process guild creation event
    #[tool]
    pub async fn process_guild_create(
        &self,
        params: Parameters<ProcessGuildCreateParams>,
    ) -> Result<Json<ProcessGuildCreateResult>, rmcp::ErrorData> {
        let handler = self.get_handler()?;
        let guild = convert_params(&params.0)?;
        handler.process_guild_create(&guild).await
            .map_err(|e| rmcp::ErrorData::from(e))?;
        Ok(Json(ProcessGuildCreateResult { success: true }))
    }

    /// Process message event
    #[tool]
    pub async fn process_message(
        &self,
        params: Parameters<ProcessMessageParams>,
    ) -> Result<Json<ProcessMessageResult>, rmcp::ErrorData> {
        let handler = self.get_handler()?;
        let msg = convert_params(&params.0)?;
        handler.process_message(&msg).await
            .map_err(|e| rmcp::ErrorData::from(e))?;
        Ok(Json(ProcessMessageResult { success: true }))
    }

    // ... 8 more nearly identical wrappers (120+ more lines!)
}
```

### Why This Matters for Large Rust Libraries

We're building `botticelli`, a comprehensive LLM toolkit with:
- **50+ trait methods** across multiple interfaces
- **200+ types** in domain model
- Goal: Expose **entire library** as MCP tools for AI agent consumption

**Current approach**: 
- ✅ Types: `#[elicit_tools(Type1, Type2, ...)]` works great (minimal code)
- ❌ Methods: Manual wrappers for every trait method (massive boilerplate)

**Result**: ~3000 lines of wrapper code vs ~50 lines with proposed macro.

### Why Trait Methods Can't Use `#[tool]` Directly

The `#[tool]` macro generates code (schema metadata, registration, potentially wrapper logic) that modifies the function. Trait implementations must **exactly** match the trait signature—no modifications allowed.

```rust
impl EventHandler for DiscordHandler {
    #[tool]  // ❌ ERROR E0407: macro changes signature
    async fn process_event(&self, event: Event) -> Result<()> {
        // ...
    }
}
```

This is a fundamental Rust limitation, not a bug.

## Proposed Solution: `#[elicit_trait_tools]`

### Design Overview

Add a new macro that:
1. Scans trait definition for methods
2. Generates delegating wrapper functions with MCP-compatible signatures
3. Integrates with existing `#[tool_router]` for registration
4. Requires trait methods use MCP-compatible signatures

### API Design

```rust
use elicitation::elicit_trait_tools;
use rmcp::{tool_router, Parameters, Json};

// Step 1: Define parameters/results with Elicit
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ProcessGuildCreateParams {
    pub guild_id: String,
    pub guild_name: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ProcessGuildCreateResult {
    pub success: bool,
}

// Step 2: Define trait with MCP-compatible signatures
#[elicit_trait_tools]  // ✨ NEW: Mark trait for tool generation
pub trait EventHandler: Send + Sync {
    /// Process guild creation event.
    async fn process_guild_create(
        &self,
        params: Parameters<ProcessGuildCreateParams>,
    ) -> Result<Json<ProcessGuildCreateResult>, rmcp::ErrorData>;

    /// Process message event.
    async fn process_message(
        &self,
        params: Parameters<ProcessMessageParams>,
    ) -> Result<Json<ProcessMessageResult>, rmcp::ErrorData>;

    // ... other methods
}

// Step 3: Normal trait implementation (no #[tool] needed!)
impl EventHandler for DiscordHandler {
    async fn process_guild_create(
        &self,
        params: Parameters<ProcessGuildCreateParams>,
    ) -> Result<Json<ProcessGuildCreateResult>, rmcp::ErrorData> {
        // Business logic
        let guild = self.db.create_guild(&params.0).await?;
        Ok(Json(ProcessGuildCreateResult { success: true }))
    }

    async fn process_message(
        &self,
        params: Parameters<ProcessMessageParams>,
    ) -> Result<Json<ProcessMessageResult>, rmcp::ErrorData> {
        // Business logic
        self.db.store_message(&params.0).await?;
        Ok(Json(ProcessMessageResult { success: true }))
    }
}

// Step 4: Generate tools with a single line! ✨
#[elicit_trait_tools_router(EventHandler, handler)]
#[tool_router(router = event_handler_tools, vis = "pub")]
impl BotticelliServer {
    // ✨ Macro auto-generates all delegating wrappers!
    // No manual code needed for each method!
}
```

**Result**: ~10 lines of tool registration code instead of ~150 lines of boilerplate.

## Implementation Guidance

### Generated Code Pattern

For each trait method, the macro should generate:

```rust
// Input: Trait method signature
async fn process_guild_create(
    &self,
    params: Parameters<ProcessGuildCreateParams>,
) -> Result<Json<ProcessGuildCreateResult>, rmcp::ErrorData>;

// Generated: Delegating tool wrapper
#[tool(description = "Process guild creation event")]
pub async fn process_guild_create(
    &self,
    params: Parameters<ProcessGuildCreateParams>,
) -> Result<Json<ProcessGuildCreateResult>, rmcp::ErrorData> {
    // Delegate to trait method on inner handler
    self.handler.process_guild_create(params).await
}
```

### Macro Attributes

The `#[elicit_trait_tools_router]` macro needs:

```rust
#[elicit_trait_tools_router(
    trait_name = EventHandler,    // Trait to scan
    field_name = handler,          // Field on impl type holding trait object
    prefix = None,                 // Optional: prefix tool names
)]
```

### Requirements for Trait Methods

To be compatible with `#[elicit_trait_tools]`, trait methods must:

1. **Use `Parameters<T>`** for input (where `T: Elicit`)
2. **Return `Result<Json<R>, rmcp::ErrorData>`** (where `R: Elicit`)
3. **Be async** (for MCP tool compatibility)
4. **Have doc comments** (used for tool descriptions)

### Alternative: Two-Phase Approach

If scanning trait definitions is complex, offer a simpler variant that scans impl blocks:

```rust
// Developer writes the trait impl with MCP signatures
impl EventHandler for DiscordHandler {
    /// Process guild creation event
    async fn process_guild_create(
        &self,
        params: Parameters<ProcessGuildCreateParams>,
    ) -> Result<Json<ProcessGuildCreateResult>, rmcp::ErrorData> {
        // Implementation
    }
}

// Macro generates tools from impl block methods
#[elicit_impl_tools(DiscordHandler)]
#[tool_router(router = event_handler_tools, vis = "pub")]
impl BotticelliServer {
    // Auto-generates wrappers for all impl methods
}
```

## Use Cases

### 1. Large API Surface Area

**Botticelli** (our use case):
- 50+ trait methods across 10+ traits
- Without macro: ~3000 lines of wrapper code
- With macro: ~50 lines of declarations

### 2. Rapid Prototyping

Developers can expose entire APIs as MCP tools during development, then selectively hide methods for production:

```rust
#[elicit_trait_tools(only = ["create", "read", "update"])]
pub trait Repository {
    // Only these methods become tools
}
```

### 3. Plugin Architectures

Dynamic trait implementations (plugins) automatically get tool wrappers:

```rust
// Plugin trait
#[elicit_trait_tools]
pub trait GamePlugin: Send + Sync {
    async fn on_player_join(...) -> Result<...>;
    async fn on_player_leave(...) -> Result<...>;
}

// Any plugin implementation automatically gets MCP tools
impl GamePlugin for MyPlugin { /* ... */ }
```

### 4. Testing and Debugging

Expose internal traits as tools during testing:

```rust
#[cfg(test)]
#[elicit_trait_tools]
pub trait InternalDebugInterface {
    async fn dump_state(...) -> Result<...>;
    async fn reset_cache(...) -> Result<...>;
}
```

## Benefits

### For Library Authors

1. **Minimal Ceremony**: Single attribute to expose entire trait as tools
2. **Type Safety**: Compile-time verification of parameter/result types
3. **Documentation**: Doc comments automatically become tool descriptions
4. **Maintainability**: Change trait signature → tools update automatically
5. **Discoverability**: All trait methods visible in MCP tool listings

### For Library Users (AI Agents)

1. **Comprehensive Tool Coverage**: Access to entire API surface
2. **Consistent Interface**: All tools follow same parameter/result patterns
3. **Self-Documenting**: Tool schemas match Rust types exactly
4. **Type Guidance**: JSON schemas help agents construct valid requests

### For Ecosystem

1. **"Tool-Native" Libraries**: Rust libraries designed from the start for MCP tool exposure
2. **Standardization**: Common pattern for trait → tool conversion
3. **Reduced Boilerplate**: Makes MCP adoption more attractive
4. **Better AI Integration**: Comprehensive tool exposure enables sophisticated agent workflows

## Comparison with Alternatives

### Manual Wrappers (Current Approach)

**Pros**:
- Full control over tool signatures
- Can add custom logic/validation
- No new dependencies

**Cons**:
- ~15-20 lines per method
- Easy to get out of sync with trait
- Doesn't scale to large APIs
- High maintenance burden

### Code Generation Scripts

**Pros**:
- Can generate custom patterns
- Not limited by macro capabilities

**Cons**:
- Separate build step
- Version control noise (generated files)
- Hard to debug generated code
- Disconnected from type system

### Proposed `#[elicit_trait_tools]`

**Pros**:
- ~1 line per trait (vs ~15-20 per method)
- Automatic sync with trait changes
- Type-safe by construction
- Integrated with existing elicitation patterns
- Standard, documented approach

**Cons**:
- Requires trait signatures be MCP-compatible
- Less flexibility than manual wrappers
- May need escape hatches for complex cases

## Migration Path

### For Existing elicitation Users

1. **No Breaking Changes**: Existing `#[elicit_tools]` and `#[tool]` work unchanged
2. **Opt-In**: Only traits marked with `#[elicit_trait_tools]` are affected
3. **Gradual Adoption**: Can mix manual wrappers with generated tools

### For New Projects

1. Design traits with MCP-compatible signatures from the start
2. Use `#[elicit_trait_tools]` for automatic tool generation
3. Override specific tools with manual wrappers when needed

## Open Questions

1. **Trait Object Support**: Should this work with `dyn Trait` or only concrete types?
   - Proposal: Support both, use field access pattern for flexibility

2. **Associated Types**: How to handle traits with associated types?
   - Proposal: Require associated types implement `Elicit` traits

3. **Default Methods**: Should default trait method implementations become tools?
   - Proposal: Yes, treat same as required methods

4. **Visibility**: Should generated tools inherit trait method visibility?
   - Proposal: Make configurable via macro attribute

5. **Error Conversion**: Should macro auto-convert trait errors to `rmcp::ErrorData`?
   - Proposal: No, require trait return `rmcp::ErrorData` directly (type safety)

6. **Method Filtering**: How to exclude certain methods from tool generation?
   - Proposal: Add `#[elicit_tool(skip)]` attribute on trait methods

## Example: Full Integration

Complete example showing the proposed pattern:

```rust
// ============================================================================
// 1. Parameter/Result Types (same as today)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CreateUserParams {
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct CreateUserResult {
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct GetUserParams {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct GetUserResult {
    pub username: String,
    pub email: String,
}

// ============================================================================
// 2. Tool-Native Trait Definition ✨
// ============================================================================

#[elicit_trait_tools]  // ✨ NEW: Enable tool generation
pub trait UserRepository: Send + Sync {
    /// Create a new user.
    ///
    /// This tool creates a user account with the provided username and email.
    async fn create_user(
        &self,
        params: Parameters<CreateUserParams>,
    ) -> Result<Json<CreateUserResult>, rmcp::ErrorData>;

    /// Get user by ID.
    ///
    /// Retrieves user information for the specified user ID.
    async fn get_user(
        &self,
        params: Parameters<GetUserParams>,
    ) -> Result<Json<GetUserResult>, rmcp::ErrorData>;

    /// Delete user by ID.
    #[elicit_tool(skip)]  // ✨ Skip this method (admin only)
    async fn delete_user(
        &self,
        params: Parameters<DeleteUserParams>,
    ) -> Result<Json<DeleteUserResult>, rmcp::ErrorData>;
}

// ============================================================================
// 3. Normal Trait Implementation
// ============================================================================

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl UserRepository for PostgresUserRepository {
    async fn create_user(
        &self,
        params: Parameters<CreateUserParams>,
    ) -> Result<Json<CreateUserResult>, rmcp::ErrorData> {
        let p = params.0;
        
        let user_id = sqlx::query!(
            "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id",
            p.username,
            p.email
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| rmcp::ErrorData::from(e))?
        .id;

        Ok(Json(CreateUserResult {
            user_id: user_id.to_string(),
            created_at: Utc::now().to_rfc3339(),
        }))
    }

    async fn get_user(
        &self,
        params: Parameters<GetUserParams>,
    ) -> Result<Json<GetUserResult>, rmcp::ErrorData> {
        let p = params.0;
        
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            p.user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| rmcp::ErrorData::from(e))?;

        Ok(Json(GetUserResult {
            username: user.username,
            email: user.email,
        }))
    }

    async fn delete_user(
        &self,
        params: Parameters<DeleteUserParams>,
    ) -> Result<Json<DeleteUserResult>, rmcp::ErrorData> {
        // Implementation...
        todo!()
    }
}

// ============================================================================
// 4. MCP Server Integration ✨
// ============================================================================

pub struct MyMcpServer {
    user_repo: Box<dyn UserRepository>,
}

// ✨ Single line generates all tools!
#[elicit_trait_tools_router(UserRepository, user_repo)]
#[tool_router(router = user_repository_tools, vis = "pub")]
impl MyMcpServer {
    // Macro automatically generates:
    //
    // #[tool(description = "Create a new user...")]
    // pub async fn create_user(&self, params: ...) -> Result<...> {
    //     self.user_repo.create_user(params).await
    // }
    //
    // #[tool(description = "Get user by ID...")]
    // pub async fn get_user(&self, params: ...) -> Result<...> {
    //     self.user_repo.get_user(params).await
    // }
    //
    // Note: delete_user skipped due to #[elicit_tool(skip)]
}

// ============================================================================
// 5. Server Setup
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = PostgresUserRepository::new("postgresql://...").await?;
    let server = MyMcpServer {
        user_repo: Box::new(repo),
    };

    // Tools are automatically registered via tool_router
    rmcp::run_server(server, user_repository_tools()).await?;
    
    Ok(())
}
```

**Summary**: 
- **Before**: ~150 lines of wrapper code for 3 trait methods
- **After**: ~10 lines of declarations, rest auto-generated

## Success Metrics

If implemented, we would consider this successful if:

1. **Reduces wrapper code by 80%+** for trait-heavy APIs
2. **Zero-cost abstraction**: No runtime overhead vs manual wrappers
3. **Compile errors guide** users to MCP-compatible signatures
4. **Works with existing** `#[tool_router]` and `#[elicit_tools]` patterns
5. **Community adoption**: Used by 3+ crates within 6 months

## Request for Feedback

We're happy to:
1. **Contribute implementation** (proc macro experience on team)
2. **Provide test cases** from our production use case (botticelli)
3. **Iterate on API design** based on maintainer feedback
4. **Document migration patterns** for existing users

**Questions for Maintainers**:
1. Does this align with elicitation's vision?
2. Preferred API? (`#[elicit_trait_tools]` vs alternatives)
3. Should this be in `elicitation` core or separate crate?
4. Any concerns about trait signature requirements?
5. Timeline for considering this proposal?

## References

- **botticelli project**: https://github.com/crumplecup/botticelli
- **elicitation crate**: https://crates.io/crates/elicitation
- **rmcp (MCP SDK)**: https://github.com/JasonShin/rmcp
- **Model Context Protocol**: https://modelcontextprotocol.io

---

## Appendix: Real-World Code Comparison

### Current Approach (Manual Wrappers)

From `botticelli/crates/botticelli_mcp/src/rmcp_server/tools/models.rs`:

```rust
// 5 model providers × 2 methods (generate + count_tokens) = 10 wrappers
// Each wrapper: ~15-20 lines
// Total: ~150-200 lines

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeminiGenerateParams {
    pub request: serde_json::Value,
}

#[tool(description = "Generate text using Gemini model")]
#[instrument(skip(params))]
pub async fn gemini_generate(
    params: GeminiGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    let client = GeminiClient::new()?;
    Ok(client.generate(&request).await?)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnthropicGenerateParams {
    pub request: serde_json::Value,
}

#[tool(description = "Generate text using Anthropic model")]
#[instrument(skip(params))]
pub async fn anthropic_generate(
    params: AnthropicGenerateParams,
) -> Result<GenerateResponse> {
    use botticelli_interface::BotticelliDriver;
    
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    let client = AnthropicClient::new()?;
    Ok(client.generate(&request).await?)
}

// ... 8 more nearly identical wrappers
```

### Proposed Approach (With `#[elicit_trait_tools]`)

```rust
// Convert BotticelliDriver trait to tool-native signatures
#[elicit_trait_tools]
pub trait BotticelliDriver: Send + Sync {
    /// Generate content using this model provider.
    async fn generate(
        &self,
        params: Parameters<GenerateParams>,
    ) -> Result<Json<GenerateResponse>, rmcp::ErrorData>;

    /// Count tokens for the given text.
    async fn count_tokens(
        &self,
        params: Parameters<CountTokensParams>,
    ) -> Result<Json<CountTokensResult>, rmcp::ErrorData>;
}

// Implementations for each provider (business logic only, no wrapper code)
impl BotticelliDriver for GeminiClient { /* ... */ }
impl BotticelliDriver for AnthropicClient { /* ... */ }
// ... other providers

// Single declaration generates ALL provider tools! ✨
#[elicit_trait_tools_router(BotticelliDriver, provider)]
#[tool_router(router = model_provider_tools, vis = "pub")]
impl BotticelliServer {
    // Auto-generates:
    // - gemini_generate
    // - gemini_count_tokens
    // - anthropic_generate
    // - anthropic_count_tokens
    // - ... all other provider methods
}

// Total: ~20 lines vs ~200 lines (90% reduction!)
```

---

**Contact**: Erik Rose (erik.w.rose@gmail.com)  
**Date**: February 2026  
**Version**: 1.0
