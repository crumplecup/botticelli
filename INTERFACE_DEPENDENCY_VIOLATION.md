# Interface Dependency Violation Analysis

## Problem

`botticelli_interface` is supposed to be a foundational crate defining contracts (traits). Instead, it depends on concrete types from:
- `botticelli_core` (GenerateRequest, GenerateResponse, Input, Message, etc.)
- `botticelli_storage` (MediaMetadata, MediaReference)
- `botticelli_rate_limit` (RateLimitError, RateLimitConfig)
- `botticelli_error` (various Result types)

This creates circular dependency pressure and violates the principle that interfaces should be abstract.

## Root Cause: Traits Reference Concrete Types

Traits in `_interface` directly reference concrete types from other crates:

```rust
// ❌ BAD: Trait references concrete types
pub trait Provider: Send + Sync {
    async fn generate(
        &self, 
        request: GenerateRequest  // Concrete type from botticelli_core
    ) -> Result<GenerateResponse, ...>;
}

pub trait NarrativeRepository {
    fn store_media(
        &self,
        metadata: MediaMetadata  // Concrete type from botticelli_storage
    ) -> Result<MediaReference, ...>;
}
```

**The Key Insight:** Traits should not reference specific types - that's what causes dependency coupling. If a trait needs a specific type concept, you abstract it to an associated type. The concrete type is chosen when implementing the trait in a downstream crate.

## Solution: Use Associated Types

Traits should be abstract and let implementers choose concrete types:

```rust
// ✅ GOOD: Trait uses associated types
pub trait Provider: Send + Sync {
    type Request;
    type Response;
    type Error;
    
    async fn generate(
        &self, 
        request: Self::Request
    ) -> Result<Self::Response, Self::Error>;
}

// Implementation in downstream crate (e.g., botticelli_gemini)
impl Provider for GeminiProvider {
    type Request = GenerateRequest;   // Concrete type chosen here
    type Response = GenerateResponse;
    type Error = GeminiError;
    
    async fn generate(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        // Implementation uses the concrete types
    }
}
```

## Violations Found

### 1. Provider trait (provider.rs)
- Uses `GenerateRequest` and `GenerateResponse` from `botticelli_core`
- **Fix**: Use associated types `type Request; type Response; type Error;`

### 2. ChatService trait (chat_service.rs)  
- Uses `GenerateRequest`, `Message`, `ToolDefinition` from `botticelli_core`
- **Fix**: Use associated types for all concrete parameters

### 3. ChatHost trait (chat_host.rs)
- Uses `ToolDefinition` from `botticelli_core`
- Uses `ChatResult` from `botticelli_error`
- **Fix**: Associated types for tools and errors

### 4. BotticelliDriver trait (traits.rs)
- Uses `GenerateRequest`, `GenerateResponse`, `Input`, `ToolDefinition` from `botticelli_core`
- Uses `RateLimitConfig` from `botticelli_rate_limit`
- **Fix**: Associated types for ALL concrete types

### 5. ToolCalling trait (traits.rs)
- Uses concrete types from `botticelli_core`
- **Fix**: Associated types

### 6. NarrativeRepository trait (narrative/repository.rs)
- Uses `MediaMetadata`, `MediaReference` from `botticelli_storage`
- Uses `BotticelliResult` from `botticelli_error`
- **Fix**: Associated types `type Metadata; type Reference; type Error;`

### 7. NarrativeExecutor trait (narrative/execution.rs)
- Uses `Input`, `TokenUsageData` from `botticelli_core`
- **Fix**: Associated types for input and usage data

### 8. Registry traits (registry_traits.rs)
- Uses `BotticelliResult` from `botticelli_error`
- **Fix**: Associated type `type Error;` for all error returns

### 9. RetryableError trait (provider.rs)
- Has implementation for `RateLimitError` in same crate
- **Fix**: Keep trait abstract, move implementation to `_rate_limit` crate

## Alternative: Strategic Trait Placement

Not all traits belong in `_interface`. Consider:

1. **Truly abstract/reusable traits** → Keep in `_interface` with associated types
2. **Tightly coupled to specific types** → Move to the crate where types live
3. **Only used in one place** → Consider removing abstraction entirely

Example decisions:
- `Provider` trait is generic and reusable → Keep in `_interface` with associated types
- `ChatService` trait is tightly coupled to core chat types → Consider moving to `_core`
- `NarrativeRepository` is abstract storage contract → Keep in `_interface` with associated types

