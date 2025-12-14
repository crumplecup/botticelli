# MCP TypeScript Specification to Rust Types - Strategy Document

**Date:** 2024-12-14  
**Status:** Strategy Proposal  
**Context:** Clean Architecture Refactor Complete - Evaluating Next Phase

## Executive Summary

**Proposition:** Create a comprehensive set of Rust types directly derived from the official MCP TypeScript specification to encode protocol invariants in the type system for compile-time verification of spec compliance.

**Evaluation:**
- **Feasibility:** ✅ HIGH - TypeScript → Rust translation is well-understood
- **Practicality:** ⚠️ MIXED - Significant upfront cost, ongoing maintenance burden
- **Value:** ✅ HIGH for MCP server, ⚠️ LOW for LLM provider abstraction

**Recommendation:** **TARGETED ADOPTION** - Focus on MCP server types, keep existing provider abstraction

---

## Background

### Current State

**botticelli_core:**
- Generic `Input`/`Output` enums for provider abstraction
- `GenerateRequest`/`GenerateResponse` types
- `StopReason` enum (recently made MCP-compliant)
- Provider-agnostic design

**botticelli_mcp:**
- Server implementation with MCP protocol support
- Manual TypeScript → Rust translations
- Ad-hoc type definitions

**Problem:**
- Dual purpose creates tension: provider abstraction vs MCP compliance
- Manual synchronization with spec updates
- No compile-time guarantee of MCP compliance
- Potential drift between spec and implementation

### MCP Specification

**Location:** https://github.com/modelcontextprotocol/modelcontextprotocol  
**Schema of Record:** `schema/2025-11-25/schema.ts` (2578 lines)  
**Protocol Version:** 2025-11-25 (versioned schema)

**Key Features:**
- Comprehensive TypeScript definitions
- JSON-RPC 2.0 foundation
- Versioned schemas (supports multiple versions)
- Rich type hierarchy with invariants

**Core Type Categories:**
1. **JSON-RPC primitives** - Request, Response, Notification, Error
2. **Content types** - Text, Image, Audio, ToolUse, ToolResult
3. **Resources** - Templates, Contents, Subscriptions
4. **Prompts** - Arguments, Messages, EmbeddedResources
5. **Tools** - Definitions, Calls, Results, Annotations
6. **Sampling** - Messages, CreateMessage, StopReason
7. **Roots** - Listing, Notifications
8. **Capabilities** - Client/Server negotiation
9. **Pagination** - Cursors, PaginatedResult
10. **Tasks** - Augmented execution, metadata

---

## Feasibility Analysis

### TypeScript → Rust Translation Patterns

#### 1. Type Aliases (TRIVIAL)

**TypeScript:**
```typescript
export type ProgressToken = string | number;
export type Cursor = string;
export type RequestId = string | number;
```

**Rust:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    String(String),
    Number(i64),
}

pub type Cursor = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}
```

**Challenges:** None - straightforward translation

#### 2. Interfaces (STRAIGHTFORWARD)

**TypeScript:**
```typescript
export interface TextContent {
  type: "text";
  text: string;
  annotations?: Annotations;
  _meta?: { [key: string]: unknown };
}
```

**Rust:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    type_: String, // or use const generic/"text" literal
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<serde_json::Map<String, serde_json::Value>>,
}
```

**Challenges:**
- Literal type `"text"` → const or validation
- `unknown` type → `serde_json::Value`
- Optional fields → `Option<T>`

#### 3. Tagged Unions (MODERATE)

**TypeScript:**
```typescript
export type ContentBlock =
  | TextContent
  | ImageContent
  | AudioContent
  | ResourceLink
  | EmbeddedResource;
```

**Rust (External Tagging):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "image")]
    Image(ImageContent),
    #[serde(rename = "audio")]
    Audio(AudioContent),
    #[serde(rename = "resource")]
    ResourceLink(ResourceLink),
    #[serde(rename = "embedded")]
    EmbeddedResource(EmbeddedResource),
}
```

**Challenges:**
- Choosing correct serde tag strategy
- MCP uses `type` discriminator
- Need `#[serde(tag = "type")]`

#### 4. Literal Types (MODERATE)

