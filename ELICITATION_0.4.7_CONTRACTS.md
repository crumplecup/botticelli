# Elicitation 0.4.7: Contract-Based Tool Architecture

**Status**: Analysis complete. Ready for implementation.

**Vision**: Replace hand-rolled elicitation with formally verified Tool contracts that work for both human (TUI) and agent (LLM sampling) protocols.

---

## What's New in 0.4.7

Elicitation 0.4.7 adds a **formally verified contract system** for building type-safe, composable MCP tools:

### Core Contracts API

```rust
use elicitation::{
    contracts::{Prop, Established, And, Is, both},
    tool::{Tool, True, then},
};

// Define domain propositions
struct EmailValidated;
impl Prop for EmailValidated {}

// Tools with contracts
impl Tool for ValidateEmailTool {
    type Input = String;
    type Output = String;
    type Pre = True;           // No precondition
    type Post = EmailValidated; // Establishes validation
    
    async fn execute(&self, email: String, _pre: Established<True>)
        -> Result<(String, Established<EmailValidated>), ToolError>
    {
        if email.contains('@') {
            Ok((email, Established::assert()))
        } else {
            Err(ToolError::validation("Invalid email"))
        }
    }
}

// Compose tools - type-checked!
let pipeline = then(validate_tool, send_tool);
```

### Key Features

1. **Type-Level Contracts**: Preconditions and postconditions in types
   - `type Pre: Prop` - What must be true before calling
   - `type Post: Prop` - What's true after success
   - `Established<P>` - Zero-cost proof that P holds

2. **Composition Operators**:
   - `then(t1, t2)` - Sequential composition (t1's output → t2's input)
   - `both_tools(t1, t2)` - Parallel composition (combine proofs)
   - `both(p, q)` - Combine two proofs into conjunction

3. **Logical Operators**:
   - `And<P, Q>` - Conjunction (both hold)
   - `Implies<Q>` - Implication (P → Q)
   - `Is<T>` - Value inhabits type T
   - `True` - Always-true proposition

4. **Formal Verification**:
   - All composition functions verified with Kani
   - 183 symbolic checks prove soundness
   - Zero-cost abstraction (proofs are PhantomData)

---

## Architecture Impact on Botticelli

### Current State (Handrolled)

**Files to Remove** (~122 lines):
- `crates/botticelli_mcp/src/elicit_text.rs` (32 lines)
- `crates/botticelli_mcp/src/elicit_bool.rs` (28 lines)
- `crates/botticelli_mcp/src/elicit_number.rs` (36 lines)
- `crates/botticelli_mcp/src/elicit_select.rs` (26 lines)

**Files to Refactor** (~367 lines):
- `crates/botticelli_mcp/src/rmcp_server/tools/elicitation.rs`
  - Lines 22-172: 4 primitive tools (elicit_text, elicit_bool, elicit_number, elicit_select)
  - Lines 176-367: 3 domain-specific tools (elicit_metadata, elicit_act, elicit_carousel)

### New Architecture (Contract-Based)

**Option 1: Provider Trait Pattern** (Original Vision)

Keep the abstraction layer but use Tool contracts internally:

```rust
// Provider trait (unchanged from original vision)
pub trait ElicitationProvider {
    async fn get_text(&self, prompt: &str) -> Result<String, Error>;
    async fn get_bool(&self, prompt: &str, default: bool) -> Result<bool, Error>;
    async fn get_number(&self, prompt: &str, min: i64, max: i64) -> Result<i64, Error>;
    async fn get_select(&self, prompt: &str, options: &[&str]) -> Result<String, Error>;
}

// Human implementation delegates to Tool impls
pub struct HumanElicitation {
    dialog: Arc<DialogResource>,
}

impl ElicitationProvider for HumanElicitation {
    async fn get_text(&self, prompt: &str) -> Result<String, Error> {
        // Internally uses ElicitTextTool with contracts
        let tool = ElicitTextTool::new(self.dialog.clone());
        let (text, _proof) = tool.execute(prompt.to_string(), True::axiom()).await?;
        Ok(text)
    }
}

// Agent implementation uses sampling
pub struct AgentElicitation {
    driver: Arc<dyn BotticelliDriver>,
}

impl ElicitationProvider for AgentElicitation {
    async fn get_text(&self, prompt: &str) -> Result<String, Error> {
        // Internally uses ElicitTextTool with contracts
        let tool = ElicitTextToolAgent::new(self.driver.clone());
        let (text, _proof) = tool.execute(prompt.to_string(), True::axiom()).await?;
        Ok(text)
    }
}
```

