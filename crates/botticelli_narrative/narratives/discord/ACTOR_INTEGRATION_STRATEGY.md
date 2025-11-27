# Actor Integration Strategy for Discord Posting

**Status**: Planning
**Created**: 2025-11-27
**Goal**: Integrate narrative-based Discord posting with the botticelli_actor scheduled execution system

---

## Current Situation

### What We Have

**Narratives (Completed)**:
- `generation_carousel.toml` - Generates potential posts, stores to `potential_discord_posts`
- `curate_and_approve.toml` - Curates and approves posts, stores to `approved_discord_posts`
- `discord_poster.toml` - Posts approved content to Discord

**Actor Infrastructure (Exists)**:
- `actor-server` binary - Scheduled task execution with circuit breakers
- Skills framework - Modular capabilities (ContentSelection, RateLimiting, DuplicateCheck, etc.)
- Platform abstraction - DiscordPlatform for posting
- State persistence - Database-backed execution tracking
- Schedule types - Interval, Cron, Once, Immediate

### The Problem

The narrative system and actor system are currently disconnected:
- **Narratives**: Executed manually via `just narrate`, no scheduling
- **Actors**: Skills-based, designed to query tables directly, no narrative execution capability

We need to bridge these two systems to enable scheduled, automated narrative execution.

---

## Proposed Architecture

### Three-Tier Integration Model

```
┌────────────────────────────────────────────────────────────┐
│  Actor Server                                              │
│  - Scheduling (intervals, cron)                            │
│  - Circuit breakers                                        │
│  - State persistence                                       │
│  - Execution tracking                                      │
└───────────────┬────────────────────────────────────────────┘
                │
                ▼
┌────────────────────────────────────────────────────────────┐
│  Skills Layer                                              │
│  - NarrativeExecutionSkill (NEW)                          │
│  - RateLimitingSkill (cross-cutting)                      │
│  - DuplicateCheckSkill (cross-cutting)                    │
└───────────────┬────────────────────────────────────────────┘
                │
                ▼
┌────────────────────────────────────────────────────────────┐
│  Narratives                                                │
│  - generation_carousel.toml                                │
│  - curate_and_approve.toml                                 │
│  - discord_poster.toml                                     │
└────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Create NarrativeExecutionSkill

**Goal**: Enable actors to execute narratives

**Tasks**:

1. Create `src/skills/narrative_execution.rs`
   - Implement `Skill` trait
   - Execute narrative files using `botticelli_narrative::Executor`
   - Pass database connection from actor to narrative
   - Return SkillOutput with narrative execution results

2. Register skill in `src/skills/mod.rs`

3. Add configuration options:
   ```toml
   [skills.narrative_execution]
   enabled = true
   narrative_path = "crates/botticelli_narrative/narratives/discord/discord_poster.toml"
   continue_on_error = true
   ```

**Example skill structure**:
```rust
pub struct NarrativeExecutionSkill {
    narrative_path: PathBuf,
}

impl Skill for NarrativeExecutionSkill {
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        // Load narrative
        let narrative = Narrative::load(&self.narrative_path)?;

        // Execute with database connection from context
        let executor = Executor::new(...);
        let result = executor.execute(narrative, &mut context.connection).await?;

        // Convert to SkillOutput
        Ok(SkillOutput {
            skill_name: "narrative_execution".to_string(),
            data: serde_json::json!({ "acts_completed": result.acts.len() }),
            metadata: HashMap::new(),
        })
    }
}
```

### Phase 2: Create Actor Configurations

**Goal**: Set up scheduled actors for generation, curation, and posting

**File Structure**:
```
actors/
├── generation_actor.toml      # Runs every 12 hours
├── curation_actor.toml         # Runs every 6 hours
└── posting_actor.toml          # Runs every 2 hours
```

**File**: `actors/posting_actor.toml`
```toml
[actor]
name = "discord_content_poster"
description = "Posts approved Discord content at scheduled intervals"
knowledge = ["approved_discord_posts"]
skills = ["narrative_execution"]

