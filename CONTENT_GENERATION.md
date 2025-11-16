# Content Generation Strategy

## Overview

This document describes Boticelli's content generation strategy for creating Discord-ready content that can be reviewed and refined before publication. Instead of directly inserting into Discord tables, users generate content into local review tables, then selectively promote approved content to Discord.

## Motivation

**Problem:** Users want to generate multiple content variations, review them, and choose the best ones before posting to Discord.

**Current Limitation:** The existing `--process-discord` flag directly inserts into Discord tables, making it difficult to:

- Generate and compare multiple variations
- Review content before publication
- Iterate on prompts without cluttering the Discord database
- Maintain a content library separate from published posts

**Solution:** Template-based content generation tables that:

- Use Discord schema as templates
- Store generated content locally for review
- Allow selective promotion to Discord tables
- Enable content versioning and refinement workflows

## Narrative Schema Extension

### New [narration] Fields

```toml
[narration]
name = "potential_posts"        # Name of the target table (NEW: becomes table name)
template = "discord_messages"   # Source schema to copy (NEW)
description = "Generate post ideas for review"
```

### Semantic Changes

| Field | Old Behavior | New Behavior |
|-------|-------------|---------------|
| `name` | Descriptive label only | **Becomes the Diesel table name** for storage |
| `description` | Optional metadata | Remains optional metadata |
| `template` | (doesn't exist) | **NEW**: References existing Discord table schema |

### Example Narratives

#### Example 1: Content Ideation

```toml
[narration]
name = "potential_posts"
template = "discord_messages"
description = "Generate post ideas for the community channel"

[toc]
order = ["motivational", "tech_tip", "poll_question"]

[acts]
motivational = "Generate an uplifting message for Monday morning..."
tech_tip = "Create a helpful coding tip about error handling..."
poll_question = "Design an engaging poll about preferred frameworks..."
```

**Result:** Creates/uses `potential_posts` table with schema copied from `discord_messages`.

#### Example 2: Channel Concepts

```toml
[narration]
name = "channel_ideas"
template = "discord_channels"
description = "Brainstorm new channel concepts for the server"

[toc]
order = ["gaming", "creative", "learning"]

[acts]
gaming = "Design a gaming channel with focus on..."
creative = "Create a creative showcase channel for..."
learning = "Develop a learning resources channel about..."
```

**Result:** Creates/uses `channel_ideas` table with schema from `discord_channels`.

## Implementation Architecture

### Component Overview

```
User Narrative (TOML)
        ↓
Narrative Parser (validates template field)
        ↓
Schema Reflection (reads Discord table schema)
        ↓
Dynamic Table Creation (creates template-based table)
        ↓
Content Generation (AI generates JSON)
        ↓
Processor Pipeline (stores in custom table)
        ↓
Review/Approval UI (user reviews content)
        ↓
Promotion (copies approved content to Discord tables)
```

### Database Layer

#### 1. Schema Templates

Every Discord table can serve as a template:

- `discord_guilds` → templates for server configurations
- `discord_channels` → templates for channel designs
- `discord_messages` → templates for message content
- `discord_users` → templates for bot personas
- `discord_roles` → templates for role hierarchies
- `discord_emojis` → templates for custom emoji sets

#### 2. Dynamic Table Creation

**Migration Strategy:**

- Use Diesel's `table!` macro generation at runtime
- Store custom table schemas in `content_generation_tables` metadata table
- Generate migrations dynamically for new template tables

**Metadata Table:**

```sql
CREATE TABLE content_generation_tables (
    table_name TEXT PRIMARY KEY,
    template_source TEXT NOT NULL,  -- e.g., "discord_messages"
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    narrative_file TEXT,             -- Source TOML file
    description TEXT
);
```

#### 3. Table Naming Convention

Generated tables use the `name` field directly:

- `potential_posts` (from `discord_messages` template)
- `channel_ideas` (from `discord_channels` template)
- `role_drafts` (from `discord_roles` template)
- `emoji_proposals` (from `discord_emojis` template)

**Constraints:**

- Table names must be valid SQL identifiers
- Must not conflict with existing Discord tables
- Recommended prefix: None required (clarity from context)

### Processor Pipeline Extension

#### New Processor: `ContentGenerationProcessor`

**Responsibilities:**

1. Detect narratives with `template` field
2. Ensure target table exists (create if needed)
3. Insert generated content into custom table
4. Add metadata (generation timestamp, narrative source, act name)

**Detection Logic:**

```rust
fn should_process(&self, context: &ProcessorContext) -> bool {
    // Process if narration has a template field
    context.narrative.template.is_some()
}
```

#### Processor Registration

```rust
// In main.rs or processor registration
if cli.process_content_generation {
    registry.register(Box::new(ContentGenerationProcessor::new()));
}
```

**CLI Flag:**

```bash
boticelli run --narrative potential_posts.toml --process-content-generation
```

### Content Review Workflow

#### Phase 1: Generation

```bash
# Generate content into review table
boticelli run \
    --narrative narratives/potential_posts.toml \
    --process-content-generation
```

**Output:** 10+ message variants stored in `potential_posts` table.

#### Phase 2: Review

```bash
# List generated content
boticelli content list --table potential_posts

# Show specific content
boticelli content show --table potential_posts --id 5

# Rate or tag content
boticelli content tag --table potential_posts --id 5 --tags "approved,funny"
```

#### Phase 3: Promotion

```bash
# Promote approved content to Discord
boticelli content promote \
    --from potential_posts \
    --to discord_messages \
    --filter "approved" \
    --channel-id 1234567890
```

**Validation:**

- Ensures foreign key constraints are satisfied
- Fills in required fields (timestamps, IDs)
- Optionally publishes to Discord API

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2) ✅ COMPLETE

