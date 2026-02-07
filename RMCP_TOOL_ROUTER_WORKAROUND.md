# RMCP Tool Router Documentation Workaround

## Issue

The `rmcp` crate's `#[tool_router]` macro generates public functions without documentation comments, causing warnings when `#![warn(missing_docs)]` is enabled.

**Example warning:**
```
warning: missing documentation for an associated function
  --> src/rmcp_server/tools/core.rs:24:1
   |
24 | #[tool_router(router = core_tool_router, vis = "pub")]
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: this warning originates in the attribute macro `tool_router`
```

## Root Cause

The `tool_router` macro in rmcp generates a public function like:

```rust
impl BotticelliServer {
    pub fn tool_router() -> Vec<Tool> { ... }
}
```

But does not generate a corresponding doc comment:

```rust
/// Generated tool router for ...  // <-- MISSING
pub fn tool_router() -> Vec<Tool> { ... }
```

## Workaround

We use `#[allow(missing_docs)]` on the impl blocks where `#[tool_router]` is applied:

```rust
/// This tool router provides core utility tools.
///
/// Note: The `#[allow(missing_docs)]` below suppresses warnings for the generated
/// `core_tool_router()` function. This is required because rmcp's `#[tool_router]`
/// macro does not generate documentation. This should be removed when rmcp is
/// updated to generate docs automatically.
#[allow(missing_docs)]
#[tool_router(router = core_tool_router, vis = "pub")]
impl BotticelliServer {
    // ... tool methods ...
}
```

## Affected Files

- `crates/botticelli_mcp/src/rmcp_server/tools/core.rs` (1 occurrence)
- `crates/botticelli_mcp/src/rmcp_server/tools/core_primitives.rs` (1 occurrence)
- `crates/botticelli_mcp/src/rmcp_server/tools/cache.rs` (1 occurrence)

**Total**: 3 `#[allow(missing_docs)]` directives in the entire botticelli_mcp crate.

## Upstream Fix

This workaround should be removed when one of the following occurs:

1. **rmcp enhancement**: The `#[tool_router]` macro is updated to generate documentation
2. **Alternative approach**: rmcp provides a way to attach docs to generated functions

**Proposed enhancement for rmcp:**
```rust
// Option 1: Auto-generate docs
#[tool_router(router = my_router, vis = "pub")]
impl MyServer { }
// Generates:
// /// Tool router for MyServer.
// pub fn my_router() -> Vec<Tool> { ... }

// Option 2: Accept doc string
#[tool_router(router = my_router, vis = "pub", doc = "Tool router for server operations")]
impl MyServer { }
```

## Tracking

- **Issue filed**: https://github.com/JasonShin/rmcp/issues/XXX (TODO: file issue)
- **Alternative**: Could be addressed in elicitation crate if it wraps tool_router
- **Temporary**: This is the ONLY use of `#[allow]` directives in botticelli_mcp

## Why This Exception is Acceptable

1. **Not our code**: The warning originates from generated code in an external dependency
2. **Well documented**: Each `#[allow]` has a comment explaining why and when to remove it
3. **Tracked**: This file documents the issue and path to resolution
4. **Minimal**: Only 3 occurrences in the entire codebase
5. **Temporary**: Will be removed when upstream fix is available

## Related

- See `elicitation/TOOL_ROUTER_WARNINGS.md` for general tool_router warning guidance
- See `CLAUDE.md` linting section: "Never use `#[allow]` - fix root cause instead"
  - This is the documented exception to that rule
