# Schema Inference from LLM Responses

## Overview

This feature extends the content generation system to automatically infer table schemas from JSON responses in LLM outputs, eliminating the need for explicit `template` fields in narratives when the schema can be derived from generated content.

## Motivation

**Current Limitation:** Content generation requires a `template` field in `[narration]`:

```toml
[narration]
name = "potential_posts"
template = "discord_messages"  # REQUIRED - references existing table
description = "Generate post ideas"
```

**Problem:** Users must pre-define table schemas even when:
- Generating novel data structures not in existing templates
- Prototyping new content types
- Working with dynamic/evolving schemas
- Creating one-off content experiments

**Proposed Solution:** Allow template-less narratives that infer schema from LLM output:

```toml
[narration]
name = "custom_content"
# NO template field - schema inferred from LLM JSON response
description = "Generate custom structured data"

[toc]
order = ["generate_structure", "populate_data"]

[acts]
generate_structure = """
Create a data structure for tracking user achievements.
Return a JSON object with: achievement_id, title, description, points, unlocked_date
"""

populate_data = """
Using the structure from the previous response, generate 10 sample achievements.
Return as JSON array with same structure.
"""
```

**Behavior:**
1. First act (`generate_structure`) generates JSON response
2. Processor extracts JSON from response
3. Schema is inferred from JSON structure (fields, types, nullability)
4. Table `custom_content` is created with inferred schema
5. Content is inserted
6. Subsequent acts use the same table structure

## Architecture

### High-Level Flow

```
Narrative without template
        ↓
Execute first act → LLM generates JSON
        ↓
Extract JSON block from response
        ↓
Infer schema from JSON structure
        ↓
Create table with inferred schema + metadata columns
        ↓
Insert content from JSON
        ↓
Execute subsequent acts → same table
```

### Detection Logic

The `ContentGenerationProcessor` already uses `should_process` to detect template-based narratives:

```rust
fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
    // Current: only process if template field exists
    context.narrative_metadata.template.is_some()
}
```

**Updated logic:**

```rust
fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
    // Process if:
    // 1. Template field exists (existing behavior), OR
    // 2. Narrative has acts and we're in inference mode (new behavior)
    context.narrative_metadata.template.is_some() ||
    context.narrative_metadata.enable_schema_inference.unwrap_or(false)
}
```

### Schema Inference Algorithm

#### 1. JSON Extraction

Reuse existing `extract_json()` from `extraction.rs`:

```rust
let json_str = extract_json(&context.execution.response)?;
let parsed = parse_json(&json_str)?;
```

**Handles:**
- JSON in markdown code blocks: ` ```json { ... } ``` `
- Raw JSON objects: `{ "field": "value" }`
- JSON arrays: `[{ ... }, { ... }]`

#### 2. Type Inference

Map JSON types to PostgreSQL types:

| JSON Type | PostgreSQL Type | Notes |
|-----------|----------------|-------|
| `string` | `TEXT` | Default for all strings |
| `number` (integer) | `BIGINT` | If no decimal point |
| `number` (float) | `DOUBLE PRECISION` | If decimal point present |
| `boolean` | `BOOLEAN` | True/false values |
| `null` | Column is `NULLABLE` | Mark field as optional |
| `array` | `TEXT[]` or `JSONB` | Arrays of primitives → `TYPE[]`, complex → `JSONB` |
| `object` | `JSONB` | Nested objects stored as JSON |

**Type Inference Function:**

```rust
/// Infer PostgreSQL column type from JSON value
fn infer_column_type(value: &JsonValue) -> (&'static str, bool) {
    match value {
        JsonValue::String(_) => ("TEXT", false),
        JsonValue::Number(n) => {
            if n.is_i64() || n.is_u64() {
                ("BIGINT", false)
            } else {
                ("DOUBLE PRECISION", false)
            }
        }
        JsonValue::Bool(_) => ("BOOLEAN", false),
        JsonValue::Null => ("TEXT", true),  // Nullable, type inferred from other rows
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                ("JSONB", true)  // Unknown array type
            } else {
                // Check first element to determine array type
                match &arr[0] {
                    JsonValue::String(_) => ("TEXT[]", false),
                    JsonValue::Number(_) => ("BIGINT[]", false),
                    JsonValue::Bool(_) => ("BOOLEAN[]", false),
                    _ => ("JSONB", false),  // Complex array
                }
            }
        }
        JsonValue::Object(_) => ("JSONB", false),
    }
}
```

#### 3. Schema Consolidation

