# Trait Tools Summary: Problem → Solution → Implementation

## Executive Summary

**Phase 1 (McpResource)** and **Phase 2 (MediaStorage)** are complete! We've successfully converted 2 traits (6 methods total) using `#[elicit_trait_tools_router]`, eliminating ~250 lines of wrapper boilerplate and establishing proven patterns for future conversions.

## The Problem (What We Discovered)

You have **50+ trait methods** across multiple interfaces in botticelli that you wanted to expose as MCP tools. The `#[tool]` macro can't be applied to trait implementation methods due to Rust's trait system constraints.

### Why `#[tool]` Fails on Trait Impls

```rust
impl EventHandler for DiscordHandler {
    #[tool]  // ❌ ERROR: Macro modifies signature
    async fn process_event(&self, event: Event) -> Result<()> {
        // Trait impls must exactly match trait signature
    }
}
```

**Error**: `E0407: method not member of trait` (macro changes signature)

### The Boilerplate Problem

To expose trait methods as tools, you needed **manual wrapper functions**:

- **Per method**: ~15-20 lines of wrapper code
- **Your scale**: 50+ trait methods
- **Total**: ~3000 lines of boilerplate
- **Maintenance**: Update trait → update wrapper (2× work)

## The Solution (What Elicitation Delivered)

### ✅ **Fully Implemented in elicitation 0.6.10**

The `#[elicit_trait_tools_router]` macro does exactly what we proposed! Version 0.6.10 adds support for `#[async_trait]` to maintain object safety.

### Syntax

```rust
#[elicit_trait_tools_router(TraitName, field_name, [method1, method2, ...])]
#[tool_router(router = trait_tool_router, vis = "pub")]  // Explicit params required!
impl MyServer {
    // Auto-generated wrappers for all listed trait methods
}
```

### Requirements

**Trait methods must use MCP-compatible signatures.** Two patterns supported:

**Pattern 1: `impl Future + Send` (zero-cost)**
```rust
fn method_name(
    &self,
    params: Parameters<MethodParams>,
) -> impl Future<Output = Result<Json<MethodResult>, rmcp::ErrorData>> + Send;
```

**Pattern 2: `#[async_trait]` (object-safe)** ← **We use this**
```rust
#[async_trait::async_trait]
trait MyTrait: Send + Sync {
    async fn method_name(
        &self,
        params: Parameters<MethodParams>,
    ) -> Result<Json<MethodResult>, rmcp::ErrorData>;
}
```

**Use `#[async_trait]` when:**
- You need trait objects (`Box<dyn Trait>`, `Arc<dyn Trait>`)
- Dynamic dispatch required (registries, plugins, polymorphism)
- Simpler syntax preferred (`async fn` vs `impl Future`)

**We chose Pattern 2 for all conversions** - object safety is required for registries.

## Real-World Implementation (Phases 1 & 2)

### ✅ Phase 1: McpResource (Complete)

**Converted**: 1 method (`read`) across 2 implementations

**Key pattern**: Registry as meta-resource
```rust
// Trait
#[async_trait]
pub trait McpResource: Send + Sync {
    async fn read(
        &self,
        params: Parameters<ReadParams>,
    ) -> Result<Json<ReadResult>, ErrorData>;
}

// ResourceRegistry implements the trait and routes internally
impl McpResource for ResourceRegistry {
    async fn read(&self, params: Parameters<ReadParams>) -> ... {
        for resource in &self.resources {
            if resource.matches_uri(&params.0.uri) {
                return resource.read(params).await;  // Delegate
            }
        }
        Err(not_found_error())
    }
}

// Tool generation
#[elicit_trait_tools_router(McpResource, resource_registry, [read])]
#[tool_router(router = resources_tool_router, vis = "pub")]
impl BotticelliServer {}
```