[actor.config]
max_posts_per_day = 8
min_interval_minutes = 120
timezone = "America/New_York"

[actor.execution]
stop_on_unrecoverable = true
max_retries = 3
continue_on_error = false

[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/discord_poster.toml"
```

**File**: `actors/curation_actor.toml`
```toml
[actor]
name = "discord_content_curator"
description = "Curates potential Discord posts"
knowledge = ["potential_discord_posts", "approved_discord_posts"]
skills = ["narrative_execution"]

[actor.config]
max_posts_per_day = 50  # Curation is not posting, so higher limit
min_interval_minutes = 360  # Every 6 hours
timezone = "America/New_York"

[actor.execution]
stop_on_unrecoverable = true
max_retries = 2
continue_on_error = true

[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/curate_and_approve.toml"
```

**File**: `actors/generation_actor.toml`
```toml
[actor]
name = "discord_content_generator"
description = "Generates potential Discord posts using carousel of narratives"
knowledge = ["potential_discord_posts"]
skills = ["narrative_execution"]

[actor.config]
max_posts_per_day = 50  # Generation limit
min_interval_minutes = 720  # Every 12 hours
timezone = "America/New_York"

[actor.execution]
stop_on_unrecoverable = true
max_retries = 2
continue_on_error = true

[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/generation_carousel.toml"
```

### Phase 3: Create Server Configuration

**Goal**: Configure actor-server to run all three actors

**File**: `actor_server.toml`
```toml
[server]
check_interval_seconds = 60  # Check every minute

[server.circuit_breaker]
max_consecutive_failures = 5
auto_pause = true
reset_on_success = true

# Generation Actor - Runs every 12 hours
[[actors]]
name = "discord_generation"
config_file = "actors/generation_actor.toml"
channel_id = "${TEST_GUILD_ID}"  # Not used for generation, but required by actor-server
enabled = true

[actors.schedule]
type = "Interval"
seconds = 43200  # 12 hours

# Curation Actor - Runs every 6 hours
[[actors]]
name = "discord_curation"
config_file = "actors/curation_actor.toml"
channel_id = "${TEST_GUILD_ID}"  # Not used for curation
enabled = true

[actors.schedule]
type = "Interval"
seconds = 21600  # 6 hours

# Posting Actor - Runs every 2 hours
[[actors]]
name = "discord_posting"
config_file = "actors/posting_actor.toml"
channel_id = "${POSTING_CHANNEL_ID}"  # Actual posting channel
enabled = true

[actors.schedule]
type = "Interval"
seconds = 7200  # 2 hours
```

### Phase 4: Add Post Status Tracking

**Goal**: Prevent duplicate posting by tracking posted content

**Option 1: SQL trigger (automatic)**
```sql
CREATE OR REPLACE FUNCTION mark_posted_after_discord_post()
RETURNS TRIGGER AS $$
BEGIN
    -- This would need to be triggered by application logic
    -- Postgres triggers can't detect narrative execution directly
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

**Option 2: Update discord_poster narrative (preferred)**

Modify `discord_poster.toml` to add a post-posting cleanup act that marks content as posted:

```toml
[toc]
order = ["get_channel", "extract_content", "post_to_channel", "mark_posted"]

# ... existing acts ...

[acts.mark_posted]
[[acts.mark_posted.input]]
type = "text"
content = """Execute this SQL to mark the post as posted:
UPDATE approved_discord_posts
SET review_status = 'posted', posted_at = NOW()
WHERE review_status = 'pending'
ORDER BY curation_score DESC, selected_at ASC
LIMIT 1;"""
```

**Option 3: Create database update bot command (best long-term solution)**

Implement in `botticelli_social`:
- `database.update_table` command
- Takes: table_name, where_clause, update_values
- Returns: number of rows updated

Then use in narrative:
```toml
[bots.mark_posted]
platform = "database"
command = "update_table"
table_name = "approved_discord_posts"
where_clause = "review_status = 'pending' ORDER BY curation_score DESC LIMIT 1"
update_values = { review_status = "posted", posted_at = "NOW()" }
```

---

## Testing Plan

### Phase 1 Testing: NarrativeExecutionSkill

```bash
# 1. Create minimal actor with narrative_execution skill
cargo build --features discord

# 2. Test narrative execution through skill
cargo run --example discord_poster --features discord

# 3. Verify narrative is executed and results returned
```

### Phase 2 Testing: Actor Configurations

```bash
# 1. Load actor config
actor-server --config actors/posting_actor.toml --dry-run

# 2. Verify configuration parses correctly
# 3. Check for validation warnings
```

### Phase 3 Testing: Server Execution

```bash
# 1. Start actor-server in foreground
export DATABASE_URL="postgresql://..."
export DISCORD_TOKEN="..."
export POSTING_CHANNEL_ID="1443633120430788659"  # content-test channel

actor-server --config actor_server.toml

# 2. Monitor logs for scheduled executions
# 3. Verify posts appear in Discord
# 4. Check circuit breaker works on failures
# 5. Test graceful shutdown (Ctrl+C)
```

### Phase 4 Testing: End-to-End Pipeline

```bash
# 1. Start server
actor-server --config actor_server.toml

# 2. Wait for generation actor (12 hours)
# 3. Wait for curation actor (6 hours)
# 4. Wait for posting actor (2 hours)
# 5. Verify posts flow through pipeline:
#    - potential_discord_posts → approved_discord_posts → Discord
```

---

## Success Criteria

### Phase 1
- ✅ NarrativeExecutionSkill compiles
- ✅ Can execute discord_poster.toml through skill
- ✅ Returns proper SkillOutput with execution metadata

### Phase 2
- ✅ All three actor configs load without errors
- ✅ No validation warnings
- ✅ Schedules are correctly configured

### Phase 3
- ✅ Actor-server starts successfully
- ✅ Actors execute on schedule
- ✅ Posts appear in Discord
- ✅ Circuit breaker pauses failing actors
- ✅ State persists across restarts

### Phase 4
- ✅ Full pipeline runs autonomously
- ✅ No duplicate posts
- ✅ Respects rate limits
- ✅ Handles errors gracefully

---

## Alternative Approach: Pure Skills (Not Recommended)

We could abandon narratives and implement posting as pure skills:

```rust
struct ContentPostingSkill {
    platform: Arc<dyn Platform>,
}

impl Skill for ContentPostingSkill {
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        // Query approved_discord_posts directly
        let post = query_next_post(&mut context.connection)?;

        // Post to Discord
        self.platform.post(post.text_content).await?;

        // Mark as posted
        mark_posted(&mut context.connection, post.id)?;

        Ok(SkillOutput { ... })
    }
}
```

**Why we're not doing this:**
1. Narratives provide better observability (execution records, act-by-act breakdown)
2. Multi-step flows are clearer in TOML than in Rust code
3. We've already built working narratives
4. LLM-based content extraction/transformation is easier in narratives
5. Easier for non-Rust developers to modify posting logic

---

## Next Steps

1. **Implement NarrativeExecutionSkill** (1-2 hours)
2. **Create actor config files** (30 minutes)
3. **Create server config** (15 minutes)
4. **Test locally** (1 hour)
5. **Add post tracking** (1 hour)
6. **Deploy to production** (30 minutes)

**Total estimated time**: 4-5 hours

---

## Questions to Resolve

1. **Channel ID requirement**: Actor-server requires channel_id for all actors, but generation/curation don't post. Should we make channel_id optional in ActorInstanceConfig?

2. **Database update command**: Should we implement `database.update_table` bot command now or use SQL-in-prompt workaround?

3. **Error handling**: If narrative execution fails, should the actor retry the entire narrative or skip to next schedule?

4. **State management**: Should narrative state (act outputs) be preserved in actor state, or just execution success/failure?
