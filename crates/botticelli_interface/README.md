# botticelli_interface

Trait definitions and interfaces for the Botticelli ecosystem.

## Overview

This crate defines the core traits and interface types that enable different components of Botticelli to work together. It acts as the contract layer between implementations and consumers.

## Key Traits

### BotticelliDriver

The main trait for LLM provider implementations:

```rust
#[async_trait]
pub trait BotticelliDriver: Send + Sync {
    /// Generate a response from the LLM
    async fn generate(&self, request: GenerateRequest) 
        -> BotticelliResult<GenerateResponse>;
    
    /// Get the name of this driver
    fn name(&self) -> &str;
}
```

Implementations:
- `GeminiClient` (in `botticelli_models` crate)
- More providers coming soon

### NarrativeRepository

Trait for persisting narrative executions:

```rust
#[async_trait]
pub trait NarrativeRepository: Send + Sync {
    /// Save a narrative execution
    async fn save_execution(&self, execution: &NarrativeExecution) 
        -> BotticelliResult<i32>;
    
    /// Get an execution by ID
    async fn get_execution(&self, id: i32) 
        -> BotticelliResult<Option<NarrativeExecution>>;
    
    /// List executions with filtering
    async fn list_executions(&self, filter: &ExecutionFilter) 
        -> BotticelliResult<Vec<ExecutionSummary>>;
    
    /// Update execution status
    async fn update_status(&self, id: i32, status: ExecutionStatus) 
        -> BotticelliResult<()>;
    
    /// Delete an execution
    async fn delete_execution(&self, id: i32) 
        -> BotticelliResult<()>;
}
```

Implementations:
- `InMemoryNarrativeRepository` (in `botticelli_narrative`)
- `PostgresNarrativeRepository` (in `botticelli_database`)

## Narrative Types

Core types for narrative execution defined in the interface layer:

```rust
/// A narrative is a sequence of acts
pub struct Narrative {
    pub metadata: NarrativeMetadata,
    pub toc: NarrativeToc,
    pub acts: HashMap<String, Act>,
}

/// Metadata about a narrative
pub struct NarrativeMetadata {
    pub name: String,
    pub description: String,
    pub template: Option<String>,
    pub skip_content_generation: bool,
}

/// Result of executing a narrative
pub struct NarrativeExecution {
    pub narrative_name: String,
    pub act_executions: Vec<ActExecution>,
}

/// Result of executing a single act
pub struct ActExecution {
    pub act_name: String,
    pub model_used: String,
    pub prompt: String,
    pub response: String,
    pub finish_reason: FinishReason,
    pub error: Option<String>,
}
```

## Repository Types

Types for filtering and summarizing executions:

```rust
/// Filter criteria for listing executions
pub struct ExecutionFilter {
    pub narrative_name: Option<String>,
    pub status: Option<ExecutionStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Summary of an execution (lightweight)
pub struct ExecutionSummary {
    pub id: i32,
    pub narrative_name: String,
    pub narrative_description: Option<String>,
    pub status: ExecutionStatus,
    pub act_count: usize,
    pub error_message: Option<String>,
}

/// Execution status
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
}
```

## Usage

### Implementing a Driver

```rust
use botticelli_interface::{BotticelliDriver, GenerateRequest, GenerateResponse};
use botticelli_error::BotticelliResult;
use async_trait::async_trait;

pub struct MyDriver {
    api_key: String,
}

#[async_trait]
impl BotticelliDriver for MyDriver {
    async fn generate(&self, request: GenerateRequest) 
        -> BotticelliResult<GenerateResponse> 
    {
        // Implementation here
        todo!()
    }
    
    fn name(&self) -> &str {
        "my-provider"
    }
}
```

### Implementing a Repository

```rust
use botticelli_interface::{NarrativeRepository, NarrativeExecution};
use botticelli_error::BotticelliResult;
use async_trait::async_trait;

pub struct MyRepository;

#[async_trait]
impl NarrativeRepository for MyRepository {
    async fn save_execution(&self, execution: &NarrativeExecution) 
        -> BotticelliResult<i32> 
    {
        // Implementation here
        todo!()
    }
    
    // ... other methods
}
```

### Using a Driver

```rust
use botticelli_interface::BotticelliDriver;
use botticelli_core::{GenerateRequest, Input};

async fn example(driver: &dyn BotticelliDriver) {
    let request = GenerateRequest {
        inputs: vec![Input::Text("Hello!".to_string())],
        system_instruction: None,
        temperature: Some(0.7),
        max_output_tokens: Some(100),
        model: "gemini-1.5-flash".to_string(),
    };
    
    let response = driver.generate(request).await?;
    println!("Response: {}", response.text);
}
```

## Design Philosophy

### Interface Segregation

This crate contains only trait definitions and types they operate on - no implementations. This allows:
- Clear separation of concerns
- Easier testing with mocks
- Multiple implementations of the same trait
- Avoid circular dependencies

### Async by Default

All trait methods are async using `async-trait` to support non-blocking I/O operations.

### Send + Sync

All traits require `Send + Sync` bounds to ensure thread-safe usage in async contexts.

## Dependencies

- `botticelli_error` - Error types
- `botticelli_core` - Core data structures
- `async-trait` - Async trait support
- `serde` - Serialization
- `chrono` - DateTime types

## Version

Current version: 0.2.0

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