**Goals:**

- [x] Parse `template` field from TOML
- [x] Implement schema reflection for Discord tables
- [x] Create `content_generation_tables` metadata table
- [x] Generate dynamic table creation SQL
- [x] Basic Diesel model generation for custom tables

**Deliverables:**

- Schema reflection module (`src/database/schema_reflection.rs`)
- Dynamic table creation functions
- Metadata tracking table and migration
- Template field added to narrative TOML parsing

**Implementation Notes:**

The schema reflection module uses Diesel's `QueryableByName` to safely query PostgreSQL's `information_schema`. Key design decisions:

1. **QueryableByName Pattern**: Required by Diesel's `sql_query` for type-safe deserialization
2. **Foreign Key Handling**: FKs automatically made nullable in generated tables (e.g., `guild_id BIGINT NULL`)
3. **Metadata Columns**: All generated tables include `generated_at`, `source_narrative`, `source_act`, `generation_model`, `review_status`, `tags`, `rating`
4. **Error Handling**: New `TableNotFound` variant added to `DatabaseErrorKind`

**Files Modified:**
- `src/narrative/toml.rs` - Added `template` field to `TomlNarration`
- `src/narrative/core.rs` - Added `template` field to `NarrativeMetadata`
- `src/database/schema_reflection.rs` - New module (310 lines)
- `src/database/error.rs` - Added `TableNotFound` error variant
- `src/database/mod.rs` - Export schema reflection functions
- `migrations/2025-11-16-193150-0000_create_content_generation_tables/` - New migration

**Testing:**
- All existing tests passing (66 tests total)
- Schema reflection unit tests included
- Zero clippy warnings

**Next Steps:** Proceed to Phase 2 to implement the content generation processor.

### Phase 2: Processor Pipeline (Week 2-3)

**Goals:**

- [ ] Implement `ContentGenerationProcessor`
- [ ] Register processor with `--process-content-generation` flag
- [ ] Handle JSON parsing and insertion
- [ ] Add generation metadata (timestamp, source narrative, act)

**Deliverables:**

- Working content generation processor
- End-to-end generation from narrative to custom table
- Test suite for processor

### Phase 3: Content Management CLI (Week 3-4)

**Goals:**

- [ ] `boticelli content list` - List generated content
- [ ] `boticelli content show` - Display specific content
- [ ] `boticelli content tag` - Tag/rate content
- [ ] `boticelli content delete` - Remove content

**Deliverables:**

- Content management subcommands
- Query interface for custom tables
- Tagging/rating system

### Phase 4: Content Promotion (Week 4-5)

**Goals:**

- [ ] `boticelli content promote` - Copy to Discord tables
- [ ] Foreign key validation and auto-filling
- [ ] Optional Discord API publishing
- [ ] Bulk promotion with filters

