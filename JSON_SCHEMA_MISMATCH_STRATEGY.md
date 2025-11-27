# JSON Schema Mismatch Strategy

## Problem Statement

When inserting JSON content into database tables, we encounter mismatches between the JSON structure and the table schema:

1. **Extra JSON fields**: JSON contains fields like `created_at` that don't exist in target table
2. **Missing JSON fields**: JSON lacks fields that exist as columns in the table
3. **Type mismatches**: JSON field types don't align with column types

Current behavior (line 344-348 in `storage_actor.rs`):

- Attempts to insert ALL JSON fields
- Fails if JSON contains fields not in table
- Fails if required table columns are missing from JSON

## Immediate Fix (As Requested)

### 1. Extra JSON Fields → Ignore

**Implementation**: Filter JSON fields against table schema before building INSERT.

```rust
// Only include JSON fields that exist in the target table
for (key, value) in obj {
    if column_types.contains_key(key.as_str()) {
        columns.push(key.clone());
        let col_type = column_types.get(key.as_str()).copied().unwrap();
        values.push(json_value_to_sql(value, col_type));
    } else {
        tracing::debug!(field = %key, "Ignoring extra JSON field not in schema");
    }
}
```

**Impact**:

- ✅ Allows LLMs to generate extra metadata without breaking insertion
- ✅ Backward compatible (only removes failure case)
- ⚠️ Silent data loss if JSON field name typos occur

### 2. Missing JSON Fields → NULL

**Implementation**: Table columns missing from JSON get NULL values (default SQL behavior).

```rust
// Columns present in table but not in JSON will be NULL or DEFAULT
// No explicit action needed - SQL INSERT handles this automatically
```

**Requirements**:

- Table schema must allow NULL for optional columns OR have DEFAULT values
- Non-nullable columns without defaults must always be in JSON

**Impact**:

- ✅ Flexible JSON structure
- ✅ Supports partial updates
- ⚠️ Silent failures if required fields missing (NULL constraint violations)

## Remaining Issues Requiring Strategy

### Issue 3: Type Mismatches

**Problem**: JSON provides string where table expects integer, etc.

**Current Behavior**: `json_value_to_sql()` does basic conversion but may fail.

**Options**:

#### Option A: Strict Type Validation (Current)

- Reject mismatched types with clear error
- **Pros**: Data integrity, clear failures
- **Cons**: Brittle, requires perfect LLM output

#### Option B: Best-Effort Coercion

- Cast strings to numbers, booleans, etc.
- Log warnings on failures
- **Pros**: More forgiving
- **Cons**: Silent data corruption risk

#### Option C: Schema-Guided Prompting

- Include exact column types in JSON formatting prompt
- Rely on LLM to match types
- **Pros**: Clean separation of concerns
- **Cons**: LLMs still make mistakes

**Recommendation**: **Option A + Option C** - Keep strict validation but improve prompts to include type information.

I would prefer option B + option C. We need to make a best effort at coercion to avoid frequent failures, but also improve prompts to reduce errors.

---

### Issue 4: Required Field Validation

**Problem**: How to handle missing required (NOT NULL) fields?

**Current Behavior**: Database raises constraint violation, narrative fails.

**Options**:

#### Option A: Pre-INSERT Validation

- Check all NOT NULL columns present in JSON before INSERT
- Return clear error listing missing fields
- **Pros**: Clear feedback, prevents wasted DB calls
- **Cons**: Requires schema reflection, extra logic

#### Option B: Database Validation (Current)

- Let database enforce constraints
- Parse error messages
- **Pros**: Simple, leverages DB guarantees
- **Cons**: Cryptic errors, poor UX

#### Option C: Schema-Aware JSON Prompting

- Include required/optional distinction in prompt
- Trust LLM to provide required fields
- **Pros**: Minimal code changes
- **Cons**: LLMs still forget fields

**Recommendation**: **Option A** - Pre-validate required fields and provide clear error messages.

