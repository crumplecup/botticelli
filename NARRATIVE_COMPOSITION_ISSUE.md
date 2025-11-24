# Narrative Composition Issue

## Problem Statement

We attempted to implement a Discord post generation carousel using multi-narrative TOML files with a composition pattern where a top-level "batch_generate" narrative references multiple sub-narratives (feature, usecase, tutorial, community, tip). However, the current implementation has incomplete support for this pattern, leading to validation failures and architectural ambiguity.

## Current State

### What Works
- Multi-narrative TOML files load successfully
- Individual narratives can be selected and executed by name
- Basic act definitions with text/structured inputs work
- File content loading via recursive search works

### What Doesn't Work
- Narratives referencing other narratives as acts (composition pattern)
- Shared acts defined at top-level that multiple narratives can use
- DRY principle violated: critique/refine logic duplicated across 5 narratives
- Unclear execution model for nested narratives

## Root Causes

### 1. Ambiguous Shared Acts Pattern

Current TOML attempts to define shared acts at top-level:

```toml
[acts.critique]
model = "gemini-2.5-flash-lite"
# ... config ...

[narratives.feature.toc]
order = ["generate", "critique", "refine"]  # References shared act
```

Questions:
- Are top-level `[acts.*]` shared across all narratives?
- Do narratives inherit them automatically?
- Can narratives override shared acts?
- How do we distinguish shared vs narrative-specific acts?

### 2. Incomplete Narrative Reference Implementation

We added `narrative_ref` field to `ActConfig` but didn't implement:
- Recursive narrative execution in executor
- Context passing between parent/child narratives
- Output propagation from child to parent
- Variable substitution across narrative boundaries
- Cycle detection

### 3. Conflicting Design Goals

**Goal A**: DRY shared acts (critique/refine logic once)
**Goal B**: Narrative composition (batch_generate calls feature/usecase/etc)

These are different patterns that may require different solutions:
- Shared acts = code reuse within TOML
- Narrative composition = workflow orchestration

## Attempted Quick Fixes (Abandoned)

1. Added `narrative_ref` field to ActConfig - incomplete, no execution
2. Modified validation to allow empty acts - masks real problems
3. Debug logging for narrative references - doesn't solve execution

## Design Questions to Resolve

### 1. Shared Acts Pattern

**Option A: Explicit Inheritance**
```toml
[acts.critique]  # Shared definition
# ...

[narratives.feature.acts.critique]
inherits = "critique"  # Explicit reference
temperature = 0.5      # Optional override
```

**Option B: Automatic Inheritance**
```toml
[acts.critique]  # Available to all narratives

[narratives.feature.toc]
order = ["generate", "critique"]  # Auto-finds shared act
```

**Option C: No Shared Acts (Current)**
```toml
[narratives.feature.acts.critique]  # Duplicate for each narrative
# Full definition...

[narratives.usecase.acts.critique]  # Duplicate again
# Same definition...
```

### 2. Narrative Composition Pattern

**Option A: Act-Based Reference**
```toml
[narratives.batch.acts.run_feature]
narrative = "feature"  # Execute as act
```

**Option B: ToC-Based Reference**
```toml
[narratives.batch.toc]
order = ["narrative:feature", "narrative:usecase"]  # Special prefix
```

**Option C: Separate Orchestration**
```toml
[orchestration.batch_generate]
narratives = ["feature", "usecase", "tutorial"]
mode = "carousel"
iterations = 10
```

### 3. Context and Variable Propagation

When narrative B is called from narrative A:
- Does B see A's outputs?
- Can B reference A's acts via `{{parent.act_name}}`?
- Does B's output become an act in A for templating?
- How do we handle naming conflicts?

### 4. Carousel Integration

Current carousel implementation:
- Runs ToC order in a loop
- Each iteration executes all acts sequentially
- No awareness of narrative composition

Questions:
- Should carousel iterate over acts or narratives?
- Can carousel compose narratives (current goal)?
- Do we need separate carousel modes?

## Proposed Strategy

### Phase 1: Define Semantics (This Document)

1. Decide on shared acts pattern (A/B/C)
2. Decide on narrative composition pattern (A/B/C)
3. Document context/variable propagation rules
4. Define carousel behavior with composition

### Phase 2: Implement Chosen Pattern

1. Update TOML schema and parser
2. Implement executor logic for pattern
3. Add validation for pattern constraints
4. Update error messages to guide users

### Phase 3: Refactor generation_carousel.toml

1. Apply chosen pattern consistently
2. Remove attempted patterns that don't match choice
3. Ensure DRY where appropriate
4. Test full execution

## Recommendation

**Start Simple, Iterate:**

1. **No shared acts for now** (Option C) - Accept duplication initially
2. **No narrative composition yet** - Use single narratives or external orchestration
3. **Simplify generation_carousel.toml** to 5 separate narrative files
4. **Use external script/carousel** to run them in sequence
5. **Get working end-to-end** with current architecture
6. **Then** design shared acts/composition properly with real use cases

This avoids:
- Building unused features
- Architectural speculation
- Incomplete implementations
- Cascading changes

Once we have 5 working narratives generating posts to potential_posts table, we'll understand what we actually need from shared acts and composition.

## Current Action Items

1. Simplify generation_carousel.toml: Remove shared acts, remove batch_generate
2. Create 5 independent narrative files or 5 complete narratives in one file
3. Test each narrative independently
4. Use just commands or shell script for orchestration
5. Validate end-to-end: carousel → potential_posts table → real posts

## Files to Update

- `generation_carousel.toml` - Simplify to working pattern
- `DISCORD_POSTING_STRATEGY.md` - Update to match simpler implementation
- This document - Archive as reference for future composition feature

## Success Criteria

- [ ] 5 narratives (feature/usecase/tutorial/community/tip) defined
- [ ] Each narrative runs independently without errors
- [ ] Each narrative produces JSON for potential_posts
- [ ] Carousel mode iterates correctly
- [ ] Budget multipliers throttle API usage
- [ ] Posts appear in potential_posts table
- [ ] Zero architectural debt from incomplete features

## Future Work (Deferred)

- Shared acts pattern design and implementation
- Narrative composition pattern design and implementation
- Parent/child context propagation
- Cycle detection for recursive narratives
- Advanced carousel modes

---

**Status**: Problem defined, awaiting decision on strategy
**Created**: 2025-11-24
**Author**: Claude + Erik