**Deliverables:**

- Promotion command
- Validation logic
- Integration tests

### Phase 5: UI and Polish (Week 5-6)

**Goals:**

- [ ] Terminal UI for content review (using `ratatui` or similar)
- [ ] Side-by-side comparison of variants
- [ ] Inline editing before promotion
- [ ] Export to JSON/CSV

**Deliverables:**

- Interactive TUI for content review
- Documentation and examples
- Tutorial narratives

## Technical Considerations

### Schema Reflection

**Approach:** Use Diesel's schema introspection to read table structure.

```rust
// Pseudo-code
fn reflect_schema(table_name: &str) -> Result<TableSchema, Error> {
    // Query information_schema or use Diesel's schema module
    let columns = get_columns_for_table(table_name)?;
    let constraints = get_constraints_for_table(table_name)?;
    Ok(TableSchema { columns, constraints })
}
```

**Challenges:**

- Diesel doesn't natively support runtime schema reflection
- May need to parse `schema.rs` or query PostgreSQL directly
- Foreign key handling for custom tables

### Dynamic Table Creation

**Options:**

1. **Runtime Migrations:** Generate and run migrations on-the-fly
2. **Pre-generated Macros:** Code generation at build time
3. **Hybrid:** Common templates pre-generated, custom ones at runtime

**Recommendation:** Runtime migrations for flexibility.

### Foreign Key Handling

**Problem:** Custom tables reference Discord tables (e.g., `channel_id` in messages).

**Solutions:**

1. **Nullable FKs:** Make foreign keys optional in custom tables
2. **Synthetic IDs:** Use placeholder IDs during generation
3. **Late Binding:** Resolve FKs during promotion

**Recommendation:** Nullable FKs + late binding during promotion.

### Content Metadata

Every generated content row should include:

```sql
-- Added to all template-based tables
generated_at TIMESTAMP NOT NULL DEFAULT NOW(),
source_narrative TEXT,           -- TOML file path
source_act TEXT,                 -- Act name that generated this
generation_model TEXT,           -- AI model used
review_status TEXT DEFAULT 'pending',  -- pending, approved, rejected
tags TEXT[],                     -- User-defined tags
rating INTEGER,                  -- Optional 1-5 rating
```

**Implementation:** Use PostgreSQL `ALTER TABLE` to add metadata columns.

## Example Use Cases

### Use Case 1: Weekly Content Pipeline

**Scenario:** Community manager generates 20 post ideas every Monday, reviews during the week, publishes top 5.

**Workflow:**

```bash
# Monday: Generate content
boticelli run --narrative weekly_posts.toml --process-content-generation

# Tuesday-Friday: Review via TUI
boticelli content review --table weekly_posts

# Friday: Promote approved content
boticelli content promote \
    --from weekly_posts \
    --to discord_messages \
    --filter "approved" \
    --channel-id 1234567890 \
    --publish
```

### Use Case 2: A/B Testing Message Variants

**Scenario:** Generate 5 variations of an announcement, test with focus group, promote winner.

**Workflow:**

```bash
# Generate variants
boticelli run --narrative announcement_variants.toml --process-content-generation

# Export for focus group
boticelli content export --table announcement_variants --format pdf

# Promote winner
boticelli content promote \
    --from announcement_variants \
    --to discord_messages \
    --id 3
```

### Use Case 3: Channel Design Workshop

**Scenario:** Brainstorm 10 new channel concepts, discuss with team, create top 3.

**Workflow:**

```bash
# Generate channel ideas
boticelli run --narrative channel_brainstorm.toml --process-content-generation

# Review and tag
boticelli content list --table channel_brainstorm
boticelli content tag --table channel_brainstorm --id 7 --tags "approved,priority"

# Promote to Discord
boticelli content promote \
    --from channel_brainstorm \
    --to discord_channels \
    --filter "approved"
```

## Security and Privacy

### Considerations

1. **Local Storage Only:** Custom tables are local PostgreSQL, not pushed to Discord
2. **No API Keys in Content:** Generated content should not contain sensitive data
3. **Rate Limiting:** Content generation respects AI API rate limits
4. **Audit Trail:** Metadata tracks what was generated, reviewed, and promoted

### Best Practices

