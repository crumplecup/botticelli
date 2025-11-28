# JSON Compliance Workflow

**Created**: 2025-11-27
**Updated**: 2025-11-28
**Purpose**: Separate content improvement from JSON formatting to reduce generation failures
**Status**: ⚠️ PARTIALLY WORKING - Architecture issue identified (see JSON_EXTRACTION_STRATEGY.md)

## Problem

Previously, the `refine` act combined two distinct tasks:
1. Improving post content based on critique
2. Formatting the output as valid JSON

This caused failures when:
- LLM produced good content but invalid JSON
- JSON formatting errors obscured content quality
- Unicode characters (emojis) broke JSON parsing
- Critique focused on JSON format instead of content

## Solution

Split into **5-act pipeline** with clear separation of concerns:

```
Generate → Critique → Refine → Format JSON → Audit JSON
  ↓          ↓          ↓           ↓            ↓
Content    Content    Content     JSON         JSON
Creation   Analysis   Improve     Convert      Validate
```

### Act Responsibilities

**Acts 1-3: Content Quality** (temperature 0.3-0.8)
- `generate`: Create post text (plain text, no JSON)
- `critique`: Analyze content quality (engagement, clarity, accuracy)
- `refine`: Improve content based on critique (plain text output)

**Acts 4-5: JSON Compliance** (temperature 0.1)
- `format_json`: Convert refined text to JSON matching schema
- `audit_json`: Validate JSON compliance and fix issues

### Benefits

1. **Better Content** - Critique focuses on quality, not JSON syntax
2. **Fewer Failures** - JSON issues fixed in dedicated acts
3. **Clearer Debugging** - Know if problem is content or format
4. **Reusability** - JSON acts shared across all narratives
5. **Lower Temperature** - Strict JSON formatting uses temp 0.1 for consistency

## JSON Schema

All posts must conform to:

```json
{
  "text_content": "string (required, 10-2000 chars)",
  "content_type": "discord_post",
  "source": "generation_carousel",
  "tags": ["array", "of", "strings"]
}
```

## Implementation

### Shared Acts (defined once, used by all narratives)

```toml
[acts.critique]
model = "gemini-2.5-flash-lite"
temperature = 0.3  # Focused analysis
prompt = "Critique content quality: engagement, clarity, accuracy..."

[acts.refine]
model = "gemini-2.5-flash-lite"
temperature = 0.7  # Balanced improvement
prompt = "Improve based on critique. Output plain text (no JSON)..."

[acts.format_json]
model = "gemini-2.5-flash-lite"
temperature = 0.1  # Strict formatting
prompt = "Format as JSON matching schema. ONLY output JSON..."

[acts.audit_json]
model = "gemini-2.5-flash-lite"
temperature = 0.1  # Strict validation
prompt = "Validate JSON compliance. Fix if needed. ONLY output JSON..."
```

### Narrative TOC

Each narrative uses the same 5-act sequence:

```toml
[narratives.feature]
name = "feature_showcase"
target = "potential_discord_posts"

[narratives.feature.toc]
order = ["generate", "critique", "refine", "format_json", "audit_json"]
```

Storage triggers automatically after `audit_json` completes.

## Testing

Test the workflow:

```bash
# Generate single post
just narrate generation_carousel.feature

# Generate batch of 15 posts (5 types × 3 iterations)
just narrate generation_carousel.batch_generate
```

Expected behavior:
- Acts 1-3 produce plain text
- Act 4 converts to JSON
- Act 5 validates and fixes
- Final output stored in `potential_discord_posts` table

## Debugging

If posts fail to store:

1. **Check act outputs** - Look for which act failed in logs
2. **Acts 1-3 failures** - Content generation issue, not JSON
3. **Act 4 failures** - Schema mismatch or invalid JSON structure
4. **Act 5 failures** - Validation logic needs adjustment
5. **UTF-8 panics** - Check string truncation uses `.chars().take(N).collect()` not byte slicing

## Curation Phase Results (2025-11-28)

**Status**: ✅ JSON compliance workflow successfully applied to curation

The 4-act curation workflow now follows the same pattern as generation:
1. `analyze` - Evaluate potential posts (plain text)
2. `select` - Choose top 2-3 posts (plain text)
3. `format_json` - Convert to JSON (temp 0.1, 3000 tokens)
4. `audit_json` - Validate and fix JSON (temp 0.1, 3000 tokens)

**Success Rate**: JSON parsing successful after increasing `max_tokens` to 3000

**Database Integration Issue**: Existing `approved_discord_posts` table has NOT NULL `selected_at` column from previous schema. This causes insertion failure when JSON doesn't include that field.

**Solution**: Per JSON_SCHEMA_MISMATCH_STRATEGY Phase 1 - code now allows missing fields and inserts NULL, but PostgreSQL NOT NULL constraints must be handled via:
- Schema migrations (production)
- Manual table drops (development)
- ALTER TABLE to make columns nullable

## Current Issues (2025-11-28)

### Architecture Issue: Processor Applied to All Acts

**Problem**: `ContentGenerationProcessor` tries to parse JSON from ALL acts (generate, critique, refine, format_json, audit_json), but only acts 4-5 should produce JSON.

**Symptoms**:
- Acts 1-3 fail with "No JSON found in response"
- Error rate ~60% due to processor attempting JSON extraction on plain-text acts
- Misleading error messages suggesting LLM failures

**Root Cause**: Processor registered globally, not filtered by act name

**Fix**: See **JSON_EXTRACTION_STRATEGY.md** for detailed analysis and options

**Recommended**: Option A (Selective Processor) - add act name filtering to skip non-JSON acts

---

### Prompt Engineering: Occasional Non-JSON Responses

**Problem**: Even with "Output ONLY valid JSON" instructions, LLMs sometimes:
- Include explanatory text before/after JSON
- Truncate JSON mid-response (hit max_tokens limit)
- Produce malformed JSON with syntax errors

**Impact**: ~30% of format_json/audit_json failures

**Fix**: See **JSON_EXTRACTION_STRATEGY.md** Options B + C:
- Add explicit JSON examples to prompts
- Increase max_tokens from 700 to 1200

---

### PostgreSQL Array Formatting

**Problem**: JSON arrays `["a","b","c"]` fail when inserted into PostgreSQL array columns

**Fix**: Use JSONB columns instead of native arrays (simpler than format conversion)

---

## Future Work

### Priority 1: Fix Architecture Issue
Implement selective processor application (JSON_EXTRACTION_STRATEGY.md Option A)

### Priority 2: Improve JSON Compliance
- Add examples to format_json prompts (Option B)
- Increase max_tokens for JSON acts (Option C)
- Use JSONB for array fields (Option D2)

### Priority 3: Reusable JSON Narrative
Consider extracting JSON compliance acts into reusable narrative:

```toml
# Could be referenced by any content generation workflow
[narratives.ensure_json]
name = "json_compliance"
toc = ["format_json", "audit_json"]
input = "{previous_output}"  # Pipe from previous narrative
```

This would allow:
- Any narrative to ensure JSON output
- Centralized JSON schema definitions
- Easier updates to JSON handling logic