For arrays of objects, merge schemas from all items:

```rust
/// Infer schema from JSON (single object or array)
pub fn infer_schema(json: &JsonValue) -> BoticelliResult<InferredSchema> {
    let items = match json {
        JsonValue::Object(_) => vec![json],
        JsonValue::Array(arr) => arr.iter().collect(),
        _ => return Err(BoticelliError::new(
            "Schema inference requires JSON object or array"
        )),
    };

    let mut schema = InferredSchema::new();

    for item in items {
        let obj = item.as_object()
            .ok_or_else(|| BoticelliError::new("Array must contain objects"))?;

        for (key, value) in obj {
            schema.add_field(key, value)?;
        }
    }

    Ok(schema)
}

struct InferredSchema {
    fields: HashMap<String, ColumnDefinition>,
}

struct ColumnDefinition {
    pg_type: String,
    nullable: bool,
    examples: Vec<JsonValue>,  // Track examples for type refinement
}

impl InferredSchema {
    fn add_field(&mut self, name: &str, value: &JsonValue) -> BoticelliResult<()> {
        let (pg_type, is_null) = infer_column_type(value);

        if let Some(existing) = self.fields.get_mut(name) {
            // Field seen before - refine type
            if is_null {
                existing.nullable = true;
            }
            existing.examples.push(value.clone());

            // Type conflict resolution (e.g., BIGINT vs DOUBLE PRECISION)
            if existing.pg_type != pg_type {
                existing.pg_type = resolve_type_conflict(&existing.pg_type, pg_type)?;
            }
        } else {
            // New field
            self.fields.insert(name.to_string(), ColumnDefinition {
                pg_type: pg_type.to_string(),
                nullable: is_null,
                examples: vec![value.clone()],
            });
        }

        Ok(())
    }
}

/// Resolve conflicts when same field has different types across rows
fn resolve_type_conflict(type1: &str, type2: &str) -> BoticelliResult<String> {
    match (type1, type2) {
        // BIGINT vs DOUBLE PRECISION → DOUBLE PRECISION (wider type)
        ("BIGINT", "DOUBLE PRECISION") | ("DOUBLE PRECISION", "BIGINT") => {
            Ok("DOUBLE PRECISION".to_string())
        }
        // TEXT vs anything → TEXT (universal fallback)
        ("TEXT", _) | (_, "TEXT") => Ok("TEXT".to_string()),
        // Array types must match
        (a, b) if a.ends_with("[]") && b.ends_with("[]") => {
            if a == b {
                Ok(a.to_string())
            } else {
                Ok("JSONB".to_string())  // Heterogeneous array → JSONB
            }
        }
        // Incompatible types → JSONB fallback
        _ => Ok("JSONB".to_string()),
    }
}
```

#### 4. Table Creation

Generate `CREATE TABLE` SQL with inferred schema:

```rust
pub fn create_inferred_table(
    conn: &mut PgConnection,
    table_name: &str,
    schema: &InferredSchema,
    narrative_name: Option<&str>,
    description: Option<&str>,
) -> DatabaseResult<()> {
    // Build column definitions
    let mut columns = Vec::new();

    for (name, def) in &schema.fields {
        let nullable = if def.nullable { "NULL" } else { "NOT NULL" };
        columns.push(format!("{} {} {}", name, def.pg_type, nullable));
    }

    // Add metadata columns (same as template-based tables)
    columns.push("generated_at TIMESTAMP NOT NULL DEFAULT NOW()".to_string());
    columns.push("source_narrative TEXT".to_string());
    columns.push("source_act TEXT".to_string());
    columns.push("generation_model TEXT".to_string());
    columns.push("review_status TEXT DEFAULT 'pending'".to_string());
    columns.push("tags TEXT[]".to_string());
    columns.push("rating INTEGER".to_string());

    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        table_name,
        columns.join(", ")
    );

    diesel::sql_query(&create_sql).execute(conn)?;

    // Track in metadata table
    let insert_metadata = format!(
        "INSERT INTO content_generation_tables (table_name, template_source, narrative_file, description)
         VALUES ('{}', 'inferred', {}, {})
         ON CONFLICT (table_name) DO NOTHING",
        table_name,
        narrative_name.map(|n| format!("'{}'", n)).unwrap_or("NULL".to_string()),
        description.map(|d| format!("'{}'", d)).unwrap_or("NULL".to_string()),
    );

    diesel::sql_query(&insert_metadata).execute(conn)?;

    Ok(())
}
```

### Integration with ContentGenerationProcessor

