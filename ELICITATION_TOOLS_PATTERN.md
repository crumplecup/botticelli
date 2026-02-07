# Elicitation Tools Pattern

## Overview

The `#[elicit_tools(...)]` attribute macro from `elicitation_macros` generates MCP tool wrappers for types that implement the `Elicit` trait. This allows types to be elicited (constructed interactively) via MCP.

## The Problem

**You cannot mix `#[elicit_tools(...)]` and `#[tool]` methods in the same impl block.**

This will fail to compile:

```rust
// ❌ WRONG - Will not compile
#[elicit_tools(MyType1, MyType2)]
#[tool_router(router = my_tool_router, vis = "pub")]
impl MyServer {
    #[tool]
    pub async fn my_regular_tool(&self, params: MyParams) -> Result<...> {
        // Regular tool implementation
    }
}
```

### Why This Fails

- **`#[elicit_tools]` generates standalone async functions** that take `Peer<RoleServer>` as parameter
- **`#[tool]` methods** are instance methods that take `&self` as the first parameter
- These have incompatible signatures and `tool_router` cannot combine them

The error you'll see:

```
error[E0277]: the trait bound `(Tool, ...): IntoToolRoute<..., _>` is not satisfied
```

## The Solution

**Use separate impl blocks** - one for elicit tools, one for regular tools:

```rust
// ✅ CORRECT - Separate impl blocks

// Impl block 1: Only elicit tools
#[elicit_tools(MyType1, MyType2, MyType3)]
#[tool_router(router = my_elicit_tool_router, vis = "pub")]
impl MyServer {}

// Impl block 2: Regular tools
#[tool_router(router = my_regular_tool_router, vis = "pub")]
impl MyServer {
    #[tool]
    pub async fn my_regular_tool(
        &self,
        Parameters(params): Parameters<MyParams>,
    ) -> Result<Json<MyResult>, rmcp::ErrorData> {
        // Regular tool implementation
    }
}
```

Then combine the routers:

```rust
impl MyServer {
    pub(crate) fn create_tool_router() -> rmcp::handler::server::tool::ToolRouter<Self> {
        Self::my_regular_tool_router() + Self::my_elicit_tool_router()
    }
}
```

## Type Requirements

For a type to work with `#[elicit_tools(...)]`, it must:

1. **Implement `Elicit` trait** (via `#[derive(elicitation::Elicit)]`)
2. **Implement `Serialize` and `Deserialize`** (required for MCP JSON output)
3. **Implement `JsonSchema`** (for MCP tool schema generation)
4. **Be constructible** - must have either:
   - Public fields, OR
   - A `new()` constructor (via `#[derive(derive_new::new)]` or manual impl), OR
   - A builder pattern

### Example Type Setup

```rust
use elicitation::Elicit;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// Enum - no constructor needed
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub enum Status {
    Active,
    Inactive,
}

// Struct with few fields - use derive(new)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit, derive_new::new)]
pub struct Config {
    host: String,
    port: u16,
}

// Struct with many fields - use builder
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit, derive_builder::Builder)]
pub struct ComplexConfig {
    host: String,
    port: u16,
    timeout: Duration,
    max_connections: usize,
    // ... many more fields
}
```

## Real-World Example

From `crates/botticelli_mcp/src/rmcp_server/tools/core_primitives.rs`:

```rust
use crate::rmcp_server::BotticelliServer;
use botticelli_core::{
    BotState, BotStats, BotServerConfig,
    Role, Input, Message, Output,
    // ... 22 more types
};
use elicitation_macros::elicit_tools;
use rmcp::{tool, tool_router};

// Impl block for 25 elicit tools
#[elicit_tools(
    BotState, BotStats, BotServerConfig,
    Role, Input, Message, Output,
    // ... 18 more types
)]
#[tool_router(router = core_primitives_elicit_tool_router, vis = "pub")]
impl BotticelliServer {}

// Separate impl block for regular delegation tools
impl BotticelliServer {
    #[tool]
    #[instrument(skip(self))]
    pub fn core_init_observability(&self) -> ObservabilityResult<()> {
        botticelli_core::init_observability()
    }
    
    // ... more regular tools
}

// Combine routers
impl BotticelliServer {
    pub(crate) fn create_tool_router() -> rmcp::handler::server::tool::ToolRouter<Self> {
        Self::core_tool_router()
            + Self::cache_tool_router()
            + Self::core_primitives_elicit_tool_router()  // ← Elicit router added
    }
}
```

## Common Issues

### Issue 1: Missing Serialize/Deserialize

**Symptom**: Compilation errors about trait bounds

**Solution**: Add `Serialize` and `Deserialize` to all types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MyType {
    field: String,
}
```

### Issue 2: Private Fields Without Constructor

**Symptom**: Elicit tools can't construct the type

**Solution**: Add a constructor or use derive(new):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit, derive_new::new)]
pub struct MyType {
    private_field: String,
}
```

### Issue 3: Complex DateTime/Duration Fields

**Not an issue!** The elicitation crate has built-in support for:
- `std::time::Duration`
- `chrono::DateTime<Utc>` (with `chrono` feature)
- `time::OffsetDateTime` (with `time` feature)
- `jiff::Timestamp` (with `jiff` feature)

Just ensure the feature is enabled:

```toml
[dependencies]
elicitation = { version = "0.6.8", features = ["chrono", "jiff"] }
```

## Feature Configuration

In workspace `Cargo.toml`:

```toml
[workspace.dependencies]
elicitation = { version = "0.6.8", features = [
    "serde_json",  # JSON Value elicitation
    "chrono",      # DateTime support
    "jiff",        # Alternative datetime library
] }
elicitation_macros = "0.6.8"
```

## Testing

Test that elicit tools are registered:

```rust
#[tokio::test]
async fn test_elicit_tools_registered() {
    let server = BotticelliServer::builder().build().expect("Server build");
    let tool_router = server.get_tool_router();
    let tools = tool_router.list_all();
    
    // Filter to elicit tools
    let elicit_tools: Vec<_> = tools
        .iter()
        .filter(|t| t.name.starts_with("elicit_"))
        .collect();
    
    println!("Found {} elicit tools", elicit_tools.len());
    
    // Verify specific types
    assert!(tools.iter().any(|t| t.name == "elicit_role"));
    assert!(tools.iter().any(|t| t.name == "elicit_message"));
}
```

## Key Takeaways

1. **Separate impl blocks** for elicit tools vs regular tools
2. **Empty impl block** for elicit tools is fine - macro generates the methods
3. **Combine routers** in `create_tool_router()` with `+` operator
4. **Type requirements**: Elicit + Serialize + Deserialize + JsonSchema + constructor
5. **DateTime/Duration support** is built-in - use appropriate feature flags

## References

- Elicitation crate: <https://docs.rs/elicitation>
- Elicitation macros: <https://docs.rs/elicitation_macros>
- RMCP (Rust MCP SDK): <https://docs.rs/rmcp>
- This pattern was discovered in: `crates/botticelli_mcp/src/rmcp_server/tools/`