**TypeScript:**
```typescript
export interface TextContent {
  type: "text";  // Literal type
  text: string;
}
```

**Rust Options:**

**A. Runtime Validation (Simpler):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub type_field: String,  // Validate == "text" at runtime
    pub text: String,
}
```

**B. Phantom Type (Compile-time):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content<T> {
    #[serde(rename = "type", skip_serializing, skip_deserializing)]
    _type: PhantomData<T>,
    pub text: String,
}

pub struct TextType;
pub type TextContent = Content<TextType>;

impl Serialize for Content<TextType> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Inject "type": "text"
    }
}
```

**Challenges:**
- Encoding literal string constraints
- Serde doesn't natively support literal types
- Trade-off: complexity vs safety

#### 5. Constraints (CHALLENGING)

**TypeScript:**
```typescript
/**
 * @TJS-type number
 * @minimum 0
 * @maximum 1
 */
priority?: number;
```

**Rust:**
```rust
/// Priority value between 0.0 and 1.0
#[derive(Debug, Clone, Copy)]
pub struct Priority(f64);

impl Priority {
    pub fn new(value: f64) -> Result<Self, PriorityError> {
        if !(0.0..=1.0).contains(&value) {
            return Err(PriorityError::OutOfRange(value));
        }
        Ok(Self(value))
    }
}

impl Serialize for Priority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Priority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}
```

**Challenges:**
- JSDoc constraints → Rust newtype validation
- Must balance ergonomics with safety
- Custom ser/de implementations

### Tooling Options

#### 1. Manual Translation (Current)
- **Pros:** Full control, tailored to Rust idioms
- **Cons:** Error-prone, maintenance burden, drift risk

#### 2. ts-rs or quicktype
- **Pros:** Automated, consistent
- **Cons:** Generic output, may not respect constraints, needs post-processing

#### 3. Hybrid: Generate + Refine
- **Pros:** Automated baseline + manual refinement
- **Cons:** Still needs review, but reduces initial work

**Recommendation:** Hybrid approach with CI validation

---

## Practicality Analysis

### Implementation Cost

#### Phase 1: Core Types (HIGH COST)
**Estimated Effort:** 2-3 weeks

**Tasks:**
1. Clone MCP spec repository
2. Parse `schema/2025-11-25/schema.ts`
3. Generate baseline Rust types (quicktype or manual)
4. Refine with proper newtype wrappers
5. Add validation logic
6. Implement serde traits
7. Write comprehensive tests
8. Document type correspondences

**Output:** `botticelli_mcp_types` crate (~5000 lines)

#### Phase 2: Integration (MODERATE COST)
**Estimated Effort:** 1 week

**Tasks:**
1. Update `botticelli_mcp` to use new types
2. Implement conversion traits (`From`/`TryFrom`)
3. Update tool implementations
4. Fix compilation errors
5. Update tests
6. Validate against MCP spec examples

#### Phase 3: CI/CD Integration (LOW COST)
**Estimated Effort:** 2-3 days

**Tasks:**
1. Add schema validation tests
2. CI job to check spec version
3. Alert on spec updates
4. Automated compatibility checks

### Maintenance Burden

#### Ongoing Costs

**1. Spec Updates (MEDIUM)**
- **Frequency:** Quarterly (estimated)
- **Effort per update:** 1-3 days
- **Risk:** Breaking changes in MCP protocol

**Mitigation:**
- Version pinning in crate
- Support multiple schema versions
- Automated diff detection

**2. Bug Fixes (LOW)**
- **Frequency:** As needed
- **Effort:** Case-by-case
- **Risk:** Type system mismatches

**3. Feature Additions (VARIABLE)**
- **Frequency:** When MCP adds features
- **Effort:** Depends on complexity

**Total Annual Maintenance:** ~2-4 weeks/year

### Compatibility

#### Benefits

✅ **Compile-time guarantees:**
```rust
// Won't compile if missing required field
let content = TextContent {
    type_: "text".to_string(),
    text: "Hello".to_string(),
    // annotations: None,  // Optional - OK
    // Missing _meta is fine
};

// Won't compile with wrong discriminator
let content = ContentBlock::Text(ImageContent { ... });  // Type error
```

