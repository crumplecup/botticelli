# Model Selection Strategy

## Implementation Status

- [x] Step 1: Model enums with hierarchical ordering ✅
  - ModelFamily enum with fallback_order()
  - GeminiModel enum (5 variants, move_up/down, friends())
  - GroqModel enum (4 variants, move_up/down, friends())
  - All tests passing
- [x] Step 2: Boundary constraints API ✅
  - ModelBounds struct with lower/upper bounds
  - ModelId unified enum across families
  - is_at_least()/is_at_most() for bound checking
  - 10 tests passing (bounds validation, movement, comparisons)
- [ ] Step 3: Rate limit detection
- [ ] Step 4: Fallback selection algorithm
- [ ] Step 5: Integration with chat interface
- [ ] Step 6: Configuration and persistence
- [ ] Step 7: Testing and documentation

---

## Vision

Build a hierarchical model selection system that automatically handles rate limits by moving "loyally" (within family, up/down capability tiers) or "friendly" (across families, lateral equivalence) while respecting user-defined boundaries.

### Core Concepts

**Loyal Movement**: Stay within model family, move up/down capability tiers
- `gemini-2.5-pro` → `gemini-2.5-flash` → `gemini-2.5-flash-lite`
- Higher index = more restrictive/cheaper/faster

**Friendly Movement**: Switch to equivalent model in different family
- `gemini-2.5-flash` → `groq-llama-3.3-70b` (lateral equivalence)
- Maintains similar capability level across families

**Boundary Constraints**: User-defined limits on model selection
- "No higher than X" - don't use expensive models
- "No lower than Y" - maintain minimum quality
- Both - operate within a specific tier range

---

## Architecture

**Implementation Location**: All model selection logic lives in `botticelli_models` crate
- Already handles all LLM providers (anthropic, gemini, groq, huggingface, ollama, openai_compat)
- Coordinates provider selection and fallback
- Natural home for cross-provider model strategy

### 1. Model Family Priority

```rust
use derive_more::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, Display)]
pub enum ModelFamily {
    /// Priority 1: Gemini (Google, generous free tier)
    #[display("gemini")]
    Gemini,
    /// Priority 2: Groq (fast inference, good free tier)
    #[display("groq")]
    Groq,
    /// Priority 3: Anthropic (Claude, pay-per-use)
    #[display("anthropic")]
    Anthropic,
    /// Priority 4: OpenAI-compatible (various providers)
    #[display("openai")]
    OpenAI,
}
```

### 2. Per-Family Tier Enums

Each family has models ordered from most capable (restrictive) to least:

```rust
use derive_more::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, Display)]
pub enum GeminiModel {
    /// Most capable, most restrictive
    #[display("gemini-2.0-flash-exp")]
    Pro,
    #[display("gemini-2.0-flash-thinking-exp-01-21")]
    Flash,
    #[display("gemini-1.5-flash")]
    FlashLite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, Display)]
pub enum GroqModel {
    #[display("llama-3.3-70b-versatile")]
    Llama70B,
    #[display("llama-3.1-70b-versatile")]
    Llama70BClassic,
    #[display("llama-3.1-8b-instant")]
    Llama8B,
}
```

### 3. Lateral Equivalence ("Friends")

```rust
pub trait ModelTier {
    /// Returns laterally equivalent models in other families
    fn friends(&self) -> Vec<Model>;
    
    /// Returns next cheaper model in same family (loyal down)
    fn downgrade(&self) -> Option<Model>;
    
    /// Returns next more capable model in same family (loyal up)
    fn upgrade(&self) -> Option<Model>;
    
    /// Model family
    fn family(&self) -> ModelFamily;
    
    /// Tier index (0 = most capable)
    fn tier_index(&self) -> usize;
}
```

Example implementation:

```rust
impl GeminiModel {
    pub fn friends(&self) -> Vec<Model> {
        match self {
            Self::Pro => vec![
                Model::Anthropic(AnthropicModel::Sonnet),
            ],
            Self::Flash => vec![
                Model::Groq(GroqModel::Llama70B),
            ],
            Self::FlashLite => vec![
                Model::Groq(GroqModel::Llama8B),
            ],
        }
    }
}
```