## Implementation Strategy

### Pattern 1: Pure Associated Types (Recommended)

```rust
// botticelli_interface/src/provider.rs
// NO concrete type imports!

pub trait Provider: Send + Sync {
    type Request;
    type Response;
    type Error: std::error::Error;
    
    async fn generate(&self, req: Self::Request) 
        -> Result<Self::Response, Self::Error>;
}

// botticelli_gemini/src/provider.rs
use botticelli_interface::Provider;
use botticelli_core::{GenerateRequest, GenerateResponse};
use crate::GeminiError;

impl Provider for GeminiProvider {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = GeminiError;
    
    async fn generate(&self, req: Self::Request) 
        -> Result<Self::Response, Self::Error> {
        // Implementation
    }
}
```

### Pattern 2: Trait Objects for Flexibility

When you need trait methods to work with many types:

```rust
pub trait NarrativeRepository: Send + Sync {
    type Metadata: serde::Serialize + serde::DeserializeOwned;
    type Reference: Clone + Send;
    
    async fn store_media(&self, data: &[u8], metadata: &Self::Metadata) 
        -> Result<Self::Reference, Box<dyn std::error::Error>>;
}
```

### Pattern 3: Move Trait to Type's Crate

If a trait is fundamentally about specific types, move it there:

```rust
// Move ChatService trait from _interface to _core
// It's really about Message, ToolDefinition, etc.
```

## Implementation Plan

### Phase 1: Audit and Categorize (CURRENT TASK)
For each trait in `_interface`:
1. ✅ List all concrete type references
2. ✅ Determine if trait should stay (abstract) or move (coupled)
3. Document migration strategy

### Phase 2: Convert to Associated Types (2-4 hours)
For traits staying in `_interface`:
1. Replace concrete types with associated types
2. Add trait bounds where needed (Error, Clone, Send, etc.)
3. Update trait documentation with associated type docs
4. **Do NOT implement traits yet** - just define them

### Phase 3: Fix Implementations (2-4 hours)
In downstream crates (gemini, anthropic, storage, etc.):
1. Add associated type declarations to impl blocks
2. Verify implementations compile
3. Update any calling code that breaks

### Phase 4: Move Coupled Traits (1-2 hours)
For traits moving to other crates:
1. Move trait definition to appropriate crate
2. Update imports across workspace
3. Update re-exports in lib.rs

### Phase 5: Clean Dependencies (30 min)
1. Remove unused dependencies from `_interface/Cargo.toml`
2. Verify: `cargo check --package botticelli_interface`
3. Verify dependency graph: `cargo tree -i botticelli_interface`

### Phase 6: Update Documentation (30 min)
1. Update CLAUDE.md with trait placement rules
2. Add to AUDIT_CHEATSHEET.md
3. Document the associated types pattern

## Success Criteria

- ✅ `botticelli_interface/Cargo.toml` only has minimal external dependencies
- ✅ `cargo tree -i botticelli_interface` shows other crates depending ON it (not vice versa)
- ✅ All traits compile with associated types
- ✅ All existing implementations work (implementations choose concrete types)
- ✅ No concrete workspace type imports in trait definitions

## Key Principle

**Traits in `_interface` must be abstract. They define contracts with associated types. Implementations in downstream crates choose concrete types.**

Don't think: "This trait needs GenerateRequest" → import it
Think: "This trait needs some Request type" → use associated type

The implementation later says: "My Request type is GenerateRequest"

## Common Mistakes to Avoid

1. ❌ Importing concrete types into trait definitions
2. ❌ Implementing traits in `_interface` for types from other crates
3. ❌ Using `Box<dyn Error>` when you can use associated `type Error`
4. ❌ Over-abstracting simple, tightly-coupled traits
5. ❌ Forgetting trait bounds on associated types

## Testing Strategy

After each phase:
1. `cargo check --package botticelli_interface` (must compile)
2. `cargo check --workspace` (all implementations compile)
3. `cargo test --workspace` (tests still pass)
4. `cargo tree -p botticelli_interface` (verify dependencies)

## Timeline Estimate

- Phase 1 (Audit): ✅ Complete
- Phase 2 (Convert): 2-4 hours
- Phase 3 (Implementations): 2-4 hours  
- Phase 4 (Move): 1-2 hours
- Phase 5 (Clean): 30 min
- Phase 6 (Docs): 30 min

**Total: 6-11 hours** (can be done incrementally)
