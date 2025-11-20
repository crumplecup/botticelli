# Database Trait Separation Analysis

## Current Problem

We're implementing Phase 3 (table references) and need to decide where database-related traits should live. Currently:

1. **`NarrativeRepository`** - Lives in `botticelli_interface`, implemented by `botticelli_database`  
   ✅ Used by facade and executor layers

2. **`TableQueryRegistry`** - Lives in `botticelli_interface`, implemented by `botticelli_database`  
   ✅ Used by narrative executor for table queries

3. **`ContentGenerationRepository`** - Lives in `botticelli_database`, only referenced within that crate  
   ❓ Should this move to `botticelli_interface`?

## The Question

Should `ContentGenerationRepository` move to `botticelli_interface` for consistency with other repository traits?

## Analysis

### Purpose of botticelli_interface

From the crate documentation:
> Defines core traits that different backend implementations must satisfy. Serves as a contract between the facade layer and provider implementations.

**Key insight**: The interface crate is for **pluggable backends** that the facade or executor depends on via trait objects.

### Content Generation Repository Usage

<function_calls>
<invoke name="bash">
<parameter name="command">cd /home/erik/repos/botticelli && rg "ContentGenerationRepository" --type rust -A 2
**Current usage patterns:**

1. **Database crate** (`botticelli_database/src/content_generation_repository.rs`):
   - Trait definition
   - PostgreSQL implementation

2. **Narrative crate** (`botticelli_narrative/src/content_generation.rs`):
   - Creates `PostgresContentGenerationRepository` instances
   - Tracks content generation lifecycle (start/complete)
   - Used during narrative execution

3. **CLI facade** (`botticelli/src/cli/content.rs`):
   - Shows generation history
   - Lists tracked generations

**Key findings:**
- ✅ Used by multiple crates (database, narrative, facade)
- ✅ Narrative executor depends on it
- ✅ Exposed in public API (facade commands)
- ❌ Currently lives in `botticelli_database` (implementation crate)

## Decision Criteria

A trait belongs in **`botticelli_interface`** if:
1. ✅ Multiple crates need the trait definition (not just implementation)
2. ✅ Facade or executor depends on it via trait object
3. ✅ It represents a pluggable "backend" component
4. ✅ We expect multiple implementations (Postgres, SQLite, in-memory, mock)

A trait stays in its **implementation crate** if:
1. ✅ Only that crate uses it internally
2. ✅ It's an implementation detail, not a public contract
3. ✅ No trait objects needed elsewhere
4. ✅ Tightly coupled to implementation-specific types

## Applying Criteria to ContentGenerationRepository

| Criterion | Assessment |
|-----------|------------|
| Multiple crates need trait | ✅ YES - narrative executor, facade CLI |
| Facade/executor depends on it | ✅ YES - narrative execution tracks generations |
| Pluggable backend component | ✅ YES - could have SQLite, in-memory, mock |
| Multiple implementations expected | ✅ YES - testing needs mocks, future DBs |
| Only used internally | ❌ NO - used by narrative and facade |
| Implementation detail | ❌ NO - part of public contract |
| No trait objects elsewhere | ❌ NO - narrative executor uses it |
| Tightly coupled to impl types | ⚠️ PARTIAL - uses DB-specific row types |

**Score: 4/4 for interface, 0/4 for implementation**

## Recommendation

**❌ KEEP `ContentGenerationRepository` in `botticelli_database`**

### Revised Rationale

Upon closer inspection of the actual trait definition, the trait is **tightly coupled to Diesel-specific types**:

```rust
fn start_generation(&mut self, new_gen: NewContentGenerationRow) -> DatabaseResult<ContentGenerationRow>;
fn complete_generation(&mut self, table_name: &str, update: UpdateContentGenerationRow) -> DatabaseResult<ContentGenerationRow>;
```

The trait uses:
- `NewContentGenerationRow` - Diesel insertable struct
- `UpdateContentGenerationRow` - Diesel changeset struct  
- `ContentGenerationRow` - Diesel queryable struct

**Key insight**: A trait in `botticelli_interface` should be **database-agnostic** and use domain types, not implementation-specific row structs.

### Why This is Different from NarrativeRepository

**NarrativeRepository** (correctly in interface):
- Uses domain types: `String`, `i32`, `Option<i32>`
- Returns domain types: `NarrativeExecution`, `ActExecution`
- No Diesel-specific types in signature
- Can be implemented by SQLite, in-memory, or any backend

**ContentGenerationRepository** (correctly in database):
- Uses Diesel row types: `NewContentGenerationRow`, `ContentGenerationRow`
- Tightly coupled to PostgreSQL implementation
- Would require significant refactoring to be backend-agnostic

### The Correct Solution

To make `ContentGenerationRepository` interface-appropriate, we would need to:

1. Create domain types (not Diesel row types)
2. Define the trait with domain types
3. Have the Postgres impl convert between domain and row types

**This is future work**, not a quick trait move.

## Pattern Documentation for CLAUDE.md

**Repository Trait Placement**:

- **Interface crate** (`botticelli_interface`):
  - Traits with database-agnostic signatures
  - Use domain types (String, i32, enums) not row structs
  - Example: `NarrativeRepository`, `TableQueryRegistry`

- **Database crate** (`botticelli_database`):
  - Traits tightly coupled to Diesel types
  - Use row structs (`NewXRow`, `XRow`, `UpdateXRow`)
  - Example: `ContentGenerationRepository` (current state)

**Rule**: Only move a repository trait to interface if it uses domain types, not implementation-specific row structs.

## Next Steps

1. ✅ Create analysis document (this file)
2. ✅ Correct conclusion: Keep trait in database crate (for now)
3. ✅ Update CLAUDE.md with repository trait placement guidelines
4. ✅ Implement ContentRepository trait for content management
5. ✅ Continue with Phase 3 table reference implementation
6. ⏸️ Future: Refactor `ContentGenerationRepository` to use domain types (separate ticket)

## Implementation Complete (2024-11-20)

**ContentRepository Trait** has been successfully implemented:

- **Location**: `botticelli_interface::ContentRepository`
- **Implementation**: `botticelli_database::DatabaseContentRepository`
- **Features**:
  - Domain types only (no Diesel row structs in trait)
  - Async operations via tokio::spawn_blocking
  - Connection pooling with diesel r2d2
  - Methods: list_content, update_review_status, delete_content

This demonstrates the pattern for moving database functionality to traits when using domain types rather than row structs.
