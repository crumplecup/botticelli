# **Botticelli MCP Migration Completion Plan**

## **Executive Summary**

Complete the migration from legacy `McpTool` trait pattern to native `rmcp` handlers, while addressing architectural violations and code quality issues identified in the audit.

**Estimated Time:** 14-18 hours across 7 phases

## **Order of Operations - CRITICAL**

**MIGRATE FIRST, DELETE LAST:**

1. **Phase 0**: Architectural foundation (errors, traits, ToolRegistry structure)
2. **Phase 1**: Tool Registry refactor (delegation infrastructure)  
3. **Phase 2**: Migrate ALL remaining tools to rmcp handlers
4. **Phase 3**: Verify everything works end-to-end  
5. **Phase 4**: Delete legacy code (safe - nothing uses it)
6. **Phase 5**: Code quality cleanup
7. **Phase 6**: Documentation and final verification

**Rationale**: Never delete working code until replacement is proven operational. Migration risk minimized by keeping legacy code functional during transition. Only after all tools migrated and verified working do we safely remove dead code.

---

## **Phase 0: Architectural Foundation** (3-4 hours)

**Goal:** Fix critical architectural violations that block proper error handling and abstraction.

### **Action 0.1: Move Error Types to `botticelli_error`** (1.5 hours)

1. **Add source error variants to McpErrorKind**
   - Location: `crates/botticelli_error/src/mcp.rs`
   - Add new variants with proper source tracking:
     ```rust
     // In botticelli_error/src/mcp.rs
     
     /// Serde JSON error with source tracking.
     #[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
     #[display("JSON serialization error: {:?} at {}:{}", source, file, line)]
     pub struct SerdeJsonError {
         /// The serde_json error source
         source: Box<serde_json::Error>,
         /// Line number where error was created
         line: u32,
         /// File where error was created
         file: &'static str,
     }
     
     impl SerdeJsonError {
         /// Create a new SerdeJsonError with automatic location tracking.
         #[track_caller]
         pub fn new(err: serde_json::Error) -> Self {
             let location = std::panic::Location::caller();
             Self {
                 source: Box::new(err),
                 line: location.line(),
                 file: location.file(),
             }
         }
     }
     
     impl Clone for SerdeJsonError {
         fn clone(&self) -> Self {
             // serde_json::Error doesn't implement Clone, recreate from string
             let err_str = format!("{}", self.source);
             let new_err = serde_json::Error::io(std::io::Error::new(
                 std::io::ErrorKind::Other,
                 err_str,
             ));
             Self {
                 source: Box::new(new_err),
                 line: self.line,
                 file: self.file,
             }
         }
     }
     
     /// RMCP error data with source tracking.
     #[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
     #[display("RMCP error [{}]: {} at {}:{}", code, message, file, line)]
     pub struct RmcpError {
         /// Error code from rmcp
         code: i32,
         /// Error message
         message: String,
         /// Line number where error was created
         line: u32,
         /// File where error was created
         file: &'static str,
     }
     
     impl RmcpError {
         /// Create a new RmcpError from rmcp::ErrorData.
         #[track_caller]
         pub fn new(err: rmcp::ErrorData) -> Self {
             let location = std::panic::Location::caller();
             Self {
                 code: err.code as i32,
                 message: err.message.into_owned(),
                 line: location.line(),
                 file: location.file(),
             }
         }
     }
     
     /// MCP error kinds.
     #[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
     pub enum McpErrorKind {
         // ... existing variants ...
         
         /// JSON serialization/deserialization error
         #[display("JSON error: {}", _0)]
         #[from(SerdeJsonError)]
         Json(SerdeJsonError),
         
         /// RMCP protocol error
         #[display("RMCP error: {}", _0)]
         #[from(RmcpError)]
         Rmcp(RmcpError),
         
         // Keep string-based variants for simple cases:
         /// Tool not found
         #[display("Tool not found: {}", _0)]
         ToolNotFound(String),
         
         /// Invalid input
         #[display("Invalid input: {}", _0)]
         InvalidInput(String),
         
         /// Resource not found
         #[display("Resource not found: {}", _0)]
         ResourceNotFound(String),
     }
     ```

2. **Move `SamplingError` + `SamplingErrorKind`**
   - Source: `crates/botticelli_mcp/src/tools/sampling.rs:225-288`
   - Destination: `crates/botticelli_error/src/chat.rs`
   - Implementation:
     ```rust
     // In botticelli_error/src/chat.rs
     
     /// Sampling error kinds.
     #[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
     pub enum SamplingErrorKind {
         #[display("Max turns exceeded: {}", max)]
         MaxTurnsExceeded { max: usize },
         
         #[display("Tool execution failed: {} - {}", tool_name, reason)]
         ToolExecutionFailed { tool_name: String, reason: String },
         
         #[display("Unknown tool: {}", name)]
         UnknownTool { name: String },
         
         #[display("Provider error: {}", _0)]
         ProviderError(String),
         
         #[display("No tool registry configured")]
         NoToolRegistry,
         
         #[display("Request building failed: {}", _0)]
         RequestBuildingFailed(String),
     }
     
     /// Sampling error with location tracking.
     #[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
     #[display("Sampling: {} at {}:{}", kind, file, line)]
     pub struct SamplingError {
         kind: SamplingErrorKind,
         line: u32,
         file: &'static str,
     }
     
     impl SamplingError {
         #[track_caller]
         pub fn new(kind: SamplingErrorKind) -> Self {
             let loc = std::panic::Location::caller();
             Self {
                 kind,
                 line: loc.line(),
                 file: loc.file(),
             }
         }
     }
     
     // Update ChatErrorKind
     pub enum ChatErrorKind {
         // ... existing variants
         
         // Replace:
         // SamplingError(String),
         
         // With:
         #[display("Sampling error: {}", _0)]
         #[from(SamplingError)]
         Sampling(SamplingError),
     }
     ```

