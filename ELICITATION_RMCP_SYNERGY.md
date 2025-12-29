# Elicitation + RMCP: Complementary Architecture Analysis

**Date:** 2024-12-29  
**Status:** Analysis - Architectural Synergy Assessment

## Executive Summary

The elicitation derive macros and rmcp tool macros are **strongly complementary**, not dissonant or orthogonal. They operate at different abstraction layers and solve different problems that naturally compose:

- **Elicitation**: *User interaction layer* - How to collect typed data conversationally
- **RMCP**: *Protocol integration layer* - How to expose functions as MCP tools

**Synergy Thesis:** Elicitation types become rmcp tool parameters, creating a type-safe pipeline from conversational elicitation to structured tool invocation.

## Architectural Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Client (Claude)                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ JSON-RPC (MCP Protocol)
┌───────────────────────────▼─────────────────────────────────┐
│              RMCP Layer (Tool Protocol)                     │
│  #[tool_router]                                             │
│    #[tool] async fn create_narrative(                       │
│        Parameters(params): Parameters<CreateNarrativeParams>│
│    )                                                        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Type-safe parameters
┌───────────────────────────▼─────────────────────────────────┐
│         Elicitation Layer (User Interaction)                │
│  #[derive(Elicit)]                                          │
│  struct CreateNarrativeParams {                             │
│      #[prompt("Enter narrative name:")]                     │
│      name: String,                                          │
│      approach: ActApproach,  // enum → Select paradigm      │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
```

## Layer Responsibilities

### RMCP Layer (Protocol Boundary)

**Purpose:** Define the tool interface exposed to MCP clients

**Concerns:**
- Tool discovery (`list_tools`)
- Tool invocation routing
- Parameter deserialization from JSON
- Result serialization to JSON
- Error handling at protocol level
- Schema generation from types

**Macro:** `#[tool]` / `#[tool_router]`

**Example:**
```rust
#[tool_router]
impl BotticelliServer {
    /// Create a new narrative through guided elicitation
    #[tool(description = "Interactive narrative creation wizard")]
    async fn create_narrative(
        &self,
        // RMCP deserializes JSON → CreateNarrativeParams
        Parameters(params): Parameters<CreateNarrativeParams>
    ) -> Result<Json<NarrativeResult>, String> {
        // Call elicitation layer to gather detailed data
        let metadata = NarrativeMetadata::elicit(&self.mcp_client).await?;
        
        // Business logic
        self.create_from_metadata(metadata).await
    }
}
```

### Elicitation Layer (User Interaction)

**Purpose:** Define how to conversationally gather typed data

**Concerns:**
- Interaction paradigms (Select, Affirm, Survey)
- Field-by-field elicitation
- Validation and retry logic
- Prompt customization
- State machine for multi-field forms
- Type composition (nested structs, enums in structs)

**Macro:** `#[derive(Elicit)]`

**Example:**
```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
#[prompt("Let's create a new narrative!")]
pub struct NarrativeMetadata {
    /// Narrative name (alphanumeric and underscores only).
    #[prompt("Enter narrative name (alphanumeric and underscores):")]
    pub name: String,
    
    /// How should we create the acts?
    pub approach: ActApproach,  // Enum automatically uses Select
    
    /// Optional description
    #[prompt("Describe your narrative (optional):")]
    pub description: Option<String>,
}

#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
enum ActApproach {
    /// Auto-extract from description
    AutoExtract,
    /// Manually specify count
    CountBased,
    /// Interactive creation
    Interactive,
}
```

## How They Compose

### Pattern 1: Tool Parameters Are Elicitation Types

```rust
// Domain type with elicitation behavior
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
struct TaskConfig {
    #[prompt("Task title:")]
    title: String,
    
    priority: Priority,  // Enum → Select paradigm
    
    #[prompt("Due date (optional):")]
    due_date: Option<String>,
}

// RMCP exposes it as a tool parameter
#[tool_router]
impl BotticelliServer {
    #[tool]
    async fn create_task(
        &self,
        Parameters(config): Parameters<TaskConfig>
    ) -> Result<Json<Task>, String> {
        // Option A: Client provides full JSON
        // { "title": "...", "priority": "High", "due_date": null }
        
        // Option B: Tool internally elicits missing/complex data
        let refined_config = if config.needs_refinement() {
            TaskConfig::elicit(&self.mcp_client).await?
        } else {
            config
        };
        
        self.create_task_from_config(refined_config).await
    }
}
```

### Pattern 2: Multi-Stage Elicitation

Tools can orchestrate complex elicitation flows:

```rust
#[tool_router]
impl BotticelliServer {
    /// High-level narrative creation tool
    #[tool]
    async fn create_narrative_wizard(
        &self,
        Parameters(params): Parameters<InitialParams>
    ) -> Result<Json<Narrative>, String> {
        // Stage 1: Elicit metadata
        let metadata = NarrativeMetadata::elicit(&self.mcp_client).await?;
        
        // Stage 2: Based on approach, elicit acts
        let acts = match metadata.approach {
            ActApproach::AutoExtract => {
                self.extract_acts_from_description(&metadata.description).await?
            }
            ActApproach::CountBased => {
                let count: i64 = i64::elicit(&self.mcp_client).await?;
                self.elicit_acts_by_count(count).await?
            }
            ActApproach::Interactive => {
                Vec::<ActDefinition>::elicit(&self.mcp_client).await?
            }
        };
        
        // Stage 3: Elicit inputs for each act
        let mut full_acts = Vec::new();
        for act in acts {
            let inputs = Vec::<InputConfig>::elicit(&self.mcp_client).await?;
            full_acts.push(Act { definition: act, inputs });
        }
        
        // Build final narrative
        Ok(Json(Narrative::from_parts(metadata, full_acts)))
    }
}
```

### Pattern 3: Validation Through Types

Both systems provide validation, but at different points:

```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
pub struct Port {
    #[prompt("Enter port number (1-65535):")]
    #[schemars(range(min = 1, max = 65535))]  // RMCP JSON schema validation
    pub value: u16,  // Elicitation validates u16 range
}

#[tool_router]
impl BotticelliServer {
    #[tool]
    async fn configure_server(
        &self,
        Parameters(config): Parameters<ServerConfig>
    ) -> Result<Json<Server>, String> {
        // Port already validated by:
        // 1. RMCP: JSON schema checked u16 range
        // 2. Elicitation: Conversational validation with retry
        
        // Business logic can trust the type
        self.bind_server(config.port.value).await
    }
}
```

## Key Synergies

### 1. Type Safety Across Boundaries

**Without Composition:**
```rust
// RMCP tool accepts loosely-typed JSON
#[tool]
async fn create_thing(&self, Parameters(params): Parameters<Value>) -> ... {
    // Manual JSON extraction
    let name = params.get("name").and_then(|v| v.as_str())?;
    
    // Manual validation
    if name.is_empty() { return Err("Invalid name"); }
    
    // Manual sub-elicitation
    let approach = self.ask_user_for_approach().await?;
    // ...
}
```

**With Composition:**
```rust
// Strong types all the way down
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
struct ThingParams {
    #[prompt("Enter name:")]
    name: String,
    approach: Approach,  // Enum auto-validated
}

#[tool]
async fn create_thing(
    &self, 
    Parameters(params): Parameters<ThingParams>
) -> ... {
    // params.name is guaranteed valid String
    // params.approach is guaranteed valid enum variant
    // No manual validation needed
}
```

### 2. Declarative Configuration

Both systems use attributes for configuration:

```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
#[prompt("Configure your deployment:")]  // Elicitation: conversation context
#[schemars(title = "Deployment Config")]  // RMCP: MCP tool metadata
pub struct DeploymentConfig {
    /// Target environment
    #[prompt("Select environment:")]  // Elicitation: field prompt
    #[schemars(description = "Deployment target")]  // RMCP: schema description
    pub env: Environment,
    
    /// Number of replicas
    #[prompt("How many replicas?")]
    #[schemars(range(min = 1, max = 100))]
    pub replicas: u32,
}
```

### 3. Composable Elicitation

Elicitation types compose naturally, and RMCP tools can orchestrate them:

```rust
// Compose complex types
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
struct DatabaseConfig {
    connection: ConnectionParams,  // Nested struct
    auth: AuthMethod,  // Enum
    pools: Vec<PoolConfig>,  // Collection of structs
    readonly: Option<bool>,  // Optional
}

// RMCP tool exposes the composition
#[tool_router]
impl BotticelliServer {
    #[tool]
    async fn configure_database(
        &self,
        Parameters(config): Parameters<DatabaseConfig>
    ) -> Result<Json<Database>, String> {
        // All nested elicitation happens automatically
        // - ConnectionParams fields elicited via Survey
        // - AuthMethod chosen via Select
        // - Each PoolConfig in Vec elicited via Survey
        // - Optional bool uses Affirm ("Include readonly replica?")
        
        Database::from_config(config).await
    }
}
```

### 4. Error Handling Alignment

Both systems use `Result` types with structured errors:

```rust
// Elicitation errors
pub enum ElicitError {
    InvalidInput(String),
    ToolCallFailed(PmcpError),
    Cancelled,
}

// RMCP tool errors
#[tool]
async fn my_tool(...) -> Result<Json<T>, String> {
    // Convert elicitation errors to tool errors
    let data = MyType::elicit(&client).await
        .map_err(|e| format!("Elicitation failed: {}", e))?;
    
    Ok(Json(data))
}
```

## Architectural Benefits

### 1. Separation of Concerns

- **RMCP**: "What tools exist and how to call them"
- **Elicitation**: "How to gather data for those tools"

Neither needs to know implementation details of the other.

### 2. Reusability

Types with `#[derive(Elicit)]` can be:
- Used as RMCP tool parameters
- Elicited standalone in non-MCP contexts
- Composed into larger types
- Serialized/deserialized independently

```rust
// Use in RMCP tool
#[tool]
async fn tool1(&self, Parameters(p): Parameters<MyType>) -> ... { }

// Use standalone
let my_type = MyType::elicit(&client).await?;

// Nest in other types
#[derive(Elicit)]
struct Container {
    field: MyType,  // Automatically elicited as part of Survey
}
```

### 3. Progressive Disclosure

RMCP tools can start simple and add elicitation complexity:

```rust
// Phase 1: Simple tool with basic params
#[tool]
async fn create_thing(
    &self,
    Parameters(params): Parameters<SimpleParams>
) -> Result<Json<Thing>, String> {
    Thing::create_simple(params).await
}

// Phase 2: Add guided elicitation for complex cases
#[tool]
async fn create_thing_guided(
    &self,
    Parameters(params): Parameters<SimpleParams>
) -> Result<Json<Thing>, String> {
    // Use elicitation for complex sub-configuration
    let advanced = AdvancedConfig::elicit(&self.mcp_client).await?;
    Thing::create_advanced(params, advanced).await
}
```

### 4. Testing Simplification

Mock each layer independently:

```rust
// Test elicitation logic without RMCP
#[tokio::test]
async fn test_metadata_elicitation() {
    let mock_client = MockMcpClient::new();
    let metadata = NarrativeMetadata::elicit(&mock_client).await?;
    assert_eq!(metadata.name, "test");
}

// Test RMCP tool logic without elicitation
#[tokio::test]
async fn test_create_narrative_tool() {
    let server = BotticelliServer::new();
    let params = CreateNarrativeParams { /* ... */ };
    let result = server.create_narrative(Parameters(params)).await?;
    assert!(result.is_ok());
}
```

## Pattern Recommendations

### 1. Domain Types with Dual Derives

Always derive both for types that cross boundaries:

```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyDomainType {
    // Fields...
}
```

**Benefits:**
- RMCP: JSON schema generation, serialization
- Elicitation: Conversational gathering
- Both: Type safety, validation

### 2. Tool Orchestration Pattern

Use RMCP tools to orchestrate multi-step elicitation:

```rust
#[tool_router]
impl BotticelliServer {
    /// High-level wizard tool
    #[tool]
    async fn wizard(&self, ...) -> ... {
        // Step 1: Elicit configuration
        let config = Config::elicit(&self.mcp_client).await?;
        
        // Step 2: Based on config, elicit details
        let details = match config.mode {
            Mode::Simple => SimpleDetails::elicit(&self.mcp_client).await?,
            Mode::Advanced => AdvancedDetails::elicit(&self.mcp_client).await?,
        };
        
        // Step 3: Execute business logic
        self.process(config, details).await
    }
}
```

### 3. Validation Layering

Apply validation at multiple levels:

```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
pub struct ValidatedInput {
    /// RMCP schema validation: must be non-empty string
    #[schemars(min_length = 1)]
    /// Elicitation: prompts until valid
    #[prompt("Enter non-empty name:")]
    /// Business validation: checked in tool
    pub name: String,
}

#[tool]
async fn process(&self, Parameters(input): Parameters<ValidatedInput>) -> ... {
    // Additional business-level validation
    if self.name_exists(&input.name).await? {
        return Err("Name already taken".into());
    }
    // ...
}
```

### 4. Prompt Integration

Coordinate prompts between layers:

```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
#[prompt("Let's configure your server:")]  // Survey-level context
#[schemars(title = "Server Configuration")]  // MCP tool-level title
pub struct ServerConfig {
    /// Field-level prompts guide elicitation
    #[prompt("Enter hostname:")]
    /// Schema descriptions help LLM understand parameters
    #[schemars(description = "Server hostname or IP address")]
    pub host: String,
}
```

## Migration Path

For migrating to RMCP with elicitation integration:

### Phase 1: Add Elicit to Domain Types

```rust
// Before
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MyType { /* ... */ }

// After
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
#[prompt("Configure MyType:")]
pub struct MyType { /* ... */ }
```

### Phase 2: Convert Tools to Use Types

```rust
// Before (manual JSON)
#[tool]
async fn my_tool(&self, params: Parameters<Value>) -> ... {
    let field1 = params.get("field1")...;
    // Manual extraction
}

// After (typed parameters)
#[tool]
async fn my_tool(&self, Parameters(params): Parameters<MyType>) -> ... {
    // params.field1 already validated
}
```

### Phase 3: Add Guided Elicitation

```rust
#[tool]
async fn my_tool_guided(&self, Parameters(base): Parameters<BaseParams>) -> ... {
    // Elicit additional details conversationally
    let details = Details::elicit(&self.mcp_client).await?;
    self.process(base, details).await
}
```

## Potential Issues & Solutions

### Issue 1: Circular Dependency

**Problem:** Elicitation needs MCP client, but tools provide MCP interface

**Solution:** Separate MCP client for internal elicitation:

```rust
pub struct BotticelliServer {
    // Client for internal elicitation (in-process transport)
    internal_client: Arc<pmcp::Client<InProcTransport>>,
    
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BotticelliServer {
    #[tool]
    async fn wizard(&self, ...) -> ... {
        // Use internal client for elicitation
        let data = MyType::elicit(&self.internal_client).await?;
        // ...
    }
}
```

### Issue 2: Double Serialization

**Problem:** Elicitation uses JSON, RMCP uses JSON - redundant?

**Solution:** They serve different purposes:
- **Elicitation JSON**: MCP tool call format (primitive elicitation tools)
- **RMCP JSON**: Tool parameter format (high-level tool interface)

They compose, not duplicate.

### Issue 3: Prompt Redundancy

**Problem:** Both have prompt/description attributes

**Solution:** Use both for different audiences:
- `#[prompt("...")]`: User-facing conversational prompt
- `#[schemars(description = "...")]`: LLM-facing schema documentation

```rust
#[derive(Elicit, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    /// For LLM reasoning
    #[schemars(description = "Port number between 1-65535 for server binding")]
    /// For user interaction
    #[prompt("What port should the server listen on?")]
    pub port: u16,
}
```

## Conclusion

**Verdict: Strongly Complementary**

The elicitation and RMCP macro systems are highly complementary:

1. **Different Layers**: Elicitation = interaction, RMCP = protocol
2. **Natural Composition**: Elicitation types → RMCP parameters
3. **Shared Patterns**: Both use derives, attributes, type-driven design
4. **Mutual Benefits**: 
   - RMCP gains conversational elicitation capabilities
   - Elicitation gains protocol-level tool exposure
   - Both gain type safety across boundaries

**Key Insight:** Elicitation provides the "how to gather" and RMCP provides the "how to expose". Together they create a complete type-safe pipeline from conversational interaction to structured tool execution.

**Recommendation:** Embrace both systems and design domain types to use both derives, creating a unified architecture that leverages the strengths of each layer.

---

## Next Steps

1. **Update RMCP_MIGRATION_VISION.md** to highlight elicitation synergy
2. **Create integration examples** showing composed patterns
3. **Document dual-derive pattern** as standard practice
4. **Design internal MCP client** for tool-driven elicitation

*Document created: 2024-12-29*  
*Status: Analysis Complete*  
*Cross-ref: RMCP_MIGRATION_VISION.md, ELICITATION_MCP_INTEGRATION_PLAN.md*
