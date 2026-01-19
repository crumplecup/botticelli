# Trait Tooling Pattern

## The Problem

- `#[tool]` requires function bodies (generates MCP scaffolding)
- Trait methods are just signatures (no bodies)
- Trait implementations can't have `#[tool]` (visibility conflicts)

## The Solution: Tool Wrappers

Create thin wrapper tools in `botticelli_mcp` that delegate to trait implementations.

### Pattern

```rust
// In botticelli_interface: Pure trait (no rmcp dependency)
pub trait MediaStorage {
    async fn store(&self, data: &[u8], metadata: &Metadata) 
        -> Result<Reference, Error>;
}

// In botticelli_mcp: Tool wrapper for polymorphic access
impl BotticelliServer {
    /// Store media using configured storage backend
    #[tool]
    #[instrument(skip(self, params))]
    pub async fn storage_store(
        &self,
        Parameters(params): Parameters<StorageStoreParams>,
    ) -> Result<Json<StorageStoreResult>, rmcp::ErrorData> {
        // Delegate to trait implementation
        let reference = self.storage
            .store(params.data(), params.metadata())
            .await?;
        
        Ok(Json(StorageStoreResult { reference }))
    }
}
```

### Benefits

1. **Polymorphic**: Works with any `MediaStorage` implementation
2. **Clean separation**: Traits stay pure, tooling in MCP layer
3. **Type-safe**: Params/Results enforce contracts
4. **Observable**: Full instrumentation on wrapper
5. **Composable**: LLMs call one tool regardless of backend

### Implementation Strategy

For each trait in `_interface`:
1. Keep trait definition pure (no rmcp)
2. Create `{trait}_tools.rs` module in `_mcp`
3. Add tool wrapper for each trait method
4. Use Elicit on all Params/Results
5. Delegate to server's trait object

### Example: Complete Pattern

```rust
// Params/Result types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageStoreParams {
    data: Vec<u8>,
    metadata: MediaMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageStoreResult {
    reference: MediaReference,
}

// Tool wrapper
impl BotticelliServer {
    #[tool]
    #[instrument(skip(self, params))]
    pub async fn storage_store(
        &self,
        Parameters(params): Parameters<StorageStoreParams>,
    ) -> Result<Json<StorageStoreResult>, rmcp::ErrorData> {
        let reference = self.storage
            .store(params.data(), params.metadata())
            .await
            .map_err(|e| rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                e.to_string(),
                None,
            ))?;
        
        Ok(Json(StorageStoreResult { reference }))
    }
}
```

## Traits to Wrap

From `botticelli_interface`:
- MediaStorage (5 methods)
- NarrativeRepository (save, load, list, etc.)
- ContentRepository
- LlmSampler
- NarrativeElicitor
- TableQueryRegistry
- DatabaseRegistryOperations
- RegistryOperations
- And more...

## Naming Convention

Tool wrappers follow pattern: `{trait_name_snake_case}_{method_name}`

Examples:
- `media_storage_store`
- `media_storage_retrieve`
- `narrative_repository_save_execution`
- `llm_sampler_sample`

## Implementation Order

1. **High-value traits first**: MediaStorage, LlmSampler, NarrativeRepository
2. **CRUD patterns**: Consistent params for create/read/update/delete
3. **Complex workflows**: Break down into atomic tool operations
4. **Registry operations**: Generic patterns for resource management

## Result

LLMs get **polymorphic tool access** to all trait implementations through
a single, consistent MCP interface.