✅ **Validation at boundaries:**
```rust
#[derive(Debug, Serialize, Deserialize)]
struct Priority(#[serde(deserialize_with = "validate_priority")] f64);

fn validate_priority<'de, D>(deserializer: D) -> Result<f64, D::Error>
where D: Deserializer<'de>
{
    let value = f64::deserialize(deserializer)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(serde::de::Error::custom("Priority must be 0.0-1.0"));
    }
    Ok(value)
}
```

✅ **Spec version enforcement:**
```rust
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    "2025-11-25",
    "2025-06-18", // Backward compat
];

pub fn validate_protocol_version(version: &str) -> Result<(), SpecError> {
    if !SUPPORTED_PROTOCOL_VERSIONS.contains(&version) {
        return Err(SpecError::UnsupportedVersion(version.to_string()));
    }
    Ok(())
}
```

#### Challenges

⚠️ **Duplication with provider types:**
- `botticelli_core::Input` vs `botticelli_mcp_types::ContentBlock`
- `botticelli_core::GenerateResponse` vs `botticelli_mcp_types::CreateMessageResult`
- Conversion overhead

⚠️ **Inflexibility:**
- Hard to extend with non-spec fields
- Backward compatibility burden
- Vendor-specific extensions problematic

⚠️ **Complexity:**
- Large type hierarchy
- Many generic parameters
- Harder to understand for contributors

---

## Value Analysis

### Benefits

#### 1. MCP Server Compliance (HIGH VALUE)

**Current Risk:** Manual type definitions may drift from spec

**With Spec Types:**
```rust
// botticelli_mcp server
use botticelli_mcp_types::*;

impl McpServer {
    async fn handle_tools_list(&self, _params: ListToolsRequestParams) 
        -> Result<ListToolsResult, JSONRPCError> 
    {
        // Type system ensures correct structure
        Ok(ListToolsResult {
            tools: self.registry.tool_definitions()
                .into_iter()
                .map(|def| Tool {
                    name: def.name,
                    description: def.description,
                    inputSchema: def.input_schema,
                    // Compiler ensures all required fields present
                })
                .collect(),
            nextCursor: None,  // Pagination
            _meta: None,
        })
    }
}
```

**Value:**
- ✅ Guaranteed spec compliance
- ✅ Automatic validation
- ✅ Reduced testing burden (type system catches errors)
- ✅ Better interoperability with other MCP implementations

#### 2. Protocol Evolution (MODERATE VALUE)

**Scenario:** MCP 2025-12-01 adds required `version` field to `Tool`

**Without Spec Types:**
- Runtime error when clients expect field
- Manual code inspection to find all usage
- Tests may pass but runtime fails

**With Spec Types:**
```rust
// After updating to new schema version
pub struct Tool {
    pub name: String,
    pub version: String,  // NEW REQUIRED FIELD
    // ...
}

// Code won't compile until fixed
let tool = Tool {
    name: "echo".to_string(),
    // version: "1.0".to_string(),  // MISSING - compiler error
    // ...
};
```

**Value:**
- ✅ Breaking changes caught at compile time
- ✅ Forced systematic updates
- ✅ No hidden runtime failures

#### 3. Documentation (LOW-MODERATE VALUE)

**Spec Types:**
```rust
/// Text provided to or from an LLM.
///
/// **MCP Specification:** 2025-11-25
/// **TypeScript Definition:** `schema.ts:1750`
/// 
/// # Examples
/// ```
/// # use botticelli_mcp_types::TextContent;
/// let content = TextContent {
///     type_: "text".to_string(),
///     text: "Hello, world!".to_string(),
///     annotations: None,
///     _meta: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    // ...
}
```

**Value:**
- ✅ Clear spec mapping
- ✅ Examples from spec
- ✅ Version tracking

### Costs

#### 1. Provider Abstraction Dilution (HIGH COST)

**Current Design Philosophy:**
```rust
// Provider-agnostic - works with ANY LLM
pub trait LlmProvider {
    async fn generate(&self, request: &GenerateRequest) 
        -> Result<GenerateResponse, ProviderError>;
}

