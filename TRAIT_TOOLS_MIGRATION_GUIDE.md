# Trait Tools Migration Guide: Battle-Tested Patterns

**Status**: Phases 1 & 2 complete! This guide reflects real-world experience from converting McpResource and MediaStorage traits.

## Quick Start

**Phase 1 (McpResource)**: ✅ Complete - 1 method, registry pattern  
**Phase 2 (MediaStorage)**: ✅ Complete - 5 methods, generic types + proper error handling  
**Phase 3**: 🔄 Pending - Choose next trait

## Complete Conversion Checklist

Use this battle-tested checklist for each trait:

### 1. Planning & Analysis
- [ ] Identify trait and count methods
- [ ] Find all implementations: `rg "impl.*TraitName"`
- [ ] Find all call sites: `rg "\.method_name\("`
- [ ] Assess complexity:
  - Associated types?
  - Generic parameters?
  - Multiple implementations?
  - Call sites in multiple crates?

### 2. Interface Crate Changes

**File**: `crates/botticelli_interface/src/[trait_name].rs`

- [ ] Create wrapper parameter/result types
  - Use concrete types OR
  - Use generic types with type parameters
- [ ] Add serialization to associated types (if any):
  ```rust
  type Metadata: Serialize + for<'de> Deserialize<'de> + JsonSchema + Send + Sync;
  ```
- [ ] Add `#[async_trait]` to trait
- [ ] Convert methods to tool-native signatures:
  ```rust
  async fn method_name(
      &self,
      params: Parameters<MethodParams>,
  ) -> Result<Json<MethodResult>, ErrorData>;
  ```

**File**: `crates/botticelli_interface/src/lib.rs`

- [ ] Export wrapper types: `pub use module::{Params, Result};`

### 3. Error Handling (CRITICAL!)

**File**: `crates/botticelli_error/src/[domain].rs`

**DO NOT convert errors to strings!** Always preserve error chains:

- [ ] Add RmcpError variant to ErrorKind
- [ ] Implement `From<rmcp::ErrorData>`
- [ ] Bridge to umbrella error with `bridge_error!`

See "Error Handling Pattern" section below for complete code.

### 4-10. Implementation, Call Sites, Tool Module, etc.

See complete checklist in file.

## Error Handling Pattern (CRITICAL!)

**NEVER convert errors to strings**. Always preserve error chains:

```rust
// 1. Add variant to domain ErrorKind
#[derive(Debug, Clone, derive_more::Display)]
pub enum DatabaseErrorKind {
    #[display("MCP tool error: {} (code {})", message, code)]
    RmcpError {
        message: String,
        code: i32,
    },
}

// 2. Implement From for ErrorData
impl From<rmcp::ErrorData> for DatabaseError {
    fn from(err: rmcp::ErrorData) -> Self {
        DatabaseError::new(DatabaseErrorKind::RmcpError {
            message: err.message.into_owned(),  // Cow → String
            code: err.code.0,
        })
    }
}

// 3. Bridge to umbrella error
bridge_error!(rmcp::ErrorData => DatabaseError => crate::BotticelliErrorKind);

// 4. Use ? operator at call sites (clean!)
let result = storage.store(params).await?;
```

## Proven Patterns

### Pattern 1: Concrete Wrapper Types (Simple)

Use when no generics needed.

### Pattern 2: Generic Wrapper Types with Aliases (Complex)

**Critical**: Macro can't resolve generic type parameters!

```rust
// Interface - generic wrappers
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreParams<M, D> {
    pub data: D,
    pub metadata: M,
}

// Tool module - MUST create concrete aliases!
type StoreParams = botticelli_interface::StoreParams<MediaMetadata, Vec<u8>>;
```

### Pattern 3: Registry as Meta-Resource

Registry implements trait and routes to sub-resources internally.

## Common Pitfalls & Solutions

See full list in file for:
- Method shadowing
- Trait not in scope
- Generic types without aliases
- String error conversions (DON'T!)
- Missing router parameters
- Optional fields

## Real-World Examples

See completed conversions:
- Phase 1: `crates/botticelli_interface/src/mcp_resource.rs`
- Phase 2: `crates/botticelli_interface/src/media_storage.rs`
- Tools: `crates/botticelli_mcp/src/rmcp_server/tools/{resources,storage}.rs`

## Progress Tracker

| Phase | Trait | Methods | Status | Lines Saved |
|-------|-------|---------|--------|-------------|
| 1 | McpResource | 1 | ✅ | ~50 |
| 2 | MediaStorage | 5 | ✅ | ~200 |
| 3 | TBD | TBD | 🔄 | ~50-500 |

**Total**: 6 methods, ~250 lines eliminated, ~2750 remaining