**Lessons**:
- ✅ Method shadowing: Remove convenience methods that duplicate trait method names
- ✅ Trait scope: Trait must be imported in tool module
- ✅ Field visibility: Use `pub(crate)` not private (macro needs direct access)
- ✅ Registry pattern: Implement trait on registry for unified tool interface

### ✅ Phase 2: MediaStorage (Complete)

**Converted**: 5 methods (`store`, `retrieve`, `get_url`, `delete`, `exists`) with generic associated types

**Key pattern**: Generic wrappers with concrete type aliases
```rust
// Interface: Generic wrapper types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreParams<M, D> {
    pub data: D,
    pub metadata: M,
}

// Trait: Use Self::AssociatedType
#[async_trait]
pub trait MediaStorage {
    type Metadata: Serialize + ...;
    type Reference: Serialize + ...;
    
    async fn store(
        &self,
        params: Parameters<StoreParams<Self::Metadata, Vec<u8>>>,
    ) -> Result<Json<StoreResult<Self::Reference>>, ErrorData>;
}

// Tool module: Concrete type aliases (CRITICAL!)
type StoreParams = botticelli_interface::StoreParams<MediaMetadata, Vec<u8>>;
type StoreResult = botticelli_interface::StoreResult<MediaReference>;

// Tool generation
#[elicit_trait_tools_router(MediaStorage, storage, [store, retrieve, get_url, delete, exists])]
#[tool_router(router = storage_tool_router, vis = "pub")]
impl BotticelliServer {}
```

**Lessons**:
- ✅ Generic types: Macro needs concrete type aliases, can't resolve generics
- ✅ Error chains: Add proper error variants (RmcpError), not string conversions
- ✅ Error bridging: Use `From` impl + `bridge_error!` macro for error propagation
- ✅ ErrorData.message: Is `Cow<'static, str>`, use `.into_owned()` for String
- ✅ Router params: Must specify `router = name_tool_router, vis = "pub"` explicitly
- ✅ Non-optional fields: Macro can't call methods on `Option<T>`, use defaults

### Error Handling Pattern (Critical!)

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
bridge_error!(rmcp::ErrorData => DatabaseError => BotticelliErrorKind);

// 4. Use ? operator at call sites (clean!)
let result = storage.store(params).await?;
```

## Progress Tracker

| Phase | Trait | Methods | Status | Lines Saved |
|-------|-------|---------|--------|-------------|
| 1 | McpResource | 1 (read) | ✅ Complete | ~50 |
| 2 | MediaStorage | 5 (store, retrieve, etc) | ✅ Complete | ~200 |
| 3 | TBD | TBD | 🔄 Planned | ~50-500 |

**Total so far**: 6 methods converted, ~250 lines eliminated

**Remaining**: ~2750 lines across 44+ methods

## Complete Checklist (Battle-Tested)

Use this for each trait conversion:

### Planning
- [ ] Identify trait and count methods
- [ ] Find all implementations (grep for `impl TraitName`)
- [ ] Find all call sites (grep for trait methods)
- [ ] Assess complexity (generics? associated types?)

### Interface Changes
- [ ] Create wrapper types (concrete or generic with type params)
- [ ] Add serialization bounds to associated types if needed
- [ ] Update trait to tool-native signatures with `#[async_trait]`
- [ ] Export wrapper types from lib.rs

### Implementation Updates
- [ ] Update all trait implementations to use Parameters/Json
- [ ] Extract values from `Parameters` wrapper
- [ ] Wrap results in `Json`
- [ ] Convert errors to `ErrorData`

### Error Handling (CRITICAL!)
- [ ] **Add RmcpError variant** to domain ErrorKind (NOT string conversion!)
- [ ] **Implement From<ErrorData>** for domain error
- [ ] **Bridge to umbrella error** with `bridge_error!` macro
- [ ] Test error propagation with `?` operator

### Call Site Updates
- [ ] Update all call sites in dependency crates
- [ ] Construct Parameters wrappers
- [ ] Extract from Json results
- [ ] Verify error handling works