// Simple generic types
pub enum Input {
    Text(String),
    Image { mime: Option<String>, data: Vec<u8> },
    // ... minimal set
}
```

**With MCP Types:**
```rust
// Now tied to MCP spec
use botticelli_mcp_types::*;

pub trait LlmProvider {
    async fn generate(&self, request: &CreateMessageRequest)  // MCP-specific
        -> Result<CreateMessageResult, ProviderError>;  // MCP-specific
}

// Must map provider-specific types to MCP
impl LlmProvider for AnthropicClient {
    async fn generate(&self, request: &CreateMessageRequest) 
        -> Result<CreateMessageResult, ProviderError> 
    {
        // Convert MCP → Anthropic format
        let anthropic_request = self.convert_request(request)?;
        
        // Call API
        let anthropic_response = self.client.messages(anthropic_request).await?;
        
        // Convert Anthropic → MCP format (extra work!)
        self.convert_response(anthropic_response)
    }
}
```

**Problems:**
- ❌ MCP is sampling-focused (messages, tool use)
- ❌ Not all providers fit MCP model (embeddings, completion, etc.)
- ❌ Extra conversion layer for every provider
- ❌ Tight coupling to one protocol
- ❌ Harder to add non-MCP providers (local models, etc.)

**Impact:** Undermines clean architecture goals

#### 2. Development Velocity (MODERATE COST)

**Scenario:** Adding image support

**Current:**
```rust
// Just add variant
pub enum Input {
    Text(String),
    Image { mime: Option<String>, data: Vec<u8> },  // Easy!
}
```

**With Spec Types:**
```rust
// Must match MCP ImageContent exactly
pub struct ImageContent {
    type_: String,  // Must be "image"
    data: String,   // Must be base64
    mimeType: String,  // Different name than our convention
    annotations: Option<Annotations>,  // Must support even if unused
    _meta: Option<MetaObject>,  // Must support
}

// Conversion overhead
impl From<our::Image> for mcp::ImageContent {
    fn from(img: our::Image) -> Self {
        ImageContent {
            type_: "image".to_string(),
            data: base64::encode(&img.data),
            mimeType: img.mime.unwrap_or("image/png".to_string()),
            annotations: None,  // Extra fields we don't use
            _meta: None,
        }
    }
}
```

**Impact:** More boilerplate, slower iteration

#### 3. Learning Curve (MODERATE COST)

- New contributors must understand MCP spec
- More complex type hierarchy
- Harder to navigate codebase
- More imports needed

---

## Recommendations

### Strategy: TARGETED ADOPTION

**Thesis:** Use MCP types where they provide clear value (MCP server), keep existing abstraction where they don't (LLM providers).

### Architecture

```
┌──────────────────────────────────────────────────────────┐
│               Application Layer (botticelli)              │
└────────────────────────┬─────────────────────────────────┘
                         │
        ┌────────────────┴────────────────┐
        │                                 │
        ↓                                 ↓
┌───────────────────────┐    ┌────────────────────────────┐
│  Provider Abstraction │    │      MCP Server            │
│  (botticelli_core)    │    │  (botticelli_mcp)          │
│                       │    │                            │
│  Generic types:       │    │  Spec types:               │
│  • Input/Output       │    │  • botticelli_mcp_types::* │
│  • GenerateRequest    │    │  • Full MCP compliance     │
│  • LlmProvider trait  │    │  • JSON-RPC 2.0            │
└───────────────────────┘    └────────────────────────────┘
        │                                 │
        ↓                                 ↓