**Updated Processing Logic:**

```rust
async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
    let table_name = &context.narrative_metadata.name;
    let mut conn = self.connection.lock()
        .map_err(|e| BackendError::new(format!("Lock failed: {}", e)))?;

    // Determine if we're using template or inference
    let processing_mode = if let Some(template) = &context.narrative_metadata.template {
        ProcessingMode::Template(template.clone())
    } else {
        ProcessingMode::Inference
    };

    match processing_mode {
        ProcessingMode::Template(template) => {
            // Existing behavior: use template schema
            create_content_table(
                &mut conn,
                table_name,
                &template,
                Some(context.narrative_name),
                context.narrative_metadata.description.as_deref(),
            )?;
        }
        ProcessingMode::Inference => {
            // New behavior: infer schema from response
            let json_str = extract_json(&context.execution.response)?;
            let parsed = parse_json(&json_str)?;
            let schema = infer_schema(&parsed)?;

            create_inferred_table(
                &mut conn,
                table_name,
                &schema,
                Some(context.narrative_name),
                context.narrative_metadata.description.as_deref(),
            )?;
        }
    }

    // Insert content (same for both modes)
    self.insert_content(
        table_name,
        &parsed,
        context.narrative_name,
        &context.execution.act_name,
        context.execution.model.as_deref(),
    )?;

    Ok(())
}
```

## Implementation Phases

### Phase 1: Core Schema Inference ✅ **COMPLETE**

**Goals:** ✅ All Achieved
- ✅ Implement `infer_column_type()` for basic types (string, number, boolean, null)
- ✅ Implement `InferredSchema` data structure
- ✅ Implement `infer_schema()` for single objects
- ✅ Add type conflict resolution (`resolve_type_conflict`)
- ✅ Unit tests for type inference

**Deliverables:** ✅ All Delivered
- ✅ `src/database/schema_inference.rs` - New module (398 lines)
- ✅ Test suite with 29 test cases (exceeds 15+ requirement)
- ✅ Exported via `src/database/mod.rs` and `src/lib.rs`

**Implementation Summary:**

Created a complete schema inference system that analyzes JSON structures and maps them to PostgreSQL types:

**Core Functions:**
- `infer_column_type(value: &JsonValue) -> (&'static str, bool)` - Maps JSON types to PostgreSQL types
- `resolve_type_conflict(type1: &str, type2: &str) -> DatabaseResult<String>` - Handles type conflicts across rows
- `infer_schema(json: &JsonValue) -> DatabaseResult<InferredSchema>` - Infers complete schema from JSON

**Data Structures:**
- `ColumnDefinition` - Stores PostgreSQL type, nullability, and example values for each field
- `InferredSchema` - Manages field-to-column mapping with conflict resolution

**Type Mapping:**

| JSON Type | PostgreSQL Type | Nullable |
|-----------|----------------|----------|
| `string` | `TEXT` | No |
| `number` (int) | `BIGINT` | No |
| `number` (float) | `DOUBLE PRECISION` | No |
| `boolean` | `BOOLEAN` | No |
| `null` | `TEXT` | Yes (type inferred from other rows) |
| `array` (string) | `TEXT[]` | No |
| `array` (number) | `BIGINT[]` or `DOUBLE PRECISION[]` | No |
| `array` (boolean) | `BOOLEAN[]` | No |
| `array` (empty) | `JSONB` | Yes |
| `array` (complex) | `JSONB` | No |
| `object` | `JSONB` | No |

**Conflict Resolution Strategy:**
- BIGINT + DOUBLE PRECISION → DOUBLE PRECISION (wider numeric)
- Any type + TEXT → TEXT (universal fallback)
- Any type + JSONB → JSONB (structured fallback)
- Mismatched array types → JSONB
- BOOLEAN conflicts → TEXT

**Files Modified:**
- `src/database/schema_inference.rs` - New module (398 lines, 29 tests)
- `src/database/error.rs` - Added `SchemaInference` error variant
- `src/database/mod.rs` - Export schema_inference module
- `src/database/schema_docs.rs` - Fixed import patterns (CLAUDE.md compliance)
- `src/lib.rs` - Re-export at crate level

**Quality Metrics:**
- All 29 tests passing (100% coverage of type inference logic)
- Zero clippy warnings
- Zero compilation errors
- Follows CLAUDE.md import patterns (`use crate::{Type}` not `use crate::module::Type`)

## User Guide - Phase 1: Schema Inference API

### Using Schema Inference Functions

The schema inference API is now available for direct use:

```rust
use boticelli::{infer_schema, infer_column_type, resolve_type_conflict};
use serde_json::json;

// Infer schema from a JSON object
let data = json!({
    "user_id": 12345,
    "username": "alice",
    "active": true,
    "score": 98.5,
    "tags": ["rust", "postgresql"]
});

let schema = infer_schema(&data)?;

// Access inferred column types
assert_eq!(schema.fields["user_id"].pg_type, "BIGINT");
assert_eq!(schema.fields["username"].pg_type, "TEXT");
assert_eq!(schema.fields["active"].pg_type, "BOOLEAN");
assert_eq!(schema.fields["score"].pg_type, "DOUBLE PRECISION");
assert_eq!(schema.fields["tags"].pg_type, "TEXT[]");

// Check nullability
assert!(!schema.fields["username"].nullable);

// Infer from JSON array (consolidates schemas)
let array = json!([
    { "id": 1, "name": "Alice", "email": null },
    { "id": 2, "name": "Bob", "email": "bob@example.com" }
]);

let schema = infer_schema(&array)?;
assert!(schema.fields["email"].nullable); // Marked nullable due to null in first row
```

### Type Conflict Resolution

When the same field has different types across rows:

```rust
let data = json!([
    { "value": 42 },       // BIGINT
    { "value": 3.14 }      // DOUBLE PRECISION
]);

let schema = infer_schema(&data)?;
assert_eq!(schema.fields["value"].pg_type, "DOUBLE PRECISION"); // Widened to DOUBLE PRECISION
```

### Error Handling

Schema inference returns `DatabaseResult` with specific error types:

```rust
use boticelli::infer_schema;
use serde_json::json;

// Empty array error
let result = infer_schema(&json!([]));
assert!(result.is_err()); // "Cannot infer schema from empty JSON array"

// Non-object error
let result = infer_schema(&json!("not an object"));
assert!(result.is_err()); // "Schema inference requires JSON object or array"

// Array with non-objects
let result = infer_schema(&json!([1, 2, 3]));
assert!(result.is_err()); // "Array must contain objects for schema inference"
```

---

### Phase 2: Array and Complex Type Support ✅ **COMPLETE** (Implemented in Phase 1)

**Goals:** ✅ All Achieved (integrated into Phase 1)
- ✅ Support primitive arrays (`TEXT[]`, `BIGINT[]`, `BOOLEAN[]`)
- ✅ Support nested objects (fallback to `JSONB`)
- ✅ Support heterogeneous arrays (fallback to `JSONB`)
- ✅ Add array type inference tests

**Note:** Array and complex type support was implemented directly in Phase 1's `infer_column_type()` function, so Phase 2 is already complete.

**Implementation:**
- Primitive arrays detected by examining first element
- Empty arrays default to `JSONB` (nullable)
- Complex arrays (objects, mixed types) fall back to `JSONB`
- Nested objects always stored as `JSONB`

**Test Coverage:**
- 8 array-specific tests in schema_inference module
- Tests for string[], number[], boolean[] arrays
- Tests for empty and complex arrays
- JSONB fallback validation

**Examples:**
```json
// TEXT[] inference
{ "tags": ["rust", "database", "llm"] }

// BIGINT[] inference
{ "scores": [100, 95, 87, 92] }

// JSONB fallback (complex)
{ "metadata": { "created": "2025-01-01", "author": { "name": "Alice" } } }

// JSONB fallback (heterogeneous array)
{ "mixed": [1, "two", true, null] }
```

### Phase 3: Table Creation and Integration ✅ **COMPLETE**

**Goals:** ✅ All Achieved
- ✅ Implement `create_inferred_table()` function
- ✅ Update `ContentGenerationProcessor::process()` to handle both modes
- ✅ Add `ProcessingMode` enum (Template vs Inference)
- ✅ Integration tests with database

**Deliverables:** ✅ All Delivered
- ✅ Table creation logic in `schema_inference.rs`
- ✅ Updated `content_generation.rs` processor
- ✅ Updated tests for new behavior

**Implementation Summary:**

Integrated schema inference with the content generation processor, enabling automatic table creation from JSON responses.

**Core Changes:**

1. **`create_inferred_table()` function** (`schema_inference.rs`)
   - Creates PostgreSQL tables from `InferredSchema`
   - Adds standard metadata columns (same as template-based tables)
   - Tracks table creation in `content_generation_tables` with `template_source = 'inferred'`
   - Handles SQL escaping for table names and metadata values