**Option 2: Direct Tool Pattern** (Leverages Contracts Fully)

Skip the provider abstraction and use Tool trait directly:

```rust
// Primitive tools as Tool impls
pub struct ElicitTextTool<P: Protocol> {
    protocol: P,
}

// Protocol determines human vs agent
pub trait Protocol: Send + Sync {
    async fn elicit_text(&self, prompt: &str) -> Result<String, Error>;
}

// Human protocol (TUI)
pub struct HumanProtocol {
    dialog: Arc<DialogResource>,
}

impl Protocol for HumanProtocol {
    async fn elicit_text(&self, prompt: &str) -> Result<String, Error> {
        self.dialog.ask_text(prompt).await
    }
}

// Agent protocol (LLM sampling)
pub struct AgentProtocol {
    driver: Arc<dyn BotticelliDriver>,
}

impl Protocol for AgentProtocol {
    async fn elicit_text(&self, prompt: &str) -> Result<String, Error> {
        // Construct sampling request with prompt engineering
        let request = GenerateRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![Input::Text(format!(
                    "You are helping elicit information. {}\n\nProvide ONLY the requested value, no explanation.",
                    prompt
                ))],
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            ..Default::default()
        };
        
        let response = self.driver.generate(&request).await?;
        Ok(response.content.trim().to_string())
    }
}

// Tool implementation (generic over protocol)
impl<P: Protocol> Tool for ElicitTextTool<P> {
    type Input = String;    // Prompt
    type Output = String;   // Response
    type Pre = True;        // No precondition
    type Post = Is<String>; // Postcondition: valid String exists
    
    async fn execute(&self, prompt: String, _: Established<True>)
        -> Result<(String, Established<Is<String>>), ToolError>
    {
        let text = self.protocol.elicit_text(&prompt).await?;
        Ok((text, Established::assert()))
    }
}
```

### Comparison

| Aspect | Option 1 (Provider) | Option 2 (Direct) |
|--------|-------------------|------------------|
| **Abstraction** | High (hides Tool) | Low (exposes Tool) |
| **Contract Visibility** | Internal only | External API |
| **Composition** | Manual | via `then()`, `both_tools()` |
| **Verification** | Not exposed | Fully exposed |
| **Learning Curve** | Lower (familiar API) | Higher (contract system) |
| **Type Safety** | Good | Excellent |
| **Flexibility** | Medium | High |

**Recommendation**: Start with **Option 2 (Direct Tool Pattern)** because:
1. Leverages 0.4.7 contracts fully
2. Enables type-safe tool chains
3. Formal verification benefits
4. More flexible for future composition patterns
5. Can add Provider wrapper later if needed

---

## Implementation Plan

### Phase 1: Protocol Trait Design

**File**: `crates/botticelli_mcp/src/elicitation_protocol.rs` (NEW)