2. **Update all imports**
   - `crates/botticelli_mcp/src/tools/sampling.rs`: Change to `use botticelli_error::{SamplingError, SamplingErrorKind}`
   - `crates/botticelli_chat/src/sampling.rs`: Add import
   - `crates/botticelli_mcp/src/lib.rs`: Update re-export

3. **Verify:** `just check botticelli_error && just check botticelli_mcp && just check botticelli_chat`

### **Action 0.2: Move Traits to `botticelli_interface`** (1.5-2 hours)

1. **Move `LlmSampler` trait**
   - Source: `crates/botticelli_mcp/src/tools/sampling.rs:98-213`
   - Destination: `crates/botticelli_interface/src/llm_sampler.rs`
   - Refactor with associated types:
     ```rust
     // In botticelli_interface/src/llm_sampler.rs
     
     use async_trait::async_trait;
     
     /// Trait for LLM sampling with tool support.
     #[async_trait]
     pub trait LlmSamplerOperations: Send + Sync {
         /// Session type for conversation state
         type Session;
         /// Tool definition type
         type ToolDefinition;
         /// Response type from generation
         type Response;
         /// Result type for completed sessions
         type Result;
         /// Error type
         type Error: std::error::Error + Send + Sync + 'static;
         /// Tool call type
         type ToolCall;
         /// Tool result type
         type ToolResult;
         
         /// Generate a single response with optional tools.
         async fn generate(
             &self,
             session: &Self::Session,
             available_tools: &[Self::ToolDefinition],
         ) -> Result<Self::Response, Self::Error>;
         
         /// Run a complete sampling session.
         async fn sample(
             &self,
             session: &mut Self::Session,
             available_tools: &[Self::ToolDefinition],
         ) -> Result<Self::Result, Self::Error>;
         
         /// Execute tool calls and return results.
         async fn execute_tools(
             &self,
             calls: &[Self::ToolCall],
         ) -> Result<Vec<Self::ToolResult>, Self::Error>;
     }
     ```

2. **Create type alias in `botticelli_mcp`**
   - In `crates/botticelli_mcp/src/tools/sampling.rs`:
     ```rust
     use botticelli_interface::LlmSamplerOperations;
     
     /// Concrete LlmSampler type for MCP usage.
     pub type LlmSampler = dyn LlmSamplerOperations<
         Session = ConversationSession,
         ToolDefinition = botticelli_core::ToolDefinition,
         Response = botticelli_core::GenerateResponse,
         Result = SamplingResult,
         Error = SamplingError,
         ToolCall = botticelli_core::ToolCall,
         ToolResult = botticelli_core::ToolResult,
     >;
     ```

3. **Keep helper implementations** in `botticelli_mcp`:
   - `SamplingCoordinator`
   - `SamplingHelper`
   - `SamplingResult` enum

4. **Update `botticelli_interface/src/lib.rs`**:
   ```rust
   mod llm_sampler;
   pub use llm_sampler::LlmSamplerOperations;
   ```

5. **Verify:** `just check botticelli_interface && just check botticelli_mcp && just check botticelli_chat`

### **Action 0.3: Remove `#[allow]` Directive** (30 minutes)

1. **Fix `CreateNarrativeResult::new`** in `src/create_narrative.rs:49`
   - Extract parameter struct:
     ```rust
     /// Parameters for CreateNarrativeResult constructor.
     #[derive(Debug, Clone, derive_new::new, derive_getters::Getters)]
     pub struct CreateNarrativeResultParams {
         toml: String,
         toml_with_comments: String,
         validation: Value,
         summary: String,
         auto_fixes_applied: Vec<String>,
         suggestions: Vec<String>,
     }
     
     impl CreateNarrativeResult {
         pub fn new(params: CreateNarrativeResultParams) -> Self {
             Self {
                 toml: params.toml().clone(),
                 toml_with_comments: params.toml_with_comments().clone(),
                 validation: params.validation().clone(),
                 summary: params.summary().clone(),
                 auto_fixes_applied: params.auto_fixes_applied().clone(),
                 suggestions: params.suggestions().clone(),
             }
         }
     }
     ```

2. **Update call sites** in `rmcp_server/tools/narrative.rs`

3. **Verify:** `cargo clippy --package botticelli_mcp -- -D warnings`

---

## **Phase 1: ToolRegistry Refactor** (2-3 hours)

**Goal:** Make `ToolRegistry` delegate to rmcp handlers instead of wrapping `McpTool` trait objects.

### **Action 1.1: Refactor ToolRegistry** (1.5 hours)