┌───────────────────────┐    ┌────────────────────────────┐
│  Provider Impls       │    │  Type Conversions          │
│  (botticelli_models)  │    │  core::Input ↔ mcp::Content│
└───────────────────────┘    └────────────────────────────┘
```

**Key Principle:** **Separation of Concerns**
- **Provider layer:** Minimal, generic, provider-agnostic types
- **MCP layer:** Full spec compliance with official types
- **Bridge:** Conversion traits between layers

### Implementation Plan

#### Phase 1: Create `botticelli_mcp_types` Crate (3 weeks)

**Goal:** Faithful Rust representation of MCP TypeScript spec

**Scope:**
- All types from `schema/2025-11-25/schema.ts`
- Proper serde configuration
- Validation for constraints
- Comprehensive doc comments with spec references
- Test suite against spec examples

**Out of Scope:**
- Provider integration (keep separate)
- Application-specific extensions

**Structure:**
```
crates/botticelli_mcp_types/
├── src/
│   ├── lib.rs              # Re-exports
│   ├── jsonrpc.rs          # JSON-RPC 2.0 types
│   ├── content.rs          # Content types (Text, Image, etc.)
│   ├── resources.rs        # Resource types
│   ├── prompts.rs          # Prompt types
│   ├── tools.rs            # Tool types
│   ├── sampling.rs         # Sampling/CreateMessage types
│   ├── roots.rs            # Root listing
│   ├── capabilities.rs     # Client/Server capabilities
│   ├── pagination.rs       # Cursor pagination
│   ├── tasks.rs            # Task augmentation
│   ├── validation.rs       # Constraint validation
│   └── version.rs          # Protocol versioning
├── tests/
│   ├── spec_examples/      # JSON from spec docs
│   └── roundtrip_test.rs   # Serde roundtrip tests
└── Cargo.toml
```

**Dependencies:**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

#### Phase 2: Integrate into `botticelli_mcp` (1 week)

**Goal:** Replace manual MCP types with spec types in server

**Changes:**
```rust
// Before
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

// After
use botticelli_mcp_types::Tool;  // From spec

impl McpServer {
    pub fn handle_tools_list(&self) -> ListToolsResult {
        ListToolsResult {
            tools: self.registry.tools()
                .map(|t| Tool {
                    name: t.name().to_string(),
                    description: Some(t.description().to_string()),
                    inputSchema: t.schema(),
                    _meta: None,
                })
                .collect(),
            nextCursor: None,
            _meta: None,
        }
    }
}
```

**Conversions:**
```rust
// Bridge between core and MCP types
impl From<botticelli_core::Input> for botticelli_mcp_types::ContentBlock {
    fn from(input: botticelli_core::Input) -> Self {
        match input {
            Input::Text(text) => ContentBlock::Text(TextContent {
                type_: "text".to_string(),
                text,
                annotations: None,
                _meta: None,
            }),
            Input::Image { mime, data } => ContentBlock::Image(ImageContent {
                type_: "image".to_string(),
                data: base64::encode(data),
                mimeType: mime.unwrap_or("image/png".to_string()),
                annotations: None,
                _meta: None,
            }),
            // ... other variants
        }
    }
}
```

#### Phase 3: CI/CD Integration (2 days)

**Goal:** Automated spec compliance monitoring

**CI Jobs:**
1. **Spec Version Check:**
   ```yaml
   - name: Check MCP Spec Version
     run: |
       SPEC_VERSION=$(grep LATEST_PROTOCOL_VERSION crates/botticelli_mcp_types/src/version.rs)
       UPSTREAM_VERSION=$(curl -s https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts | grep LATEST_PROTOCOL_VERSION)
       if [ "$SPEC_VERSION" != "$UPSTREAM_VERSION" ]; then
         echo "⚠️ MCP spec updated: $UPSTREAM_VERSION"
         exit 1
       fi
   ```

2. **Roundtrip Tests:**
   ```rust
   #[test]
   fn test_spec_example_roundtrip() {
       let json = include_str!("../spec_examples/tools_list.json");
       let typed: ListToolsResult = serde_json::from_str(json).unwrap();
       let serialized = serde_json::to_string(&typed).unwrap();
       // Compare semantically
       assert_json_eq!(json, serialized);
   }
   ```

3. **Schema Validation:**
   - Use JSON Schema to validate generated JSON
   - Compare against `schema.json` from MCP repo

#### Phase 4: Documentation (1 day)

**Deliverables:**
1. MCP_TYPES_GUIDE.md - mapping TS→Rust
2. Update CLEAN_SAMPLING_USAGE_GUIDE.md with MCP types
3. Migration guide for server code
4. Crate-level docs with spec links

### Non-Goals

**❌ Do NOT:**
1. Replace `botticelli_core` types with MCP types
2. Force all providers through MCP types
3. Support all MCP versions simultaneously (pick one, upgrade gracefully)
4. Auto-generate from TypeScript (manual is more idiomatic)
5. Expose MCP types in application APIs

### Success Criteria

✅ **Phase 1 Complete When:**
- All MCP types implemented
- 100% serialization/deserialization coverage
- Spec examples pass roundtrip tests
- Documentation maps TS→Rust 1:1

✅ **Phase 2 Complete When:**
- `botticelli_mcp` server uses spec types
- No manual MCP type definitions remain
- All server tests pass
- Conversion layer tested

✅ **Phase 3 Complete When:**
- CI monitors spec updates
- Automated validation in place
- Alerts configured

✅ **Overall Success:**
- MCP server 100% spec compliant
- Provider abstraction unchanged
- Zero regression in functionality
- Maintenance burden <1 day/month

---

## Risk Analysis

### High Risks

**1. Spec Instability (MEDIUM PROBABILITY, HIGH IMPACT)**

**Risk:** MCP spec changes frequently, breaking types

**Mitigation:**
- Pin to specific protocol version
- Support backward compatibility
- Automated CI alerts on upstream changes
- Versioned crate releases

**2. Conversion Overhead (LOW PROBABILITY, MODERATE IMPACT)**

**Risk:** Performance penalty from core↔MCP conversions

**Mitigation:**
- Profile hotspots
- Zero-copy where possible
- Benchmark conversion paths
- Consider `Cow<'a, T>` for borrowed data