- Review all generated content before promotion
- Use `--dry-run` flag for promotion to preview changes
- Implement role-based access if multiple users share database
- Regular cleanup of old/rejected content

## Testing Strategy

### Unit Tests

- Schema reflection correctness
- Dynamic table SQL generation
- Processor detection logic
- Metadata insertion

### Integration Tests

- End-to-end narrative → custom table → promotion
- Foreign key constraint handling
- Multiple templates in one session
- Error handling for invalid templates

### Example Narratives

Create test narratives for each template:

- `tests/narratives/test_messages.toml`
- `tests/narratives/test_channels.toml`
- `tests/narratives/test_roles.toml`

## Future Enhancements

### Advanced Features

1. **Content Versioning:** Track edits to generated content
2. **Scheduled Promotion:** Auto-publish at specific times
3. **Template Mixins:** Combine multiple templates
4. **AI-Assisted Review:** Auto-score content quality
5. **Collaborative Review:** Multi-user approval workflows
6. **Content Analytics:** Track performance of promoted content

### Tool Integration

- Export to Notion, Google Docs, Slack
- Import existing content as seed data
- Integration with Discord analytics
- Webhook notifications for new content

## Open Questions

1. **Naming Conflicts:** How to handle if user names table same as existing Discord table?
   - **Answer:** Validation error, require different name

2. **Schema Evolution:** What if Discord schema changes?
   - **Answer:** Re-create template tables, migrate existing content

3. **Cross-Template References:** Can one custom table reference another?
   - **Answer:** Phase 2+ feature, allow custom FK relationships

4. **Template Inheritance:** Can templates extend other templates?
   - **Answer:** Future enhancement, not in initial implementation

5. **Batch Generation:** Generate 100+ variations in one run?
   - **Answer:** Support via `count` parameter in narrative

## Success Metrics

### Phase 1 Success Criteria

- [ ] Can create custom table from Discord template
- [ ] Can generate content into custom table
- [ ] Can query custom table via SQL

### Phase 2 Success Criteria

- [ ] `--process-content-generation` flag works end-to-end
- [ ] Metadata correctly added to all generated content
- [ ] Error handling for invalid templates

### Phase 3 Success Criteria

- [ ] Content management CLI works for basic CRUD
- [ ] Can tag and filter generated content
- [ ] Can view content in terminal

### Phase 4 Success Criteria

- [ ] Can promote content to Discord tables
- [ ] Foreign key validation prevents invalid promotions
- [ ] Can bulk promote with filters

### Final Success Criteria

- [ ] Complete tutorial narrative with examples
- [ ] Documentation covers all use cases
- [ ] Zero clippy warnings, all tests passing
- [ ] Performance: Generate and promote 1000 items in < 1 minute

## References

- [NARRATIVE_TOML_SPEC.md](NARRATIVE_TOML_SPEC.md) - Narrative format specification
- [DISCORD_SCHEMA.md](DISCORD_SCHEMA.md) - Discord database schema
- [NARRATIVE_PROCESSORS.md](NARRATIVE_PROCESSORS.md) - Processor pipeline architecture
- [POSTGRES.md](POSTGRES.md) - Database configuration

## Appendix: Schema Examples

### Example: potential_posts (from discord_messages template)

```sql
CREATE TABLE potential_posts (
    -- Fields from discord_messages template
    id BIGINT PRIMARY KEY,
    channel_id BIGINT,  -- Nullable in custom table
    author_id BIGINT,   -- Nullable in custom table
    content TEXT,
    message_type TEXT,
    
    -- Content generation metadata
    generated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    source_narrative TEXT,
    source_act TEXT,
    generation_model TEXT,
    review_status TEXT DEFAULT 'pending',
    tags TEXT[],
    rating INTEGER
);
```

### Example: channel_ideas (from discord_channels template)

```sql
CREATE TABLE channel_ideas (
    -- Fields from discord_channels template
    id BIGINT PRIMARY KEY,
    guild_id BIGINT,  -- Nullable in custom table
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL,
    topic TEXT,
    position INTEGER,
    
    -- Content generation metadata
    generated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    source_narrative TEXT,
    source_act TEXT,
    generation_model TEXT,
    review_status TEXT DEFAULT 'pending',
    tags TEXT[],
    rating INTEGER
);
```