1. **Update struct definition** in `src/tools/mod.rs:98-166`:
   ```rust
   use crate::BotticelliServer;
   use std::sync::Arc;
   
   /// Registry for tools - delegates to rmcp ToolRouter.
   #[derive(Clone)]
   pub struct ToolRegistry {
       server: Arc<BotticelliServer>,
   }
   
   impl ToolRegistry {
       /// Create from BotticelliServer.
       pub fn new(server: Arc<BotticelliServer>) -> Self {
           Self { server }
       }
       
       /// Get metrics collector if configured.
       pub fn metrics(&self) -> Option<&Arc<PrometheusMetrics>> {
           self.server.metrics.as_ref()
       }
   }
   ```

2. **Delete methods:**
   - `register()`
   - `get()`
   - `list()`
   - `Default` impl with manual tool registration

### **Action 1.2: Implement execute() method** (1 hour)

1. **Add execute delegation** in `src/tools/mod.rs`:
   ```rust
   impl ToolRegistry {
       /// Execute a tool by name.
       #[instrument(skip(self, input), fields(tool_name = name, input_size = input.to_string().len()))]
       pub async fn execute(&self, name: &str, input: Value) -> McpResult<Value> {
           use rmcp::handler::server::wrapper::Parameters;
           
           debug!(tool_name = name, "Executing tool");
           
           match name {
               "echo" => {
                   debug!("Deserializing EchoParams");
                   let params: crate::EchoParams = serde_json::from_value(input)
                       .map_err(|e| {
                           error!(error = %e, "Failed to deserialize EchoParams");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })?;
                   
                   debug!("Calling rmcp handler");
                   let result = self.server.echo(Parameters(params))
                       .await
                       .map_err(|e| {
                           error!(error = ?e, "RMCP handler failed");
                           McpError::new(McpErrorKind::Rmcp(RmcpError::new(e)))
                       })?;
                   
                   debug!("Serializing EchoResult");
                   serde_json::to_value(result.0)
                       .map_err(|e| {
                           error!(error = %e, "Failed to serialize EchoResult");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })
               }
               
               "server_info" => {
                   debug!("Calling rmcp handler");
                   let result = self.server.server_info()
                       .await
                       .map_err(|e| {
                           error!(error = ?e, "RMCP handler failed");
                           McpError::new(McpErrorKind::Rmcp(RmcpError::new(e)))
                       })?;
                   
                   debug!("Serializing ServerInfoResult");
                   serde_json::to_value(result.0)
                       .map_err(|e| {
                           error!(error = %e, "Failed to serialize ServerInfoResult");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })
               }
               
               "execute_act" => {
                   debug!("Deserializing ExecuteActParams");
                   let params: crate::ExecuteActParams = serde_json::from_value(input)
                       .map_err(|e| {
                           error!(error = %e, "Failed to deserialize ExecuteActParams");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })?;
                   
                   debug!("Calling rmcp handler");
                   let result = self.server.execute_act(Parameters(params))
                       .await
                       .map_err(|e| {
                           error!(error = ?e, "RMCP handler failed");
                           McpError::new(McpErrorKind::Rmcp(RmcpError::new(e)))
                       })?;
                   
                   debug!("Serializing ExecuteActResult");
                   serde_json::to_value(result.0)
                       .map_err(|e| {
                           error!(error = %e, "Failed to serialize ExecuteActResult");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })
               }
               
               "execute_narrative" => {
                   debug!("Deserializing ExecuteNarrativeParams");
                   let params: crate::ExecuteNarrativeParams = serde_json::from_value(input)
                       .map_err(|e| {
                           error!(error = %e, "Failed to deserialize ExecuteNarrativeParams");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })?;
                   
                   debug!("Calling rmcp handler");
                   let result = self.server.execute_narrative(Parameters(params))
                       .await
                       .map_err(|e| {
                           error!(error = ?e, "RMCP handler failed");
                           McpError::new(McpErrorKind::Rmcp(RmcpError::new(e)))
                       })?;
                   
                   debug!("Serializing ExecuteNarrativeResult");
                   serde_json::to_value(result.0)
                       .map_err(|e| {
                           error!(error = %e, "Failed to serialize ExecuteNarrativeResult");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })
               }
               
               "generate" => {
                   debug!("Deserializing GenerateParams");
                   let params: crate::GenerateParams = serde_json::from_value(input)
                       .map_err(|e| {
                           error!(error = %e, "Failed to deserialize GenerateParams");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })?;
                   
                   debug!("Calling rmcp handler");
                   let result = self.server.generate(Parameters(params))
                       .await
                       .map_err(|e| {
                           error!(error = ?e, "RMCP handler failed");
                           McpError::new(McpErrorKind::Rmcp(RmcpError::new(e)))
                       })?;
                   
                   debug!("Serializing GenerateResult");
                   serde_json::to_value(result.0)
                       .map_err(|e| {
                           error!(error = %e, "Failed to serialize GenerateResult");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })
               }
               
               #[cfg(feature = "discord")]
               "discord_post_message" => {
                   debug!("Deserializing DiscordPostMessageParams");
                   let params: crate::DiscordPostMessageParams = serde_json::from_value(input)
                       .map_err(|e| {
                           error!(error = %e, "Failed to deserialize DiscordPostMessageParams");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })?;
                   
                   debug!("Calling rmcp handler");
                   let result = self.server.discord_post_message(Parameters(params))
                       .await
                       .map_err(|e| {
                           error!(error = ?e, "RMCP handler failed");
                           McpError::new(McpErrorKind::Rmcp(RmcpError::new(e)))
                       })?;
                   
                   debug!("Serializing DiscordPostMessageResult");
                   serde_json::to_value(result.0)
                       .map_err(|e| {
                           error!(error = %e, "Failed to serialize DiscordPostMessageResult");
                           McpError::new(McpErrorKind::Json(SerdeJsonError::new(e)))
                       })
               }
               
               // Add all other tools...
               
               _ => {
                   warn!(tool_name = name, "Tool not found");
                   Err(McpError::tool_not_found(name.to_string()))
               }
           }
       }
   }
   ```