### Medium Risks

**3. Maintenance Burden (MEDIUM PROBABILITY, MODERATE IMPACT)**

**Risk:** Team spends >2 days/month maintaining types

**Mitigation:**
- Automated tooling for updates
- Clear ownership (one maintainer)
- Quarterly update cycle (not immediate)
- Good documentation reduces questions

**4. Contributor Confusion (MEDIUM PROBABILITY, LOW IMPACT)**

**Risk:** Two type systems confuse new contributors

**Mitigation:**
- Clear architecture docs
- Naming conventions: `mcp::*` vs `core::*`
- Examples in both systems
- Good crate-level documentation

### Low Risks

**5. Type System Limitations (LOW PROBABILITY, LOW IMPACT)**

**Risk:** Rust can't express some TypeScript constraints

**Mitigation:**
- Runtime validation as fallback
- Document any relaxed constraints
- Thorough testing compensates

---

## Alternatives Considered

### Alternative 1: Status Quo

**Keep manual types, improve testing**

**Pros:**
- No implementation cost
- Maximum flexibility
- Simple architecture

**Cons:**
- Drift risk remains
- Manual synchronization
- No compile-time guarantees

**Verdict:** ❌ Rejected - spec compliance too important for MCP server

### Alternative 2: Full Replacement

**Replace `botticelli_core` types with MCP types everywhere**

**Pros:**
- Single source of truth
- Maximum spec compliance
- Consistent types throughout

**Cons:**
- Destroys provider abstraction
- Forces non-MCP providers through MCP types
- Tight coupling to one protocol
- Slower development velocity

**Verdict:** ❌ Rejected - violates clean architecture principles

### Alternative 3: JSON Schema Validation

**Keep Rust types, validate JSON at runtime**

**Pros:**
- No type translation needed
- Runtime flexibility

**Cons:**
- Runtime cost
- No compile-time safety
- Harder to debug failures
- Doesn't leverage Rust's type system

**Verdict:** ❌ Rejected - defeats purpose of type safety

### Alternative 4: Hybrid (Recommended)

**MCP types for server, keep core abstraction**

**Pros:**
- Targeted compliance where needed
- Preserves provider abstraction
- Best of both worlds

**Cons:**
- Two type systems
- Conversion layer needed

**Verdict:** ✅ **RECOMMENDED** - pragmatic balance

---

## Conclusion

### Summary

**RECOMMENDED APPROACH:** Targeted adoption of MCP spec types in `botticelli_mcp` server implementation while preserving generic provider abstraction in `botticelli_core`.