2. **`ProcessingMode` enum** (`content_generation.rs`)
   - `Template(String)` - Use explicit template schema
   - `Inference` - Infer schema from JSON response
   - Enables dual-mode processing in single processor

3. **Updated `ContentGenerationProcessor`**
   - Detects mode: template exists → Template mode, otherwise → Inference mode
   - Parses JSON first (needed for inference)
   - Routes to appropriate table creation function
   - Logs mode and inferred field counts
   - Always processes (no longer requires template field)

**Processing Flow:**

```rust
// 1. Detect mode
let mode = if let Some(template) = &metadata.template {
    ProcessingMode::Template(template.clone())
} else {
    ProcessingMode::Inference
};

// 2. Parse JSON
let json = parse_json(&extract_json(&response)?)?;

// 3. Create table based on mode
match mode {
    ProcessingMode::Template(template) => {
        create_content_table(conn, table_name, &template, ...)?;
    }
    ProcessingMode::Inference => {
        let schema = infer_schema(&json)?;
        create_inferred_table(conn, table_name, &schema, ...)?;
    }
}

// 4. Insert content (same for both modes)
insert_content(table_name, &json, ...)?;
```

**Files Modified:**
- `src/database/schema_inference.rs` - Added `create_inferred_table()` (68 lines)
- `src/database/mod.rs` - Export `create_inferred_table`
- `src/lib.rs` - Re-export at crate level
- `src/narrative/content_generation.rs` - Added `ProcessingMode` enum and dual-mode logic
- `tests/narrative_content_generation_test.rs` - Updated test for new behavior

**Quality Metrics:**
- All 51 unit tests passing
- 3 processor tests updated and passing
- Zero clippy warnings
- Zero compilation errors

**Next Steps:** Phase 4 will add TOML configuration support for explicitly enabling/disabling schema inference.

### Phase 4: Narrative Configuration (Week 2-3)

**Goals:**
- [ ] Add optional `infer_schema` field to `[narration]` TOML
- [ ] Update narrative validation to allow template-less mode
- [ ] Update documentation and examples
- [ ] Create example narratives

**Deliverables:**
- Updated TOML parsing in `narrative/toml.rs`
- Example narratives in `narratives/inferred_*.toml`
- Updated `CONTENT_GENERATION.md` documentation

**TOML Schema:**

```toml
[narration]
name = "custom_data"
description = "Generate custom structured data"
# Option 1: Explicit inference flag
infer_schema = true

# Option 2: Inference automatic if template missing (simpler)
# (no template field → infer mode)
```

**Recommendation:** Option 2 (automatic) for better UX. If `template` is missing and content generation is enabled, automatically use inference mode.

### Phase 5: Error Handling and Edge Cases (Week 3)

**Goals:**
- [ ] Handle schema inference failures gracefully
- [ ] Detect incompatible type changes across acts
- [ ] Provide clear error messages for invalid JSON
- [ ] Add retry logic for schema conflicts

**Error Scenarios:**

1. **Invalid JSON in response:**
   ```
   Error: Failed to extract valid JSON from LLM response
   Hint: Ensure act prompt requests JSON output
   ```

2. **Schema conflict across acts:**
   ```
   Error: Schema conflict in table 'custom_data'
   Field 'user_id' changed from BIGINT to TEXT
   Hint: Ensure consistent types across all acts
   ```

3. **Empty or array-only response:**
   ```
   Error: Cannot infer schema from empty JSON array
   Hint: First act must return at least one object
   ```

4. **Nested complexity limit:**
   ```
   Warning: Complex nested object detected in field 'metadata'
   Action: Storing as JSONB instead of expanding columns
   ```

**Error Handling:**
```rust
pub enum SchemaInferenceErrorKind {
    InvalidJson(String),
    EmptyResponse,
    SchemaConflict { field: String, expected: String, actual: String },
    UnsupportedType(String),
}
```

### Phase 6: Testing and Documentation (Week 3-4)

**Goals:**
- [ ] Comprehensive test suite (30+ tests)
- [ ] Example narratives for common use cases
- [ ] Update `CONTENT_GENERATION.md` with inference guide
- [ ] Developer documentation
- [ ] Performance benchmarks

**Test Coverage:**
- Type inference (all JSON types)
- Schema consolidation (multiple objects)
- Type conflict resolution
- Table creation and metadata
- Error handling
- Edge cases (empty arrays, deeply nested objects)

**Example Narratives:**

1. `narratives/inferred_achievements.toml` - Gaming achievements tracking
2. `narratives/inferred_feedback.toml` - User feedback collection
3. `narratives/inferred_analytics.toml` - Custom analytics events