```rust
use crate::error::{ElicitationError, ElicitationResult};

/// Protocol for primitive elicitation operations.
///
/// This trait abstracts over human (TUI) and agent (LLM sampling) protocols.
/// Both implementations must provide the same four primitive operations.
#[async_trait::async_trait]
pub trait Protocol: Send + Sync {
    /// Elicit free-form text from user/agent.
    async fn elicit_text(&self, prompt: &str) -> ElicitationResult<String>;
    
    /// Elicit boolean confirmation from user/agent.
    async fn elicit_bool(&self, prompt: &str, default: bool) -> ElicitationResult<bool>;
    
    /// Elicit integer within range from user/agent.
    async fn elicit_number(
        &self,
        prompt: &str,
        min: i64,
        max: i64,
    ) -> ElicitationResult<i64>;
    
    /// Elicit selection from options.
    async fn elicit_select(
        &self,
        prompt: &str,
        options: &[&str],
    ) -> ElicitationResult<String>;
}

/// Human protocol implementation (interactive TUI).
pub struct HumanProtocol {
    dialog: Arc<DialogResource>,
}

impl HumanProtocol {
    pub fn new(dialog: Arc<DialogResource>) -> Self {
        Self { dialog }
    }
}

#[async_trait::async_trait]
impl Protocol for HumanProtocol {
    async fn elicit_text(&self, prompt: &str) -> ElicitationResult<String> {
        self.dialog.ask_text(prompt).await
            .map_err(|e| ElicitationError::dialog(e))
    }
    
    async fn elicit_bool(&self, prompt: &str, default: bool) -> ElicitationResult<bool> {
        self.dialog.ask_confirmation(prompt, default).await
            .map_err(|e| ElicitationError::dialog(e))
    }
    
    async fn elicit_number(
        &self,
        prompt: &str,
        min: i64,
        max: i64,
    ) -> ElicitationResult<i64> {
        self.dialog.ask_number(prompt, min, max).await
            .map_err(|e| ElicitationError::dialog(e))
    }
    
    async fn elicit_select(
        &self,
        prompt: &str,
        options: &[&str],
    ) -> ElicitationResult<String> {
        self.dialog.ask_choice(prompt, options).await
            .map_err(|e| ElicitationError::dialog(e))
    }
}

/// Agent protocol implementation (LLM sampling).
pub struct AgentProtocol {
    driver: Arc<dyn BotticelliDriver>,
}

impl AgentProtocol {
    pub fn new(driver: Arc<dyn BotticelliDriver>) -> Self {
        Self { driver }
    }
    
    /// Construct sampling request with prompt engineering for specific primitive.
    fn build_request(&self, prompt: &str, primitive_type: &str) -> GenerateRequest {
        GenerateRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![Input::Text(format!(
                    "You are helping elicit information.\n\n\
                    Task: {}\n\
                    Type: {}\n\n\
                    Provide ONLY the requested value, no explanation or formatting.\n\
                    For text: provide the literal text\n\
                    For bool: provide 'true' or 'false'\n\
                    For number: provide the integer\n\
                    For select: provide the exact option text",
                    prompt, primitive_type
                ))],
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl Protocol for AgentProtocol {
    async fn elicit_text(&self, prompt: &str) -> ElicitationResult<String> {
        let request = self.build_request(prompt, "text");
        let response = self.driver.generate(&request).await
            .map_err(|e| ElicitationError::sampling(e))?;
        Ok(response.content.trim().to_string())
    }
    
    async fn elicit_bool(&self, prompt: &str, default: bool) -> ElicitationResult<bool> {
        let request = self.build_request(prompt, "bool");
        let response = self.driver.generate(&request).await
            .map_err(|e| ElicitationError::sampling(e))?;
        
        let text = response.content.trim().to_lowercase();
        match text.as_str() {
            "true" | "yes" | "y" => Ok(true),
            "false" | "no" | "n" => Ok(false),
            _ => Ok(default), // Fallback to default
        }
    }
    
    async fn elicit_number(
        &self,
        prompt: &str,
        min: i64,
        max: i64,
    ) -> ElicitationResult<i64> {
        let request = self.build_request(prompt, "integer");
        let response = self.driver.generate(&request).await
            .map_err(|e| ElicitationError::sampling(e))?;
        
        let text = response.content.trim();
        let num = text.parse::<i64>()
            .map_err(|e| ElicitationError::parse(format!("Invalid number: {}", e)))?;
        
        // Clamp to range
        Ok(num.max(min).min(max))
    }
    
    async fn elicit_select(
        &self,
        prompt: &str,
        options: &[&str],
    ) -> ElicitationResult<String> {
        let options_str = options.join(", ");
        let enhanced_prompt = format!("{}\nOptions: {}", prompt, options_str);
        let request = self.build_request(&enhanced_prompt, "selection");
        
        let response = self.driver.generate(&request).await
            .map_err(|e| ElicitationError::sampling(e))?;
        
        let text = response.content.trim();
        
        // Try exact match first
        if options.contains(&text) {
            return Ok(text.to_string());
        }
        
        // Try case-insensitive match
        for option in options {
            if option.eq_ignore_ascii_case(text) {
                return Ok(option.to_string());
            }
        }
        
        // Fallback: return first option
        Ok(options[0].to_string())
    }
}
```