### **Action 1.3: Implement tool_definitions()** (30 minutes)

1. **Add tool_definitions method**:
   ```rust
   impl ToolRegistry {
       /// Get tool definitions for LLM function calling.
       #[instrument(skip(self))]
       pub fn tool_definitions(&self) -> Vec<botticelli_core::ToolDefinition> {
           use rmcp::handler::server::tool::ToolInfo;
           
           debug!("Retrieving tool definitions from rmcp ToolRouter");
           
           let tools = self.server.tool_router.list_all()
               .into_iter()
               .map(|tool: ToolInfo| {
                   trace!(
                       tool_name = tool.name,
                       has_description = tool.description.is_some(),
                       "Converting tool info to ToolDefinition"
                   );
                   
                   botticelli_core::ToolDefinition::new(
                       tool.name,
                       tool.description.unwrap_or_default(),
                       tool.input_schema,
                   )
               })
               .collect::<Vec<_>>();
           
           debug!(tool_count = tools.len(), "Retrieved tool definitions");
           tools
       }
   }
   ```

2. **Update BotticelliServerBuilder**:
   - Add method to build ToolRegistry after server construction
   - Or pass server Arc when creating ToolRegistry

3. **Verify:** `just check botticelli_mcp`

---

## **Phase 2: Migrate Remaining Tools** (3-4 hours)

**Goal:** Complete migration of elicitation session tools still using legacy pattern. Keep legacy infrastructure in place until migration verified.

### **Action 2.1: Migrate Elicitation Session Tools** (2 hours)

Tools in `src/tools/elicitation/session_tools.rs`:
- `CreateNarrativeSessionTool`
- `ElicitMetadataTool`
- `ElicitActTool`
- `ElicitCarouselTool`
- `FinalizeNarrativeTool`
- `GetNarrativeStateTool`
- `ValidateNarrativeSessionTool`
- `ApplyValidationFixesTool`

For each tool:

1. **Verify param/result types exist** in top-level modules (e.g., `src/session_tools.rs`)
2. **Create rmcp handler** in `src/rmcp_server/tools/elicitation.rs`:
   ```rust
   impl BotticelliServer {
       pub async fn create_narrative_session(
           &self,
           Parameters(params): Parameters<CreateNarrativeSessionParams>,
       ) -> Result<Json<CreateNarrativeSessionResult>, rmcp::ErrorData> {
           // Implementation
       }
   }
   ```
3. **Add to `execute()` match** in ToolRegistry
4. **Delete legacy impl**

### **Action 2.2: Migrate Narrative Creation Tools** (1 hour)

Tools in `src/tools/narrative_creation.rs`:
- Check if `StartNarrativeTool` and related are still needed
- Migrate any remaining narrative creation tools

### **Action 2.3: Update ToolRegistry execute()** (30 minutes)

Add all newly migrated tools to the match statement.

### **Action 2.4: Verify Complete Migration** (30 minutes)

1. **Search for stragglers**:
   ```bash
   grep -r "impl McpTool" src/
   # Should return: no results
   
   grep -r "Arc<dyn McpTool>" src/
   # Should return: no results
   ```

2. **Run full checks**: `just check-all botticelli_mcp`

---

## **Phase 3: Verify Migration Complete** (1 hour)

**Goal:** Ensure all tools work with new architecture before deleting legacy code. This is the safety gate - only proceed to Phase 4 after all verification passes.

### **Action 3.1: Integration Testing** (30 min)

1. **Test ToolRegistry delegation**:
   ```rust
   // In tests/tool_registry_integration_test.rs
   #[tokio::test]
   async fn test_all_tools_execute() {
       let server = create_test_server();
       let registry = ToolRegistry::new(Arc::clone(&server));
       
       // Test each tool category
       let test_cases = vec![
           ("echo", json!({"message": "test"})),
           ("server_info", json!({})),
           ("create_narrative", json!({"name": "test", "description": "test narrative"})),
           ("modify_narrative", json!({"name": "test", "changes": "add act"})),
           ("save_narrative", json!({"name": "test", "toml": "[acts]"})),
           ("execute_act", json!({"narrative": "test", "act": 0})),
           ("execute_narrative", json!({"narrative": "test"})),
           ("generate", json!({"prompt": "test"})),
           ("session_get_context", json!({"session_id": "test"})),
           ("session_update_context", json!({"session_id": "test", "context": {}})),
           ("session_create", json!({"name": "test"})),
           ("session_destroy", json!({"session_id": "test"})),
           ("session_next_turn", json!({"session_id": "test"})),
           ("session_undo_turn", json!({"session_id": "test"})),
           ("session_save", json!({"session_id": "test", "path": "/tmp/test"})),
           ("session_load", json!({"path": "/tmp/test"})),
       ];
       
       for (tool_name, input) in test_cases {
           debug!(tool_name, "Testing tool execution");
           let result = registry.execute(tool_name, input).await;
           assert!(
               result.is_ok() || is_expected_error(&result),
               "Tool {} failed unexpectedly: {:?}",
               tool_name,
               result
           );
       }
   }
   
   fn is_expected_error(result: &Result<Value, McpError>) -> bool {
       match result {
           Err(e) => {
               // Expected errors: missing params, invalid input, not found
               matches!(
                   e.kind,
                   McpErrorKind::InvalidInput(_) | McpErrorKind::ResourceNotFound(_)
               )
           }
           Ok(_) => true,
       }
   }
   ```