**Documentation Sections:**
- Overview of schema inference
- When to use template vs inference
- Type mapping reference
- Troubleshooting guide
- Best practices

## Design Decisions

### 1. Automatic vs Explicit Inference

**Options:**
- **A:** Require `infer_schema = true` flag in TOML
- **B:** Automatic when `template` field is missing

**Decision:** Option B (automatic)

**Rationale:**
- ✅ Simpler UX (less configuration)
- ✅ Follows principle of least surprise (no template → infer)
- ✅ Backward compatible (existing narratives have templates)
- ✅ Reduces boilerplate

### 2. Schema Persistence

**Question:** Should inferred schemas be stored for reuse?

**Options:**
- **A:** Ephemeral (infer every time)
- **B:** Store in `content_generation_tables` metadata
- **C:** Generate migration files for review

**Decision:** Option B (store in metadata table)

**Rationale:**
- ✅ Consistent with template-based approach
- ✅ Enables schema evolution tracking
- ✅ Allows manual schema review
- ❌ Option C too complex for automatic inference

**Metadata Storage:**
```sql
-- Existing table with new column
ALTER TABLE content_generation_tables
ADD COLUMN inferred_schema JSONB;

-- Example row
{
  "table_name": "custom_achievements",
  "template_source": "inferred",
  "inferred_schema": {
    "achievement_id": { "type": "BIGINT", "nullable": false },
    "title": { "type": "TEXT", "nullable": false },
    "points": { "type": "BIGINT", "nullable": true }
  },
  "created_at": "2025-11-16T10:30:00Z"
}
```

### 3. Type Widening Strategy

**Question:** How to handle type conflicts?

**Strategy:**
- BIGINT + DOUBLE PRECISION → DOUBLE PRECISION (wider numeric)
- Any type + TEXT → TEXT (universal fallback)
- Array types must match exactly, else → JSONB
- Complex types → JSONB

**Alternative Considered:** Strict mode (fail on type mismatch)

**Rejected Because:**
- ❌ Too fragile for LLM-generated content
- ❌ Requires perfect consistency across acts
- ❌ Poor user experience (cryptic failures)

**Type Widening Preference:**
```
SMALLINT → INTEGER → BIGINT → DOUBLE PRECISION → TEXT → JSONB
(narrow)                                                (wide)
```

### 4. Primary Key Handling

**Question:** Should inferred tables have primary keys?

**Options:**
- **A:** Auto-generate `id SERIAL PRIMARY KEY`
- **B:** Infer from JSON (if field named `id` exists)
- **C:** No primary key (simpler)

**Decision:** Option B with Option C fallback

**Logic:**
```rust
// If JSON contains "id" field → use as primary key
if schema.fields.contains_key("id") {
    create_table_with_pk("id", &schema);
} else {
    // No PK - just regular columns
    create_table_without_pk(&schema);
}
```

**Rationale:**
- ✅ Respects user intent (explicit `id` in JSON)
- ✅ No magic columns (avoids confusion)
- ✅ Flexible (content tables rarely need strong PK constraints)

## Use Cases

### Use Case 1: Prototyping Novel Schemas

**Scenario:** Developer wants to experiment with a new content structure.

**Narrative:**
```toml
[narration]
name = "experiment_achievements"
description = "Prototype achievement tracking system"

[toc]
order = ["design", "populate"]

[acts]
design = """
Design a JSON structure for tracking user achievements with:
- Unique identifier
- Achievement title and description
- Point value
- Unlock timestamp
- Rarity tier (common/rare/epic/legendary)

Return a sample JSON object.
"""

populate = """
Using the previous structure, generate 20 diverse achievements
spanning different rarity tiers. Return as JSON array.
"""
```

**Outcome:**
- First act infers schema from single object
- Table `experiment_achievements` created
- Second act inserts 20 rows
- Developer can query, review, iterate

### Use Case 2: Dynamic Event Logging

**Scenario:** Track custom analytics events with varying structures.

**Narrative:**
```toml
[narration]
name = "analytics_events"
description = "Custom analytics event schema"

[toc]
order = ["generate_events"]

[acts]
generate_events = """
Generate 50 analytics events for a mobile app with fields:
- event_id (unique)
- event_type (page_view, button_click, purchase, etc.)
- user_id
- timestamp
- properties (JSON object with event-specific data)

Return as JSON array.
"""
```