### 4. Unified Model Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Gemini(GeminiModel),
    Groq(GroqModel),
    Anthropic(AnthropicModel),
    OpenAI(OpenAIModel),
}

impl Model {
    pub fn family(&self) -> ModelFamily {
        match self {
            Self::Gemini(_) => ModelFamily::Gemini,
            Self::Groq(_) => ModelFamily::Groq,
            Self::Anthropic(_) => ModelFamily::Anthropic,
            Self::OpenAI(_) => ModelFamily::OpenAI,
        }
    }
    
    pub fn tier_index(&self) -> usize {
        match self {
            Self::Gemini(m) => *m as usize,
            Self::Groq(m) => *m as usize,
            Self::Anthropic(m) => *m as usize,
            Self::OpenAI(m) => *m as usize,
        }
    }
    
    pub fn friends(&self) -> Vec<Model> {
        match self {
            Self::Gemini(m) => m.friends(),
            Self::Groq(m) => m.friends(),
            Self::Anthropic(m) => m.friends(),
            Self::OpenAI(m) => m.friends(),
        }
    }
    
    pub fn downgrade(&self) -> Option<Model> {
        // Get next variant in enum (higher index)
        // Use strum::EnumIter to get variants vector
    }
    
    pub fn upgrade(&self) -> Option<Model> {
        // Get previous variant in enum (lower index)
    }
}
```

### 5. Boundary Constraints

```rust
use derive_new::new;
use derive_getters::Getters;

#[derive(Debug, Clone, Getters)]
pub struct ModelBounds {
    /// Don't use models more capable/expensive than this
    ceiling: Option<Model>,
    /// Don't use models less capable/cheaper than this
    floor: Option<Model>,
}

impl ModelBounds {
    pub fn allows(&self, model: Model) -> bool {
        // Check if model is within bounds
        // Compare tier_index and family priority
    }
    
    pub fn no_higher_than(model: Model) -> Self {
        Self { ceiling: Some(model), floor: None }
    }
    
    pub fn no_lower_than(model: Model) -> Self {
        Self { ceiling: None, floor: Some(model) }
    }
    
    pub fn between(floor: Model, ceiling: Model) -> Self {
        Self { ceiling: Some(ceiling), floor: Some(floor) }
    }
}
```

### 6. Selection Algorithm

```rust
use derive_new::new;
use derive_getters::Getters;

#[derive(Debug, Clone, Getters, new)]
pub struct ModelSelector {
    bounds: ModelBounds,
    current: Model,
}

impl ModelSelector {
    /// Select next model after rate limit hit
    pub fn select_fallback(&mut self) -> Option<Model> {
        // Algorithm:
        // 1. Try loyal downgrade (cheaper in same family)
        // 2. If blocked by floor, try friends (lateral)
        // 3. If no friends available, try next family priority
        // 4. If all families exhausted, try loyal downgrade again
        // 5. Return None if no valid options
    }
    