2. **Test LlmSampler integration**:
   ```rust
   #[tokio::test]
   async fn test_llm_sampler_with_tools() {
       let server = create_test_server();
       let sampler = create_test_sampler(Arc::clone(&server));
       
       // Verify tool definitions available
       let tools = sampler.tool_definitions();
       assert!(tools.len() >= 16, "All tools should be available, got {}", tools.len());
       
       // Verify tool names include migrated tools
       let tool_names: Vec<_> = tools.iter().map(|t| t.name()).collect();
       assert!(tool_names.contains(&"echo"));
       assert!(tool_names.contains(&"create_narrative"));
       assert!(tool_names.contains(&"session_create"));
       
       // Test tool execution through sampler
       let test_calls = vec![
           ToolCall::new("echo", json!({"message": "test"})),
       ];
       let result = sampler.execute_tools(&test_calls).await;
       assert!(result.is_ok(), "Tool execution failed: {:?}", result);
   }
   ```

3. **Run tests**: `just test-package botticelli_mcp`

4. **Verify**: All integration tests pass with new delegation

### **Action 3.2: Manual End-to-End Verification** (30 min)

1. **Start MCP server**: 
   ```bash
   just run-mcp-server
   ```

2. **Test with MCP client**: 
   - Connect Claude Desktop or other MCP client
   - Exercise each tool category:
     - Core tools (echo, server_info)
     - Narrative tools (create, modify, save)
     - Execution tools (execute_act, execute_narrative, generate)
     - Session tools (create, update, next_turn, etc.)
   - Verify responses correct

3. **Test LLM sampling with narrative generation**:
   ```bash
   # Run a narrative that uses tool calls
   just test-narrative-generation
   ```

4. **Check instrumentation logs**:
   ```bash
   # Verify spans and events appear
   RUST_LOG=debug just run-mcp-server 2>&1 | grep "botticelli_mcp::tools"
   ```
   - Should see: `execute` spans with tool names
   - Should see: `debug!` events for delegation
   - Should see: complete request flow traces

**Success criteria - ALL must pass**:
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ MCP client can execute all tools
- ✅ LlmSampler executes tools successfully
- ✅ No legacy code paths executed (check logs for old tool structs)
- ✅ Instrumentation traces complete request flow
- ✅ Zero compilation warnings
- ✅ Zero clippy warnings

**If any verification fails**: Do NOT proceed to Phase 4. Fix issues in Phase 2, re-verify Phase 3.

---

## **Phase 4: Delete Legacy Infrastructure** (1 hour)

**⚠️ ONLY PROCEED AFTER Phase 3 verification passes completely.**

**Goal:** Remove old trait-based system now that replacement is proven operational.

### **Action 4.1: Delete McpTool Trait** (15 minutes)

1. **Remove from `src/tools/mod.rs:82-96`**:
   - Delete entire `McpTool` trait definition
   - Remove from any doc comments

2. **Remove from `src/lib.rs`**:
   - Delete `pub use tools::McpTool;` export

3. **Verify**: `just check botticelli_mcp` (should compile without McpTool)

### **Action 4.2: Delete Legacy Tool Implementations** (30 minutes)

Delete these files entirely:

```bash
# Duplicates of rmcp handlers
rm crates/botticelli_mcp/src/tools/echo.rs
rm crates/botticelli_mcp/src/tools/server_info.rs
rm crates/botticelli_mcp/src/tools/execution.rs      # execute_act, execute_narrative
rm crates/botticelli_mcp/src/tools/narrative.rs      # create/modify/save/validate
rm crates/botticelli_mcp/src/tools/generate.rs
rm crates/botticelli_mcp/src/tools/discord.rs

# Empty tombstones
rm crates/botticelli_mcp/src/tools/elicitation_primitives.rs
rm crates/botticelli_mcp/src/tools/scene.rs

# Session tools (now migrated to rmcp)
rm crates/botticelli_mcp/src/tools/session/context.rs
rm crates/botticelli_mcp/src/tools/session/flow.rs
rm crates/botticelli_mcp/src/tools/session/management.rs
rm crates/botticelli_mcp/src/tools/session/state.rs
rm -r crates/botticelli_mcp/src/tools/session/  # Remove directory if empty
```

**Keep** param/result type modules:
- `src/echo.rs` (contains `EchoParams`, `EchoResult`)
- `src/execution.rs` (contains execution types)
- `src/discord_tools.rs` (contains Discord types)
- `src/server_info.rs` (contains `ServerInfoResult`)

### **Action 4.3: Clean Up Module Declarations** (10 minutes)