As mentioned above, we need to use the JSON formatting narrative to improve schema prompts (option c), but if we also do Option A, then we can optionally return the error messages to the LLM and ask for it to try again.

---

### Issue 5: Column Name Mismatches

**Problem**: LLM generates `textContent` but table has `text_content`.

**Current Behavior**: Field ignored (after fix #1), data lost.

**Options**:

#### Option A: Fuzzy Matching

- Try snake_case, camelCase, lowercase variations
- **Pros**: Forgiving
- **Cons**: Ambiguity, performance cost

#### Option B: Strict Matching + Schema in Prompt

- Require exact column names
- Include schema in JSON formatting prompt
- **Pros**: Explicit, predictable
- **Cons**: More prompt tokens

#### Option C: Mapping Configuration

- Allow TOML to define JSON→DB field mappings
- **Pros**: Explicit control, supports legacy schemas
- **Cons**: Extra configuration burden

**Recommendation**: **Option B** - Exact matching with schema-aware prompts. Add Option C later if needed.

Given the high failure rate from the LLMs, we need to employ fuzzy matching to meet it where it is.

---

### Issue 6: Array and Nested Object Handling

**Problem**: JSON contains arrays or nested objects, but table has flat structure.

**Current Behavior**: `json_value_to_sql()` likely serializes to JSONB or TEXT.

**Options**:

#### Option A: JSONB Column Type

- Store complex structures as JSONB
- **Pros**: Preserves structure, queryable
- **Cons**: Schema inference must detect and use JSONB type

#### Option B: Flatten on Insert

- Extract nested values to flat columns
- **Pros**: Normalized data
- **Cons**: Complex logic, field mapping required

#### Option C: Reject Complex Structures

- Only allow flat JSON objects
- **Pros**: Simple, predictable
- **Cons**: Limits use cases

**Recommendation**: **Option A** - Use JSONB for arrays/objects. Update schema inference to detect complex types.

Agreed.

---

## Implementation Plan

### Phase 1: Immediate Fixes (✅ COMPLETE)

1. ✅ Filter extra JSON fields (Issue #1)
2. ✅ Allow NULL for missing fields (Issue #2)
3. ✅ Add tracing for filtered/missing fields
4. ✅ Best-effort type coercion (Issue #3, Option B)
5. ✅ Fuzzy field name matching (Issue #5, Option A)
6. ✅ JSONB support for complex types (Issue #6, Option A)

### Phase 2: Validation Layer (Next Session)

1. Pre-INSERT validation for required fields (Issue #4)
2. Clear error messages listing problems
3. Schema-aware error reporting

### Phase 3: Prompt Enhancement (Future)

1. Include full schema in JSON formatting act
2. Specify column types and constraints
3. Mark required vs optional fields

### Phase 4: Advanced Features (Future)

1. Fuzzy field name matching (Issue #5, Option A)
2. JSONB support for complex types (Issue #6, Option A)
3. Field mapping configuration (Issue #5, Option C)

---

## Code Changes Required

### 1. `storage_actor.rs::handle_insert_content()` (Lines 344-348)

```rust
// BEFORE: Insert all JSON fields
for (key, value) in obj {
    columns.push(key.clone());
    let col_type = column_types.get(key.as_str()).copied().unwrap_or("text");
    values.push(json_value_to_sql(value, col_type));
}

// AFTER: Filter to only table columns
for (key, value) in obj {
    if let Some(&col_type) = column_types.get(key.as_str()) {
        columns.push(key.clone());
        values.push(json_value_to_sql(value, col_type));
    } else {
        tracing::debug!(
            field = %key,
            table = %table_name,
            "Ignoring extra JSON field not in table schema"
        );
    }
}
```

### 2. Add Pre-Validation (Phase 2)

```rust
fn validate_required_fields(
    json_obj: &serde_json::Map<String, JsonValue>,
    schema: &TableSchema,
) -> BotticelliResult<()> {
    let missing: Vec<_> = schema
        .columns
        .iter()
        .filter(|col| !col.nullable && col.default_value.is_none())
        .filter(|col| !json_obj.contains_key(&col.name))
        .map(|col| col.name.as_str())
        .collect();

    if !missing.is_empty() {
        return Err(botticelli_error::BackendError::new(format!(
            "Missing required fields: {}",
            missing.join(", ")
        ))
        .into());
    }

    Ok(())
}
```

### 3. Enhanced JSON Formatting Prompt (Phase 3)

```toml
[act.format_json]
prompt = """
Reformat the following content as JSON matching this exact schema:

{
  "text_content": "string (required, max 2000 chars)",
  "title": "string (optional, max 200 chars)",
  "tags": ["string"] (optional array),
  "priority": "integer" (optional, 1-5)
}

Content to format:
{content}

Return ONLY the JSON object, no markdown formatting.
"""
```

---

## Decision Log

| Issue               | Decision                                              | Status      | Rationale                      |
| ------------------- | ----------------------------------------------------- | ----------- | ------------------------------ |
| Extra JSON fields   | Ignore silently                                       | ✅ **DONE** | Flexibility, LLM metadata      |
| Missing JSON fields | Allow NULL                                            | ✅ **DONE** | Optional columns, partial data |
| Type mismatches     | Best-effort coercion + improved prompts (Option B+C)  | ✅ **DONE** | Balance flexibility & quality  |
| Required fields     | Pre-validation with retry option (Option A+C)         | ⏳ TODO     | Clear errors, better UX        |
| Name mismatches     | Fuzzy matching (Option A)                             | ✅ **DONE** | Handle case/underscore diffs   |
| Complex types       | JSONB support (Option A)                              | ✅ **DONE** | Preserve structure             |

## Implementation Status

### Phase 1: Lenient Insertion (✅ DONE)

**Location:** `crates/botticelli_narrative/src/storage_actor.rs`

- `handle_insert_content()` - Filters extra fields (line 345-356)
- Missing fields automatically get NULL (standard SQL behavior)
- Fuzzy matching via `find_column_match()` (line 346)

### Phase 2: Type Coercion (✅ DONE)

**Locations:**
- `crates/botticelli_narrative/src/storage_actor.rs::json_value_to_sql()` (line 504+)
  - String → integer/float parsing
  - Boolean → integer coercion
  - Number type widening
  - Fallback to NULL on failure

- `crates/botticelli_database/src/schema_inference.rs::coerce_value()` (new)
  - Comprehensive type coercion for schema inference
  - Handles arrays, JSONB, timestamps
  - Structured logging for diagnostics

### Phase 3: Enhanced Prompts (✅ DONE)

**Location:** `crates/botticelli_narrative/narratives/discord/generation_carousel.toml`

- `ensure_json_schema` act - Schema conformance check
- `audit_json` act - Self-audit for compliance
- Clear field requirements and constraints in prompts

### Phase 4: Required Field Validation (⏳ TODO)

**Needed:**
1. Query table constraints before insertion
2. Pre-validate required fields exist and are non-NULL
3. Return clear error messages on failure
4. Optional: LLM retry with field-specific prompts

---

## Testing Strategy

### Unit Tests

- Extra field filtering
- Missing field NULL insertion
- Required field validation
- Type mismatch detection

### Integration Tests

- Full carousel with schema mismatches
- Partial JSON objects
- Complex nested structures

### Observability

- Log all filtered fields (debug)
- Log all NULL fills (debug)
- Log type coercions (warn)
- Error messages include schema diff

---

## Metrics

Track in content_generation table:

- `fields_filtered`: Count of extra JSON fields ignored
- `fields_missing`: Count of table columns left NULL
- `type_coercions`: Count of successful type conversions
- `validation_errors`: Count of pre-INSERT failures

Add to `CompleteGeneration` message.