    /// Attempt to use a better model if available
    pub fn try_upgrade(&mut self) -> Option<Model> {
        // Opposite of fallback - try to use better models
        // when rate limits have reset
    }
}
```

---

## Implementation Steps

### Step 1: Model Family Enum
**File**: `crates/botticelli_models/src/model_family.rs`

Create `ModelFamily` enum with strum derives for iteration and priority ordering.

### Step 2: Per-Family Model Enums
**Files**: 
- `crates/botticelli_models/src/models/gemini.rs`
- `crates/botticelli_models/src/models/groq.rs`
- `crates/botticelli_models/src/models/anthropic.rs`
- `crates/botticelli_models/src/models/openai.rs`

Create enums for each family's models, ordered by capability (restrictiveness).

### Step 3: Friends Mapping
**Implementation**: Add `friends()` methods to each model enum

Define lateral equivalence relationships based on:
- Capability (reasoning, speed, context window)
- Cost (tokens/dollar on paid tiers)
- Rate limits (requests/day on free tier)

### Step 4: Unified Model Type
**File**: `crates/botticelli_models/src/model.rs`

Create `Model` enum wrapping all family-specific enums, with methods for:
- Family extraction
- Tier indexing
- Friends delegation
- Upgrade/downgrade navigation

### Step 5: Boundary Constraints
**File**: `crates/botticelli_models/src/model_bounds.rs`

Implement `ModelBounds` with:
- Ceiling/floor constraints (private fields with derive_getters)
- Validation logic
- Constructor methods (`no_higher_than`, etc.) - use derive_new where applicable

### Step 6: Rate Limit Detection
**File**: `crates/botticelli_models/src/rate_limit.rs`

Create utilities to detect rate limit errors:
- Parse HTTP 429 responses
- Parse API-specific error codes
- Track rate limit state per model

### Step 7: Selection Algorithm
**File**: `crates/botticelli_models/src/model_selector.rs`

Implement `ModelSelector` with fallback logic:
1. Loyal downgrade (same family, cheaper tier)
2. Friendly lateral (equivalent in different family)
3. Family priority fallback (next priority family)
4. Return None if exhausted

Use derive_new for constructor, derive_getters for field access.

### Step 8: Chat Integration
**Updates**: `crates/botticelli_chat/src/session.rs`

Integrate `ModelSelector` into `ChatSession`:
- Track current model
- Detect rate limits
- Auto-select fallback
- Retry with new model

### Step 9: Configuration
**File**: `crates/botticelli_models/src/config.rs`

Add configuration for:
- Default starting model
- Boundary constraints
- Family priority order (allow user override)
- Rate limit retry behavior

### Step 10: Testing
**File**: `crates/botticelli_models/tests/model_selection_test.rs`

Test scenarios:
- Loyal downgrade within family
- Friendly lateral movement
- Boundary constraint enforcement
- Exhaustion handling (no valid models)
- Family priority ordering

---

## Usage Examples

### Basic Usage

```rust
use botticelli_models::{Model, ModelBounds, ModelSelector, GeminiModel};

// Start with Gemini Flash, no lower than Lite
let bounds = ModelBounds::no_lower_than(
    Model::Gemini(GeminiModel::FlashLite)
);

let mut selector = ModelSelector::new(
    bounds,
    Model::Gemini(GeminiModel::Flash),
);

// Rate limit hit on Gemini Flash
if let Some(fallback) = selector.select_fallback() {
    // Try Groq Llama 70B (friend of Flash)
    println!("Falling back to: {}", fallback);
}
```

### With Ceiling and Floor

```rust
// Only use mid-tier models
let bounds = ModelBounds::between(
    Model::Gemini(GeminiModel::FlashLite),  // floor
    Model::Gemini(GeminiModel::Flash),      // ceiling
);

let mut selector = ModelSelector::new(
    Model::Gemini(GeminiModel::Flash),
    bounds,
);

// Will only fall back to Flash → FlashLite → Groq equivalents
// Won't try Gemini Pro or Claude Sonnet (above ceiling)
```

### Upgrade After Reset

```rust
// Try to use better model after waiting
if let Some(better) = selector.try_upgrade() {
    println!("Upgrading to: {}", better);
}
```

---

## Free Tier Considerations

### Gemini (Google)
- **Pro**: 2 RPM, strict
- **Flash**: 15 RPM, generous for dev
- **Flash Lite**: 30 RPM, very permissive

### Groq
- **Llama 70B**: 30 RPM, 7K RPD
- **Llama 11B**: Higher limits
- **Llama 8B**: Highest limits

### Strategy
1. Start with Gemini Flash (good balance)
2. Fall back to Groq Llama 70B (lateral friend)
3. Further fallback: Groq smaller models
4. Ultimate fallback: Gemini Flash Lite

This gives maximum free tier capacity while maintaining quality.

---

## Future Enhancements

### Dynamic Friends
Learn equivalence from actual performance rather than hardcoded mappings.

### Cost Tracking
Track token usage and optimize for cost when user pays.

### Quality Feedback
Let users rate responses to refine tier equivalence.

### Time-Based Retry
Automatically retry higher-tier models after rate limit windows reset.

---

## Benefits

1. **Resilience**: Never blocked by single model's rate limits
2. **Cost Optimization**: Use cheapest adequate model
3. **Quality Control**: User-defined capability floors
4. **Budget Control**: User-defined cost ceilings
5. **Transparency**: Clear fallback strategy
6. **Flexibility**: Works with free and paid tiers

---

## References

- Model capabilities: Provider documentation
- Rate limits: API error responses
- Equivalence: Empirical testing during development
- Strum iteration: `EnumIter` trait for variant traversal