1. **Update `src/tools/mod.rs`**:
   ```rust
   // Remove mod declarations:
   // mod echo;
   // mod server_info;
   // mod execution;
   // mod narrative;
   // mod generate;
   // mod discord;
   // mod elicitation_primitives;
   // mod scene;
   // mod session;
   
   // Remove Default impl for ToolRegistry (lines 168-230)
   // that registered legacy tools
   
   // Keep only:
   mod sampling;  // LlmSampler trait and helpers
   pub use sampling::*;
   
   // ToolRegistry struct definition stays
   ```

2. **Update `src/lib.rs`**:
   ```rust
   // Remove tool struct exports
   // pub use tools::{EchoTool, ServerInfoTool, ...};
   
   // Keep:
   pub use tools::{ToolRegistry, LlmSampler, SamplingCoordinator};
   
   // Keep param/result exports
   pub use echo::{EchoParams, EchoResult};
   pub use server_info::ServerInfoResult;
   // etc.
   ```

3. **Verify**: `just check botticelli_mcp`

### **Action 4.4: Remove Unused Imports** (5 minutes)

1. **Run clippy**: `cargo clippy --package botticelli_mcp`
2. **Remove unused imports** flagged by clippy
3. **Verify**: Zero unused import warnings

---

## **Phase 5: Code Quality Fixes** (3-4 hours)

**Goal:** Address audit findings for code hygiene after migration complete and legacy code removed.

### **Action 5.1: Move Inline Tests to tests/ Directory** (1.5 hours)

Files with inline tests:
- `src/tools/narrative_validation_helpers.rs`
- `src/resources/narrative.rs`
- `src/resources/content.rs`
- `src/tools/bot_commands.rs`

For each file:

1. **Create test file** in `tests/`:
   ```bash
   # Example:
   mv src/resources/narrative.rs narrative_resource_test.rs
   ```

2. **Update imports** to use crate-level exports:
   ```rust
   // Before:
   use super::*;
   
   // After:
   use botticelli_mcp::{NarrativeResource, McpResource};
   ```

3. **Remove `#[cfg(test)] mod tests`** from source file

4. **Verify:** `just test-package botticelli_mcp`

### **Action 5.2: Fix Import Patterns** (1 hour)

Replace all `super::` imports with `use crate::{...}`:

1. **Automated find**:
   ```bash
   grep -r "use super::" src/ | wc -l
   # Target: 20+ occurrences
   ```

2. **High concentration areas**:
   - `src/rmcp_server/` modules
   - `src/resources/` modules

3. **Pattern replacement**:
   ```rust
   // Before:
   use super::{McpResource, ResourceInfo};
   use super::super::helpers::to_mcp_error;
   
   // After:
   use crate::{McpResource, ResourceInfo};
   use crate::rmcp_server::helpers::to_mcp_error;
   ```

4. **Verify:** `just check botticelli_mcp`

### **Action 5.3: Clean Unused Imports** (30 minutes)

Address 15+ warnings from cargo check:

1. **Run check**: `cargo check --package botticelli_mcp 2>&1 | grep "unused import"`

2. **Remove unused imports**:
   - `rmcp::tool` (6 files in `rmcp_server/tools/`)
   - `instrument` (6 files)
   - `to_mcp_error`, `NarrativeAnalysis`, etc.

3. **Verify:** `cargo check --package botticelli_mcp 2>&1 | grep warning` returns 0 results

### **Action 5.4: Complete Instrumentation** (1 hour)

Add `#[instrument]` to all public and internal functions:

1. **Target areas** (missing ~60% of functions):
   - `src/rmcp_server/tools/*.rs` handlers
   - Helper functions in `src/tools/narrative_utils.rs`
   - All public APIs without instrumentation

2. **Pattern with proper span fields**:
   ```rust
   use tracing::{debug, error, info, instrument, trace, warn};
   
   // Simple function
   #[instrument(skip(self), fields(param1, param2))]
   pub async fn method(&self, param1: String, param2: i32) -> Result<T, E> {
       debug!(param1, param2, "Starting operation");
       
       // Operation
       let result = do_work().await?;
       
       debug!(result_size = result.len(), "Operation completed");
       Ok(result)
   }
   
   // With error logging
   #[instrument(skip(self, conn), fields(table_name, limit))]
   pub async fn query(
       &self,
       conn: &mut Connection,
       table_name: &str,
       limit: i64,
   ) -> Result<Vec<Row>, Error> {
       debug!(table_name, limit, "Executing query");
       
       match execute_query(conn, table_name, limit).await {
           Ok(rows) => {
               debug!(count = rows.len(), "Query successful");
               Ok(rows)
           }
           Err(e) => {
               error!(error = %e, table_name, limit, "Query failed");
               Err(e)
           }
       }
   }
   
   // Skip large structures
   #[instrument(skip(self, large_data, connection))]
   pub fn process(&self, id: String, large_data: &[u8]) -> Result<(), Error> {
       debug!(id, data_size = large_data.len(), "Processing");
       // ...
   }
   ```