### Tool Module
- [ ] **Create type aliases** for generic wrappers (if needed)
- [ ] Create tools/[name].rs with imports
- [ ] Import trait (must be in scope!)
- [ ] Import wrapper types
- [ ] Apply `#[elicit_trait_tools_router]` macro
- [ ] **Specify explicit router parameters**: `router = name_tool_router, vis = "pub"`
- [ ] Remove old manual tool wrappers
- [ ] Remove old parameter/result type exports
- [ ] Add router to `create_tool_router()`

### Validation
- [ ] `cargo check --all-features -p botticelli_mcp -p [dependencies]`
- [ ] Fix all compilation errors
- [ ] Fix all warnings (unused imports, etc.)
- [ ] Verify zero errors, zero warnings

### Documentation
- [ ] Document lessons learned immediately
- [ ] Update plan.md with progress
- [ ] Update this file with patterns discovered
- [ ] Commit with detailed message

## Anti-Patterns (Do NOT Do These!)

❌ **String error conversions**: `.map_err(|e| e.to_string())`
✅ **Proper error types**: Add variant + From impl + bridge

❌ **Direct generic imports**: `use interface::StoreParams;` (won't compile)
✅ **Type aliases**: `type StoreParams = interface::StoreParams<Concrete, Types>;`

❌ **Implicit router params**: `#[tool_router]`
✅ **Explicit router params**: `#[tool_router(router = name_tool_router, vis = "pub")]`

❌ **Optional fields**: `storage: Option<Arc<T>>`  
✅ **Non-optional with default**: `storage: Arc<T>` with `#[builder(default = "...")]`

❌ **Trait not in scope**: Macro can't find methods
✅ **Import trait**: `use botticelli_interface::TraitName;`

## Next Steps

**Phase 3 candidates** (ordered by simplicity):

1. **BotCommandRegistry** - 1 method, 2 implementations, simple
2. **ActProcessor** - 1 method, multiple implementations  
3. **BotticelliDriver** - 2 methods, high value (~500 lines saved)

Choose based on team capacity and risk tolerance.
- Dynamic dispatch is required (registries, plugins)
- Simpler syntax preferred over performance

**Use `impl Future + Send` when:**
- No trait objects needed
- Want zero-cost abstractions
- Maximum performance required

### Example: Before & After

**Before (Manual - 150+ lines)**:
```rust
#[tool(description = "Generate text using Gemini")]
pub async fn gemini_generate(params: GeminiGenerateParams) -> Result<GenerateResponse> {
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    let client = GeminiClient::new()?;
    Ok(client.generate(&request).await?)
}

#[tool(description = "Generate text using Anthropic")]
pub async fn anthropic_generate(params: AnthropicGenerateParams) -> Result<GenerateResponse> {
    let request: GenerateRequest = serde_json::from_value(params.request)?;
    let client = AnthropicClient::new()?;
    Ok(client.generate(&request).await?)
}

// ... 8 more nearly identical wrappers
```

**After (Automatic - 10 lines)**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateParams {
    pub request: GenerateRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateResult {
    pub response: GenerateResponse,
}

#[elicit_trait_tools]
pub trait BotticelliDriver: Send + Sync {
    fn generate(
        &self,
        params: Parameters<GenerateParams>,
    ) -> impl Future<Output = Result<Json<GenerateResult>, rmcp::ErrorData>> + Send;
}

// In MCP server
#[elicit_trait_tools_router(BotticelliDriver, gemini, [generate])]
#[elicit_trait_tools_router(BotticelliDriver, anthropic, [generate])]
#[tool_router]
impl BotticelliServer {
    // All provider generate methods auto-exposed as tools!
}
```

**Impact**: 150 lines → 10 lines (93% reduction)

## What You Need to Do

### 1. Read the Docs ✅

- **`~/repos/elicitation/ELICIT_TRAIT_TOOLS_ROUTER.md`**: Complete usage guide
- **`~/repos/elicitation/BOTTICELLI_INTEGRATION.md`**: Integration patterns
- **`TRAIT_TOOLS_MIGRATION_GUIDE.md`** (this repo): Step-by-step migration

### 2. Update Trait Signatures

Convert traits to MCP-compatible format:

**Pattern**:
```rust
// Old
async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error>;

// New (tool-native)
fn generate(
    &self,
    params: Parameters<GenerateParams>,
) -> impl Future<Output = Result<Json<GenerateResult>, rmcp::ErrorData>> + Send;
```

**Implementation uses `async move`**:
```rust
impl BotticelliDriver for GeminiClient {
    fn generate(
        &self,
        params: Parameters<GenerateParams>,
    ) -> impl Future<Output = Result<Json<GenerateResult>, rmcp::ErrorData>> + Send {
        let client = self.clone();
        let request = params.0.request;
        
        async move {
            let response = client.generate_internal(&request).await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
            
            Ok(Json(GenerateResult { response }))
        }
    }
}
```

### 3. Replace Manual Wrappers

Remove ~3000 lines of wrapper code, replace with macro invocations:

```rust
use elicitation::elicit_trait_tools_router;

#[elicit_trait_tools_router(BotticelliDriver, provider, [generate])]
#[tool_router]
impl BotticelliServer {
    // Wrappers auto-generated!
}
```

## Migration Priority

**High Priority** (Most Benefit):
1. ✅ **BotticelliDriver** (5 providers × 2 methods = 10 tools)
2. ✅ **BotCommandRegistry** (platform commands)
3. ✅ **EventHandler** (Discord events, 20+ methods)

**Medium Priority**:
4. **ActProcessor** (narrative processing)
5. **CommandValidator** (security layer)

**Keep Manual** (Not Tool-Compatible):
- Streaming methods (return `impl Stream`)
- Methods with complex lifetimes
- Internal-only methods

## Expected Impact

### Code Reduction
- **Current**: ~3000 lines of wrapper code
- **After**: ~150 lines of declarations
- **Savings**: ~95% reduction

### Maintainability
- **Before**: Update trait → update wrapper (2 places, easy to get out of sync)
- **After**: Update trait → update method list in macro (1 place)

### Type Safety
- **Before**: Easy to mistype wrapper signatures
- **After**: Compiler ensures trait signatures match tool requirements

## Status

- ✅ **Elicitation 0.6.9**: Macro implemented and documented
- ✅ **Botticelli workspace**: Elicitation 0.6.9 dependency added
- ✅ **Documentation**: Complete migration guide created
- 🔄 **Next**: Start pilot migration with `BotticelliDriver` trait

## Key Documents

1. **`ELICITATION_TRAIT_TOOLS_PROPOSAL.md`**: Original proposal (for reference)
2. **`ELICITATION_TRAIT_TOOLS_STATUS.md`**: Implementation status
3. **`TRAIT_TOOLS_MIGRATION_GUIDE.md`**: Step-by-step migration instructions
4. **`~/repos/elicitation/ELICIT_TRAIT_TOOLS_ROUTER.md`**: Official macro documentation

## Quick Start

1. Read `~/repos/elicitation/ELICIT_TRAIT_TOOLS_ROUTER.md`
2. Pick one trait to migrate (suggest: `BotticelliDriver`)
3. Follow `TRAIT_TOOLS_MIGRATION_GUIDE.md` steps
4. Test with MCP client
5. Iterate to other traits

**Timeline**: Can start immediately (all dependencies ready)

## Success Criteria

- ✅ Trait methods exposed as MCP tools
- ✅ ~95% reduction in boilerplate code
- ✅ Single source of truth (trait methods)
- ✅ Type-safe tool generation
- ✅ Maintainable (add method → update list → done)

---

**Bottom Line**: Your vision of "tool everything" is now achievable! The macro exists, it's documented, and it's ready to use. The only remaining work is migrating your trait signatures to the MCP-compatible format.