### Phase 2: Contract-Based Tool Implementations

**File**: `crates/botticelli_mcp/src/elicitation_tools.rs` (NEW)

```rust
use elicitation::{
    contracts::{Established, Is, Prop},
    tool::{Tool, True},
    ElicitResult,
};
use crate::elicitation_protocol::Protocol;

/// Tool for eliciting text with contracts.
pub struct ElicitTextTool<P: Protocol> {
    protocol: P,
}

impl<P: Protocol> ElicitTextTool<P> {
    pub fn new(protocol: P) -> Self {
        Self { protocol }
    }
}

impl<P: Protocol> Tool for ElicitTextTool<P> {
    type Input = String;    // Prompt
    type Output = String;   // Response
    type Pre = True;        // No precondition
    type Post = Is<String>; // Valid String exists
    
    async fn execute(
        &self,
        prompt: String,
        _pre: Established<True>,
    ) -> ElicitResult<(String, Established<Is<String>>)> {
        let text = self.protocol.elicit_text(&prompt).await?;
        Ok((text, Established::assert()))
    }
}

/// Tool for eliciting boolean with contracts.
pub struct ElicitBoolTool<P: Protocol> {
    protocol: P,
}

impl<P: Protocol> ElicitBoolTool<P> {
    pub fn new(protocol: P) -> Self {
        Self { protocol }
    }
}

// Define proposition for bool
struct BoolValue;
impl Prop for BoolValue {}

impl<P: Protocol> Tool for ElicitBoolTool<P> {
    type Input = (String, bool); // (Prompt, default)
    type Output = bool;
    type Pre = True;
    type Post = BoolValue;
    
    async fn execute(
        &self,
        (prompt, default): (String, bool),
        _pre: Established<True>,
    ) -> ElicitResult<(bool, Established<BoolValue>)> {
        let value = self.protocol.elicit_bool(&prompt, default).await?;
        Ok((value, Established::assert()))
    }
}

/// Tool for eliciting numbers with contracts.
pub struct ElicitNumberTool<P: Protocol> {
    protocol: P,
}

impl<P: Protocol> ElicitNumberTool<P> {
    pub fn new(protocol: P) -> Self {
        Self { protocol }
    }
}

// Proposition: number is within valid range
struct NumberInRange;
impl Prop for NumberInRange {}

impl<P: Protocol> Tool for ElicitNumberTool<P> {
    type Input = (String, i64, i64); // (Prompt, min, max)
    type Output = i64;
    type Pre = True;
    type Post = NumberInRange;
    
    async fn execute(
        &self,
        (prompt, min, max): (String, i64, i64),
        _pre: Established<True>,
    ) -> ElicitResult<(i64, Established<NumberInRange>)> {
        let num = self.protocol.elicit_number(&prompt, min, max).await?;
        Ok((num, Established::assert()))
    }
}

/// Tool for eliciting selections with contracts.
pub struct ElicitSelectTool<P: Protocol> {
    protocol: P,
}

impl<P: Protocol> ElicitSelectTool<P> {
    pub fn new(protocol: P) -> Self {
        Self { protocol }
    }
}

// Proposition: selection is valid option
struct ValidSelection;
impl Prop for ValidSelection {}

impl<P: Protocol> Tool for ElicitSelectTool<P> {
    type Input = (String, Vec<String>); // (Prompt, options)
    type Output = String;
    type Pre = True;
    type Post = ValidSelection;
    
    async fn execute(
        &self,
        (prompt, options): (String, Vec<String>),
        _pre: Established<True>,
    ) -> ElicitResult<(String, Established<ValidSelection>)> {
        let opts: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
        let selection = self.protocol.elicit_select(&prompt, &opts).await?;
        Ok((selection, Established::assert()))
    }
}
```