**RATIONALE:**
1. **High value for MCP server** - compile-time spec compliance, automated validation, reduced testing burden
2. **Low cost** - isolated to one crate, clear boundaries
3. **Preserves architecture** - doesn't break provider abstraction
4. **Manageable maintenance** - pinned versions, automated monitoring
5. **Pragmatic** - apply tool where it helps, avoid where it hurts

### Implementation Timeline

**Total Effort:** 5 weeks

- Week 1-3: Create `botticelli_mcp_types` crate
- Week 4: Integrate into `botticelli_mcp` server
- Week 5: CI/CD integration + documentation

**First Release:** botticelli_mcp_types v0.1.0 (targets MCP 2025-11-25)

### Success Metrics

**6 months post-launch:**
- ✅ Zero MCP spec compliance issues reported
- ✅ CI catches spec updates automatically
- ✅ <1 day/month maintenance time
- ✅ Provider abstraction unchanged
- ✅ Developer satisfaction high

### Next Steps

1. **Approval:** Stakeholder review of strategy
2. **Planning:** Break Phase 1 into 2-week sprints
3. **Research:** Evaluate ts-rs vs quicktype vs manual
4. **Prototype:** Implement 5 core types as proof-of-concept
5. **Review:** Validate approach before full implementation

---

## Appendix A: Type Translation Examples

### Example 1: Simple Interface

**TypeScript:**
```typescript
export interface Cursor {
  cursor?: string;
  [key: string]: unknown;
}
```

**Rust:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    
    #[serde(flatten)]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
}
```

### Example 2: Tagged Union

**TypeScript:**
```typescript
export type StopReason = 
  | "endTurn"
  | "maxTokens" 
  | "stopSequence"
  | "tool_use";
```

**Rust:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}
```

### Example 3: Constrained Type

**TypeScript:**
```typescript
/**
 * @minimum 0
 * @maximum 1
 */
priority?: number;
```

**Rust:**
```rust
#[derive(Debug, Clone, Copy)]
pub struct Priority(f64);

impl Priority {
    pub fn new(value: f64) -> Result<Self, ValueError> {
        if !(0.0..=1.0).contains(&value) {
            return Err(ValueError::OutOfRange {
                field: "priority",
                min: 0.0,
                max: 1.0,
                actual: value,
            });
        }
        Ok(Self(value))
    }
    
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Priority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Priority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer
    {
        self.0.serialize(serializer)
    }
}
```

---

## Appendix B: Maintenance Playbook

### When MCP Spec Updates

**1. Notification:**
- CI job fails with version mismatch
- GitHub issue auto-created
- Maintainer notified

**2. Assessment (30 min):**
- Review changelog: https://github.com/modelcontextprotocol/modelcontextprotocol/releases
- Identify breaking changes
- Categorize: patch, minor, major

**3. Implementation:**

**Patch (typos, docs):** 30 min
- Update comments
- No code changes needed

**Minor (new optional fields):** 2-4 hours
- Add new fields as `Option<T>`
- Update tests
- Backward compatible

**Major (breaking changes):** 1-2 days
- Create new types alongside old
- Deprecation warnings
- Migration guide
- Support both versions briefly

**4. Release:**
- Version bump: `botticelli_mcp_types`
- Changelog entry
- Update `botticelli_mcp` dependency
- Integration testing
- Publish

**5. Communication:**
- Release notes
- Migration guide if breaking
- Update documentation

**Target SLA:** <1 week from spec release to types release

---

## Appendix C: References

1. **MCP Specification:** https://github.com/modelcontextprotocol/modelcontextprotocol
2. **TypeScript Schema:** https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2025-11-25/schema.ts
3. **MCP Documentation:** https://modelcontextprotocol.io/specification
4. **Serde Documentation:** https://serde.rs/
5. **Rust JSON-RPC:** https://docs.rs/jsonrpc-core/latest/jsonrpc_core/
6. **Clean Architecture Refactor:** `NARRATIVE_SAMPLING_CLEAN_ARCHITECTURE.md`
7. **Current MCP Usage:** `CLEAN_SAMPLING_USAGE_GUIDE.md`

---

**Document Status:** DRAFT - Awaiting Review  
**Author:** Claude (AI Assistant)  
**Reviewers:** TBD  
**Approval:** Pending