**Outcome:**
- Schema inferred from array of 50 events
- `properties` field stored as JSONB (flexible)
- Events can be queried via PostgreSQL JSON operators

### Use Case 3: User Feedback Collection

**Scenario:** Generate sample user feedback for testing review workflows.

**Narrative:**
```toml
[narration]
name = "user_feedback"
description = "Sample user feedback for testing"

[toc]
order = ["feedback_v1", "feedback_v2"]

[acts]
feedback_v1 = """
Generate 10 user feedback entries with:
- feedback_id, user_name, rating (1-5), comment, submitted_at

Return as JSON array.
"""

feedback_v2 = """
Generate 10 more feedback entries, but add a 'category' field
(bug_report, feature_request, general_feedback).
"""
```

**Outcome:**
- First act creates table with 5 columns
- Second act adds `category` column (detected as new field)
- Schema evolves naturally with content

**Note:** Schema evolution requires careful handling - may need ALTER TABLE logic.

## Risks and Mitigations

### Risk 1: Inconsistent LLM Output

**Risk:** LLM returns different types for same field across acts.

**Example:**
```json
// Act 1
{ "user_id": 12345 }

// Act 2
{ "user_id": "usr_67890" }
```

**Mitigation:**
- Type widening (BIGINT → TEXT)
- Log warnings for type conflicts
- Provide schema review command: `boticelli content schema --table user_feedback`

### Risk 2: Missing Fields in Later Acts

**Risk:** First act has field, second act omits it.

**Example:**
```json
// Act 1
{ "name": "Alice", "email": "alice@example.com" }

// Act 2
{ "name": "Bob" }  // Missing email
```

**Mitigation:**
- Mark all inferred fields as NULLABLE by default
- Consolidate schema across all acts before finalizing
- Warn if field disappears: "Field 'email' missing in act 2"

### Risk 3: Schema Bloat

**Risk:** LLM adds many fields, creating wide tables.

**Example:**
```json
{
  "field1": "...", "field2": "...", /* ... */, "field50": "..."
}
```

**Mitigation:**
- Set column limit (e.g., max 30 columns)
- Fallback to JSONB for objects with >30 fields
- Warn user about schema complexity

### Risk 4: Type Inference Ambiguity

**Risk:** Cannot distinguish between string numbers and actual numbers.

**Example:**
```json
{ "user_id": "12345" }  // String or number?
```

**Mitigation:**
- Always infer from JSON type, not string content
- `"12345"` → TEXT (as given by LLM)
- `12345` → BIGINT (actual number)
- Document in prompt requirements

## Alternatives Considered

### Alternative 1: Require Schema in First Act

**Approach:** Force first act to return schema definition.

```toml
[acts]
define_schema = """
Return a JSON schema definition:
{
  "fields": [
    {"name": "user_id", "type": "bigint"},
    {"name": "username", "type": "text"}
  ]
}
"""
```

**Rejected Because:**
- ❌ Adds complexity to user prompts
- ❌ Requires users to understand PostgreSQL types
- ❌ Not as flexible as automatic inference
- ✅ Could be added as optional override later

### Alternative 2: Use JSON Schema Standard