3. **Specific examples for rmcp handlers**:
   ```rust
   // In src/rmcp_server/tools/narrative.rs
   impl BotticelliServer {
       #[instrument(skip(self, params), fields(name = params.name))]
       pub async fn create_narrative(
           &self,
           Parameters(params): Parameters<CreateNarrativeParams>,
       ) -> Result<Json<CreateNarrativeResult>, rmcp::ErrorData> {
           debug!(
               name = params.name,
               has_model = params.default_model.is_some(),
               "Creating narrative from description"
           );
           
           // Validate name
           if !NarrativeHelper::is_valid_name(&params.name) {
               warn!(name = params.name, "Invalid narrative name");
               return Err(rmcp::ErrorData::new(
                   ErrorCode::INVALID_PARAMS,
                   Cow::Owned(format!("Invalid narrative name: {}", params.name)),
                   None,
               ));
           }
           
           debug!("Generating narrative TOML");
           let mut toml = generate_narrative_toml(
               &params.description,
               &params.name,
               params.default_model.as_deref(),
               params.default_temperature,
           )?;
           
           debug!("Applying auto-fixes");
           let (fixed_toml, fixes_applied) = auto_fix_common_issues(&toml);
           toml = fixed_toml;
           
           debug!(fixes_count = fixes_applied.len(), "Validating narrative");
           let validation = Validator::validate_toml(&toml);
           
           info!(
               name = params.name,
               valid = validation.is_valid(),
               errors = validation.errors().len(),
               warnings = validation.warnings().len(),
               fixes = fixes_applied.len(),
               "Narrative generation complete"
           );
           
           // Build result...
       }
   }
   ```

4. **Helper function examples**:
   ```rust
   // In src/tools/narrative_utils.rs
   impl NarrativeHelper {
       #[instrument(skip(description), fields(desc_len = description.len()))]
       pub fn generate_toml(description: &str) -> Result<String, Error> {
           debug!("Parsing description");
           let parsed = parse_description(description)?;
           
           debug!(acts_count = parsed.acts.len(), "Building TOML");
           let toml = build_toml(&parsed);
           
           debug!(toml_len = toml.len(), "TOML generation complete");
           Ok(toml)
       }
       
       #[instrument]
       pub fn is_valid_name(name: &str) -> bool {
           let valid = name.chars().all(|c| c.is_alphanumeric() || c == '_')
               && name.chars().next().map_or(false, |c| c.is_alphabetic());
           
           trace!(name, valid, "Name validation result");
           valid
       }
       
       #[instrument(skip(toml))]
       pub fn count_acts(toml: &str) -> usize {
           let count = toml.lines()
               .filter(|line| line.starts_with("[[acts]]"))
               .count();
           
           trace!(count, "Counted acts in TOML");
           count
       }
   }
   ```

5. **Verification checklist**:
   - [ ] All `pub fn` have `#[instrument]`
   - [ ] All `pub async fn` have `#[instrument]`
   - [ ] All `pub(crate) fn` have `#[instrument]`
   - [ ] All `pub(super) fn` have `#[instrument]`
   - [ ] Large structures use `skip(...)`
   - [ ] Important values in `fields(...)`
   - [ ] Key operations emit debug events
   - [ ] Errors logged before return
   - [ ] Success outcomes logged with metrics

---

## **Phase 6: Final Cleanup & Documentation** (1-2 hours)

**Goal:** Polish, document, and validate the migration.

### **Action 6.1: Make tools Module Private** (15 minutes)

1. **Update `src/lib.rs`**:
   ```rust
   // Before:
   pub mod tools;
   
   // After:
   mod tools;
   
   // Keep selective exports:
   pub use tools::{
       SamplingCoordinator,
       ToolRegistry,
       // ... other needed types
   };
   ```

2. **Verify:** External crates can't access internal tool implementations

### **Action 6.2: Update Documentation** (30 minutes)

1. **Update module docs** in `src/lib.rs`:
   ```rust
   //! Model Context Protocol (MCP) server for Botticelli.
   //!
   //! This crate provides an MCP server using the `rmcp` library.
   //! All tools are implemented as native rmcp handlers.
   //!
   //! # Architecture
   //!
   //! - **rmcp handlers**: Tool implementations in `rmcp_server/tools/`
   //! - **ToolRegistry**: Thin wrapper for LLM function calling compatibility
   //! - **Param/Result types**: Shared API contracts
   ```