### Phase 3: MCP Server Integration

**File**: `crates/botticelli_mcp/src/rmcp_server/tools/elicitation.rs` (REFACTOR)

```rust
use elicitation::tool::{Tool, True};
use rmcp::service::RmcpTool;
use crate::elicitation_protocol::{HumanProtocol, AgentProtocol};
use crate::elicitation_tools::{ElicitTextTool, ElicitBoolTool, ElicitNumberTool, ElicitSelectTool};

/// Register elicitation tools with MCP server.
pub fn register_elicitation_tools(
    server: &mut RmcpServer,
    protocol: Arc<dyn Protocol>,
) {
    // Create tool instances with protocol
    let text_tool = ElicitTextTool::new(protocol.clone());
    let bool_tool = ElicitBoolTool::new(protocol.clone());
    let number_tool = ElicitNumberTool::new(protocol.clone());
    let select_tool = ElicitSelectTool::new(protocol.clone());
    
    // Register as MCP tools
    server.register_tool(RmcpTool::new(
        "elicit_text",
        "Elicit text input from user/agent",
        text_tool,
    ));
    
    server.register_tool(RmcpTool::new(
        "elicit_bool",
        "Elicit boolean confirmation from user/agent",
        bool_tool,
    ));
    
    server.register_tool(RmcpTool::new(
        "elicit_number",
        "Elicit integer from user/agent within range",
        number_tool,
    ));
    
    server.register_tool(RmcpTool::new(
        "elicit_select",
        "Elicit selection from options",
        select_tool,
    ));
}

// Human session helper
pub fn create_human_session(dialog: Arc<DialogResource>) -> Arc<dyn Protocol> {
    Arc::new(HumanProtocol::new(dialog))
}

// Agent session helper
pub fn create_agent_session(driver: Arc<dyn BotticelliDriver>) -> Arc<dyn Protocol> {
    Arc::new(AgentProtocol::new(driver))
}
```

### Phase 4: Delete Redundant Code

1. **Delete files** (~122 lines):
   - `crates/botticelli_mcp/src/elicit_text.rs`
   - `crates/botticelli_mcp/src/elicit_bool.rs`
   - `crates/botticelli_mcp/src/elicit_number.rs`
   - `crates/botticelli_mcp/src/elicit_select.rs`

2. **Update lib.rs**:
   ```rust
   // Remove old exports
   // pub mod elicit_text;
   // pub mod elicit_bool;
   // pub mod elicit_number;
   // pub mod elicit_select;
   
   // Add new exports
   pub mod elicitation_protocol;
   pub mod elicitation_tools;
   ```

### Phase 5: Testing

**File**: `tests/elicitation_contracts_test.rs` (NEW)

```rust
use botticelli_mcp::{
    elicitation_protocol::{HumanProtocol, AgentProtocol},
    elicitation_tools::ElicitTextTool,
};
use elicitation::tool::{Tool, True};

#[tokio::test]
async fn test_human_protocol_contracts() {
    // Mock DialogResource
    let dialog = Arc::new(MockDialogResource::new());
    let protocol = HumanProtocol::new(dialog);
    let tool = ElicitTextTool::new(protocol);
    
    // Execute with contract
    let (text, proof) = tool.execute("Enter name:".to_string(), True::axiom())
        .await
        .expect("Should succeed");
    
    assert_eq!(text, "Alice");
    // Proof is zero-sized
    assert_eq!(std::mem::size_of_val(&proof), 0);
}

#[tokio::test]
async fn test_agent_protocol_contracts() {
    // Mock BotticelliDriver
    let driver = Arc::new(MockDriver::new());
    let protocol = AgentProtocol::new(driver);
    let tool = ElicitTextTool::new(protocol);
    
    // Execute with contract
    let (text, proof) = tool.execute("Enter name:".to_string(), True::axiom())
        .await
        .expect("Should succeed");
    
    assert!(text.len() > 0);
    // Proof is zero-sized
    assert_eq!(std::mem::size_of_val(&proof), 0);
}

#[tokio::test]
async fn test_tool_composition() {
    use elicitation::tool::then;
    
    // Create two tools
    let dialog = Arc::new(MockDialogResource::new());
    let protocol = HumanProtocol::new(dialog);
    let text_tool = ElicitTextTool::new(protocol.clone());
    let validate_tool = ValidateEmailTool::new(); // Custom domain tool
    
    // Compose: elicit text, then validate as email
    let pipeline = then(text_tool, validate_tool);
    
    // Execute composed tool
    let (email, proof) = pipeline.execute("Enter email:".to_string(), True::axiom())
        .await
        .expect("Should succeed");
    
    assert!(email.contains('@'));
    // proof now carries EmailValidated postcondition
}
```