**Approach:** Parse JSON Schema (draft-07) for type definitions.

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "user_id": { "type": "integer" },
    "username": { "type": "string" }
  }
}
```

**Rejected Because:**
- ❌ Unlikely LLMs will output JSON Schema unprompted
- ❌ Requires explicit prompting for schema format
- ❌ More complex parsing logic
- ✅ Could be added as explicit template mode later

### Alternative 3: Defer to First INSERT

**Approach:** Don't create table upfront, dynamically add columns on first insert.

**Rejected Because:**
- ❌ Fragile (table structure unclear)
- ❌ Difficult to track schema evolution
- ❌ No validation or error checking
- ❌ Poor developer experience

## Success Metrics

### Phase 1 Success
- [ ] Type inference works for all basic JSON types
- [ ] 15+ unit tests passing
- [ ] Schema consolidation handles multiple objects

### Phase 2 Success
- [ ] Array types inferred correctly
- [ ] JSONB fallback for complex types
- [ ] 10+ array-specific tests passing

### Phase 3 Success
- [ ] Tables created with inferred schemas
- [ ] Integration test: narrative → table → data
- [ ] Metadata tracking works

### Phase 4 Success
- [ ] Template-less narratives load and execute
- [ ] Example narratives run end-to-end
- [ ] Documentation complete

### Final Success
- [ ] Zero clippy warnings, all tests passing
- [ ] 3+ real-world example narratives
- [ ] Schema inference works for 90%+ of LLM-generated JSON
- [ ] Performance: Infer schema from 100 objects in <100ms

## Future Enhancements

1. **Schema Evolution Tracking**
   - Detect when fields are added/removed across acts
   - Generate ALTER TABLE migrations automatically
   - Version schemas in metadata table

2. **Explicit Schema Override**
   - Allow users to provide schema hints in TOML
   - Example: `[schema] user_id = "bigint not null"`

3. **Smart Type Hints**
   - Detect common patterns (emails, URLs, UUIDs)
   - Infer constraints (e.g., email → VARCHAR with CHECK)
   - Use embeddings to match field names to types

4. **Schema Validation Mode**
   - Strict mode: Fail on type conflicts
   - Warn mode: Log warnings but continue (default)
   - Permissive mode: Always widen to TEXT

5. **JSON Schema Export**
   - Export inferred schemas as JSON Schema
   - Enable reuse and validation in other tools

## Open Questions

1. **Should schema inference work across narrative executions?**
   - Example: Run narrative once (infer schema), run again (reuse table)
   - Answer: Yes - check if table exists, reuse if compatible

2. **How to handle schema drift?**
   - If second execution infers different schema?
   - Options: Fail, warn, widen types, create versioned table
   - Recommendation: Warn + widen types

3. **Should users be able to lock schemas?**
   - Prevent schema changes after first inference
   - Use case: Production tables that shouldn't evolve
   - Recommendation: Add `lock_schema = true` flag in future

4. **How deep to infer nested objects?**
   - Flatten one level? Two levels? Always JSONB?
   - Recommendation: JSONB for all nested objects (simple, flexible)

5. **Should we support schema migrations between acts?**
   - Act 1: Create table with 3 columns
   - Act 2: Add 2 new columns (ALTER TABLE)
   - Recommendation: Phase 2 feature, requires careful design

## References

- [CONTENT_GENERATION.md](CONTENT_GENERATION.md) - Content generation architecture
- [JSON Type System](https://www.json.org/) - JSON specification
- [PostgreSQL Data Types](https://www.postgresql.org/docs/current/datatype.html)
- [Diesel Schema Reflection](https://docs.diesel.rs/)

## Appendix: Type Mapping Reference

### Complete JSON → PostgreSQL Mapping

| JSON Type | Example | PostgreSQL Type | Notes |
|-----------|---------|----------------|-------|
| String | `"hello"` | `TEXT` | Universal string storage |
| Number (int) | `42` | `BIGINT` | 64-bit integer (-9.2E18 to 9.2E18) |
| Number (float) | `3.14` | `DOUBLE PRECISION` | 64-bit floating point |
| Boolean | `true` | `BOOLEAN` | True/false |
| Null | `null` | Column marked `NULLABLE` | Type inferred from other rows |
| Array (string) | `["a", "b"]` | `TEXT[]` | Array of text |
| Array (number) | `[1, 2, 3]` | `BIGINT[]` | Array of integers |
| Array (bool) | `[true, false]` | `BOOLEAN[]` | Array of booleans |
| Array (mixed) | `[1, "a"]` | `JSONB` | Heterogeneous → JSON storage |
| Array (objects) | `[{}, {}]` | `JSONB` | Complex → JSON storage |
| Object | `{"a": 1}` | `JSONB` | Nested object storage |
| Empty array | `[]` | `JSONB` | Unknown type → JSON |

### Type Conflict Resolution Table

| Type 1 | Type 2 | Result | Rationale |
|--------|--------|--------|-----------|
| BIGINT | DOUBLE PRECISION | DOUBLE PRECISION | Wider numeric type |
| BIGINT | TEXT | TEXT | Universal fallback |
| TEXT[] | BIGINT[] | JSONB | Incompatible arrays |
| TEXT | JSONB | JSONB | Structured > unstructured |
| BOOLEAN | BIGINT | TEXT | No safe conversion |
| NULL | Any | Any (nullable) | Null widens to any type |

### Reserved Metadata Columns

These columns are automatically added and should not appear in inferred schemas:

- `generated_at` - Timestamp of content generation
- `source_narrative` - Narrative name
- `source_act` - Act name that generated row
- `generation_model` - LLM model identifier
- `review_status` - Review workflow state
- `tags` - User-defined tags (text array)
- `rating` - User rating (1-5 scale)

If LLM output contains any of these field names, they are renamed:
- `generated_at` → `content_generated_at`
- `source_narrative` → `content_source_narrative`
- etc.
