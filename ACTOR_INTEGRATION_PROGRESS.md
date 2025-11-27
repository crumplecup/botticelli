# Actor Integration Progress

**Date**: 2025-11-27
**Status**: In Progress (Phase 1 Complete)

---

## Completed ✅

### Phase 1: Make channel_id Optional

**Files Modified**:
1. `crates/botticelli_actor/src/platforms/noop.rs` (NEW)
   - Created NoOpPlatform that implements Platform trait but does nothing
   - Allows actors to run without posting to any social media platform
   - Used for generation and curation actors

2. `crates/botticelli_actor/src/platforms/mod.rs`
   - Exported NoOpPlatform

3. `crates/botticelli_actor/src/lib.rs`
   - Added `pub use platforms::NoOpPlatform;`

4. `crates/botticelli_actor/src/bin/actor-server.rs`
   - Modified actor loading logic (lines 159-183)
   - Now creates NoOpPlatform when channel_id is None
   - All actors are registered regardless of channel_id
   - Removed orphaned else clause that prevented non-posting actors

**Result**: Actors can now run without a Discord channel_id. Perfect for narrative-only execution.

**Verified**: `cargo check --package botticelli_actor` passes

---

## In Progress 🚧

### Phase 2: Implement database.update_table Bot Command

**Goal**: Create a proper database bot command for updating table rows

**Required Implementation**:

1. Create `crates/botticelli_social/src/database/mod.rs`
   - Implement DatabaseCommandExecutor
   - Implement BotCommandExecutor trait
   - Add `update_table` command

2. Register in `crates/botticelli_social/src/lib.rs`

3. Add to BotCommandRegistry initialization

**Command Specification**:
```toml
[bots.mark_posted]
platform = "database"
command = "update_table"
table_name = "approved_discord_posts"
where_clause = "review_status = 'pending'"
order_by = "curation_score DESC, selected_at ASC"
limit = 1

[bots.mark_posted.updates]
review_status = "posted"
posted_at = "NOW()"
```

**Safety Requirements**:
- Use parameterized queries (diesel)
- Validate table names against whitelist
- Sanitize all inputs
- Return rows affected count

---

## Pending ⏳

### Phase 3: Create NarrativeExecutionSkill

**Files to Create**:
- `crates/botticelli_actor/src/skills/narrative_execution.rs`

**Requirements**:
- Implement Skill trait
- Execute narrative files using botticelli_narrative::Executor
- Pass database connection from SkillContext
- Handle narrative errors gracefully
- Return SkillOutput with execution metadata

**Configuration**:
```toml
[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/discord_poster.toml"
```

### Phase 4: Update discord_poster Narrative

**File to Modify**:
- `crates/botticelli_narrative/narratives/discord/discord_poster.toml`

**Changes Needed**:
1. Add fourth act: `mark_posted`
2. Use database.update_table bot command
3. Mark posted content as 'posted' to prevent duplicates

### Phase 5: Create Actor Configurations

**Files to Create**:
```
actors/
├── generation_actor.toml      # Runs every 12 hours, no channel_id
├── curation_actor.toml         # Runs every 6 hours, no channel_id
└── posting_actor.toml          # Runs every 2 hours, with channel_id
```

### Phase 6: Create Server Configuration

**File to Create**:
- `actor_server.toml`

**Contents**:
- Server settings (check_interval, circuit_breaker)
- Three actor instances with schedules

### Phase 7: Testing

**Test Plan**:
1. Dry-run validation
2. Single actor execution
3. Full server execution
4. End-to-end pipeline test

---

## Next Steps (Priority Order)

1. **Implement database.update_table command** (1-2 hours)
   - Create database command executor
   - Add update_table implementation
   - Test with SQL queries
   - Register in bot command registry

2. **Create NarrativeExecutionSkill** (1 hour)
   - Implement skill that executes narratives
   - Handle database connection passing
   - Test with discord_poster.toml

3. **Update discord_poster narrative** (15 min)
   - Add mark_posted act
   - Use database.update_table command

4. **Create actor configs** (30 min)
   - Three TOML files for actors
   - Configure skills and schedules

5. **Create server config** (15 min)
   - Single TOML file
   - Register all three actors

6. **Test everything** (1 hour)
   - Validation testing
   - Single execution testing
   - Full integration testing

**Total Estimated Time Remaining**: 4-5 hours

---

## Key Decisions Made

1. **NoOpPlatform over Optional Platform**: Cleaner than making platform optional in Actor struct, doesn't require changing entire actor system

2. **Database Commands as Bot Commands**: Follows existing pattern, allows using database operations in narratives just like Discord commands

3. **Narrative-Based over Pure Skills**: Leverages existing narrative system, better observability, easier for non-Rust developers to modify

4. **Hybrid Architecture**: Actors handle scheduling/reliability, narratives handle content logic

---

## Files Modified Summary

```
Modified:
  crates/botticelli_actor/src/bin/actor-server.rs
  crates/botticelli_actor/src/lib.rs
  crates/botticelli_actor/src/platforms/mod.rs

Created:
  crates/botticelli_actor/src/platforms/noop.rs
  crates/botticelli_narrative/narratives/discord/ACTOR_INTEGRATION_STRATEGY.md
  ACTOR_INTEGRATION_PROGRESS.md
```

---

## Questions Resolved

1. ✅ **Channel ID optional?** Yes - implemented via NoOpPlatform
2. ⏳ **Database update command?** In progress - proper implementation
3. ⏳ **Error handling?** TBD - retry entire narrative or skip

## Open Questions

1. Should narrative state (act outputs) be preserved in actor state?
2. How should we handle partial narrative failures?
3. Should we add rate limiting at the narrative level or skill level?