2. **Add MIGRATION.md**:
   ```markdown
   # MCP Migration Complete
   
   ## Summary
   
   All tools migrated from legacy `McpTool` trait to native `rmcp` handlers.
   
   ## Architecture
   
   - Tools: `src/rmcp_server/tools/*.rs`
   - API types: Top-level modules (e.g., `src/echo.rs`, `src/execution.rs`)
   - LLM integration: `ToolRegistry` delegates to rmcp handlers
   
   ## Benefits
   
   - Single source of truth (rmcp handlers)
   - No duplication
   - Type-safe, no trait objects
   - Cleaner codebase (~3000 lines removed)
   ```

3. **Update CLAUDE.md** if needed with new patterns

### **Action 6.3: Comprehensive Verification** (45 minutes)

1. **Run all checks**:
   ```bash
   just check-all botticelli_mcp
   just test-package botticelli_mcp
   just check botticelli_chat  # Dependent crate
   just clippy-all
   ```

2. **Verify no regressions**:
   - All tests pass
   - Zero clippy warnings
   - Zero unused imports
   - All instrumentation present

3. **Manual smoke test**:
   - Build MCP server binary
   - Test echo tool via MCP client
   - Test LLM sampling with tools

### **Action 6.4: Commit & Push** (15 minutes)

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(mcp): Complete migration to rmcp handlers

Complete migration from legacy McpTool trait to native rmcp handlers.

Phase 0 - Architectural fixes:
- Move SamplingError to botticelli_error
- Move LlmSampler trait to botticelli_interface
- Remove #[allow] directive

Phase 1 - ToolRegistry refactor:
- Delegate to rmcp handlers
- Remove trait object infrastructure
- Implement tool_definitions() from rmcp metadata

Phase 2 - Migrate remaining tools:
- Migrate elicitation session tools (context, flow, management, state)
- Migrate narrative creation tools
- Update ToolRegistry match arms
- All tools now use rmcp

Phase 3 - Verify migration complete:
- Integration tests for all tools
- Manual end-to-end verification
- LlmSampler integration tests
- Check instrumentation working
- **Safety gate - must pass before Phase 4**

Phase 4 - Delete legacy code:
- Remove McpTool trait
- Delete duplicate tool implementations  
- Remove empty modules
- Clean up module declarations

Phase 5 - Code quality:
- Move inline tests to tests/ directory
- Fix all super:: imports to use crate::
- Clean unused imports
- Complete instrumentation coverage

Phase 6 - Documentation:
- Make tools module private
- Update documentation
- Add MIGRATION.md
- Final comprehensive verification

Results:
- ~3000 lines removed
- Zero duplication
- Single source of truth
- All checks passing

Testing:
- All unit tests passing
- Integration tests verified
- MCP client connectivity confirmed

🤖 Generated with Claude Code

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"

git push origin HEAD
```

---

## **Verification Checklist**

After each phase, verify:

- [ ] `cargo check --package botticelli_mcp` - no errors
- [ ] `cargo clippy --package botticelli_mcp -- -D warnings` - zero warnings
- [ ] `cargo test --package botticelli_mcp` - all tests pass
- [ ] No `#[allow]` directives in codebase
- [ ] No `#[cfg(test)] mod tests` in source files
- [ ] No `use super::` imports (except test modules)
- [ ] All public/internal functions have `#[instrument]`
- [ ] `grep -r "impl McpTool" src/` returns nothing
- [ ] `just check-all botticelli_mcp` passes

---

## **Rollback Plan**

If issues arise:

1. **Phase 0-1**: Git revert, architectural changes isolated
2. **Phase 2**: Keep ToolRegistry changes, roll back individual tool migrations
3. **Phase 3**: Re-run verification, fix issues in Phase 2
4. **Phase 4**: Safe to revert - legacy code can be restored
5. **Phase 5-6**: Low risk, can defer to future cleanup

**Critical point:** Do NOT proceed past Phase 3 until all verification passes.

---

## **Success Criteria**

✅ Zero `McpTool` trait implementations remain  
✅ All tools accessible via rmcp handlers  
✅ `LlmSampler` works with refactored `ToolRegistry`  
✅ Zero architectural violations (errors/traits in correct crates)  
✅ Zero clippy warnings  
✅ All tests passing  
✅ Documentation updated  
✅ Phase 3 verification gate passed before deletion

**Estimated Total Time:** 14-18 hours

---

## Phase 6 Update: Testing Pattern Migration (In Progress)

### Objective
Apply consistent testing patterns across all test files:
- Use `anyhow::Result<()>` with `?` operator for seamless error handling
- Initialize tracing with `helpers::init_test_tracing("info")`  
- Add comprehensive tracing for full observability
- Migrate from old McpTool trait to ToolRegistry.execute() or direct server APIs

### Progress: 16/28 files completed (93 tests, 57%)

**Completed Files:**
1. echo_tool_test.rs (3 tests)
2. elicit_text_test.rs (3 tests)
3. elicit_bool_test.rs (4 tests)
4. elicit_number_test.rs (6 tests)
5. elicit_select_test.rs (6 tests)
6. export_metrics_test.rs (5 tests)
7. conversation_test.rs (7 tests)
8. execution_test.rs (1 test)
9. server_info_tool_test.rs (3 tests)
10. query_content_test.rs (4 tests)
11. validation_test.rs (4 tests)
12. scene_tools_test.rs (10 tests)
13. validate_narrative_test.rs (7 tests)
14. execution_tools_test.rs (10 tests)
15. execution_tools_rmcp_test.rs (13 tests)
16. validate_narrative_rmcp_test.rs (9 tests)

**Remaining Work:**

*Obsolete Files (5)* - Use old pmcp protocol:
- in_proc_transport_test.rs
- elicitation_integration_test.rs
- elicitation_derive_test.rs
- pmcp_http_test.rs
- pmcp_server_test.rs

Action: Mark with #[ignore] and document as obsolete

*Need Migration (4)* - Use old McpTool trait:
- narrative_generation_test.rs
- narrative_validation_test.rs
- narrative_tools_test.rs
- discord_tools_test.rs

Action: Migrate to ToolRegistry.execute()

*Complex Integration (2)*:
- integration_workflow_test.rs
- server_lifecycle_test.rs

Action: Investigate and update or mark feature-gated

### Pattern Success
- Clean, precise edits with proper observability
- Consistent structure across all updated tests
- Zero test failures after migration
- Full tracing support for debugging