---

## Benefits of Contract-Based Architecture

### 1. Type Safety
- Cannot call tools without establishing preconditions
- Postconditions automatically established by tool execution
- Compiler prevents invalid tool chains

### 2. Formal Verification
- All composition operators verified with Kani
- 183 symbolic checks prove soundness
- Mathematical guarantees, not just testing

### 3. Zero Cost
- All proofs are `PhantomData<fn() -> T>`
- Compile away completely
- No runtime overhead

### 4. Composability
- Sequential: `then(t1, t2)` chains tools
- Parallel: `both_tools(t1, t2)` combines independent tools
- Complex workflows: nest arbitrarily deep

### 5. Protocol Abstraction
- Same Tool impls work for human and agent
- Protocol trait isolates elicitation mechanism
- Easy to add new protocols (e.g., HTTP API, Discord bot)

### 6. Integration with Elicitation Ecosystem
- Leverages rmcp integration
- Works with elicitation derive macros
- Compatible with existing tool system

---

## Migration Guide

### For Existing Code

**Before** (handrolled types):
```rust
let params = ElicitTextParams { prompt: "Name?".to_string() };
let result = elicit_text(params).await?;
println!("Got: {}", result.value);
```

**After** (contract-based tools):
```rust
let protocol = create_human_session(dialog);
let tool = ElicitTextTool::new(protocol);
let (text, _proof) = tool.execute("Name?".to_string(), True::axiom()).await?;
println!("Got: {}", text);
```

### For New Code

Use composition operators to build multi-step workflows:

```rust
use elicitation::tool::{then, both_tools};

// Sequential: validate email, then send
let workflow = then(validate_email_tool, send_email_tool);
let (sent, proof) = workflow.execute(email, True::axiom()).await?;

// Parallel: validate email AND check consent
let workflow = both_tools(validate_email_tool, check_consent_tool);
let ((email, consent), proof) = workflow.execute(input, True::axiom()).await?;
```

---

## Next Steps

1. **Implement Protocol trait** (Phase 1):
   - Create `elicitation_protocol.rs`
   - Implement `HumanProtocol` (wraps DialogResource)
   - Implement `AgentProtocol` (uses LLM sampling)

2. **Implement Tool types** (Phase 2):
   - Create `elicitation_tools.rs`
   - Implement 4 primitive tools with contracts
   - Add domain propositions (BoolValue, NumberInRange, etc.)

3. **Refactor MCP server** (Phase 3):
   - Update `rmcp_server/tools/elicitation.rs`
   - Remove custom types, use Tool trait
   - Add session creation helpers

4. **Delete redundant code** (Phase 4):
   - Remove 4 type definition files
   - Update lib.rs exports

5. **Test and verify** (Phase 5):
   - Add contract-based tests
   - Test human protocol (mock DialogResource)
   - Test agent protocol (mock BotticelliDriver)
   - Verify tool composition

---

## References

- **Elicitation 0.4.7 CHANGELOG**: `../elicitation/CHANGELOG.md`
- **Contracts documentation**: `../elicitation/crates/elicitation/src/contracts.rs`
- **Tool trait documentation**: `../elicitation/crates/elicitation/src/tool.rs`
- **Kani verification**: `../elicitation/crates/elicitation/src/kani_tests.rs`
- **PARALLEL_IMPLEMENTATION_ANALYSIS.md**: Original provider vision document
