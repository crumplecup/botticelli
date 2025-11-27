# Bot Server Deployment Plan

**Status**: Planning  
**Created**: 2025-11-27  
**Goal**: Deploy multi-phase bot server with generation, curation, and posting actors

---

## Architecture Overview

The bot server runs three independent actors that work together to create a content pipeline:

```
┌─────────────────────────────────────────────────────────┐
│                   Generation Actor                       │
│  Schedule: Daily (every 24 hours)                       │
│  Narrative: generation_carousel.batch_generate          │
│  Output: 50 posts → potential_discord_posts table       │
└────────────────────┬────────────────────────────────────┘
                     │
                     v
┌─────────────────────────────────────────────────────────┐
│                   Curation Actor                         │
│  Schedule: Every 12 hours                               │
│  Narrative: curate_and_approve.toml                     │
│  Input: potential_discord_posts (review_status=pending) │
│  Output: 2-3 posts → approved_discord_posts table       │
└────────────────────┬────────────────────────────────────┘
                     │
                     v
┌─────────────────────────────────────────────────────────┐
│                   Posting Actor                          │
│  Schedule: Every 2 hours                                │
│  Narrative: discord_poster.toml                         │
│  Input: approved_discord_posts (review_status=pending)  │
│  Output: Posts to Discord, marks as 'posted'            │
└─────────────────────────────────────────────────────────┘
```

---

## Current Status

### ✅ Completed Infrastructure

1. **Ractor Migration** - Actor system using tokio-native framework (no runtime conflicts)
2. **StorageActor** - Asynchronous database operations with connection pooling
3. **NarrativeExecutionSkill** - Actors can execute narratives with full database support
4. **Multi-narrative TOML** - Multiple narratives defined in single file with shared acts
5. **Schema Inference** - Tables auto-created from JSON output structure
6. **Budget Multipliers** - Rate limiting at 80% of quota (configurable per rate type)
7. **NoOpPlatform** - Actors can run without posting (for generation/curation)
8. **Bot Actor Skeletons** - Created GenerationBot, CurationBot, PostingBot in `botticelli_server/src/bots/`
   - Ractor-based actors with message handlers
   - Jitter calculation for natural posting rhythm
   - **CurationBot fully implemented** (spawns `botticelli run` CLI subprocess)
   - Architecture: Bots spawn CLI rather than re-implementing narrative loading

### ✅ Completed Narratives

1. **generation_carousel.toml** - 5 narratives × carousel iterations
   - Batch generator runs all five focus angles (feature, usecase, tutorial, community, problem_solution)
   - Each narrative: generate → critique → refine → format_json → audit_json
   - Outputs to `potential_discord_posts` table via `target = "potential_discord_posts"`
   - Successfully tested with 3 iterations (15 posts)

2. **curate_and_approve.toml** - LLM-based curation
   - Queries potential_discord_posts (review_status='pending')
   - Scores posts on 5 criteria (Engagement, Clarity, Value, Tone, Polish)
   - Selects top 2-3 posts for approval
   - Outputs to `approved_discord_posts` table
   - **Queue Processing**: Runs every 12 hours, processes ALL pending content in queue until empty

3. **discord_poster.toml** - Simple posting workflow
   - Queries approved_discord_posts (review_status='pending')
   - Posts to Discord channel
   - Successfully tested (posted 1,688-char message)

### ⚠️ Known Limitations

1. **No automatic status tracking** - Posted content must be marked manually:
   ```sql
   UPDATE approved_discord_posts SET review_status = 'posted' WHERE ...;
   ```
   **Solution**: Implement `database.update_table` bot command (Phase 2 below)

2. **No interval enforcement** - Posting actor will post immediately when run
   **Solution**: Actor-server scheduling handles this via `schedule.seconds`

3. **No time window constraints** - Posts at any time of day
   **Solution**: Requires actor-server configuration features (future enhancement)

---

## Deployment Phases

### Phase 1: Implement Database Update Command ✅

**Status**: COMPLETE (implemented in ACTOR_INTEGRATION_PROGRESS.md Phase 2)

**What exists:**
- `DatabaseCommandExecutor` with `update_table` command
- Parameterized queries via diesel
- Table whitelist validation
- PostgreSQL-compatible UPDATE with LIMIT via subquery
- Registered in BotCommandRegistry

**Next step**: Update `discord_poster.toml` to use this command (Phase 2)

---

### Phase 2: Update Posting Narrative

**Goal**: Add automatic status tracking to prevent duplicate posts

**File to modify**: `crates/botticelli_narrative/narratives/discord/discord_poster.toml`

**Changes needed**:

```toml
# Add fourth act to mark content as posted
[acts.mark_posted]
[[acts.mark_posted.input]]
type = "bot_command"
platform = "database"
command = "update_table"

[acts.mark_posted.input.args]
table_name = "approved_discord_posts"
where_clause = "review_status = 'pending'"
limit = 1

[acts.mark_posted.input.args.updates]
review_status = "posted"
posted_at = "NOW()"
```

**Update narrative TOC**:
```toml
[narrative.toc]
order = ["get_channel", "extract_content", "post_to_channel", "mark_posted"]
```

**Testing**:
1. Run posting narrative twice
2. Verify first run posts content
3. Verify second run finds no pending posts (or posts next one)

**Success criteria**:
- ✅ Posts content to Discord
- ✅ Updates review_status to 'posted'
- ✅ Sets posted_at timestamp
- ✅ No duplicate posts on subsequent runs

---

### Phase 3: Create Actor Configurations

**Goal**: Define TOML configs for each actor with schedules and skills

**Directory structure**:
```
actors/
├── generation_actor.toml      # Daily content generation
├── curation_actor.toml         # Every 12 hours
└── posting_actor.toml          # Every 2 hours
```

#### Generation Actor Config

**File**: `actors/generation_actor.toml`

```toml
[actor]
name = "Content Generation Actor"
description = "Generates 50 posts daily using carousel batch mode"

[actor.schedule]
type = "Interval"
seconds = 86400  # Daily (24 hours)
jitter_percent = 10  # ±10% randomization (±2.4 hours)

[actor.execution]
continue_on_error = false  # Stop if generation fails
stop_on_unrecoverable = true
max_retries = 2
processing_strategy = "single_run"  # Run once per trigger

[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"  # Runs carousel with 10 iterations (50 posts)

# No platform configuration - uses NoOpPlatform (no channel_id)
```

**Key decisions**:
- **Daily schedule**: 50 posts/day is sustainable with budget multipliers (80% of quota)
- **No channel_id**: Generation doesn't post to Discord, only database
- **Stop on error**: Generation failures should be investigated (not silent)
- **Batch size**: 10 iterations × 5 narratives = 50 posts
  - Adjust `carousel.iterations` in generation_carousel.toml to change batch size

#### Curation Actor Config

**File**: `actors/curation_actor.toml`

```toml
[actor]
name = "Content Curation Actor"
description = "Curates best posts from potential pool every 12 hours, processes until queue empty"

[actor.schedule]
type = "Interval"
seconds = 43200  # Every 12 hours
jitter_percent = 15  # ±15% randomization (±1.8 hours)

[actor.execution]
continue_on_error = true  # Continue even if no posts to curate
stop_on_unrecoverable = true
max_retries = 3
processing_strategy = "drain_queue"  # Process ALL pending content until empty

[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/curate_and_approve.toml"
narrative_name = "curate_and_approve"

# No platform configuration - uses NoOpPlatform
```

**Key decisions**:
- **12-hour schedule**: Gives time for generation to accumulate posts
- **Drain queue strategy**: Processes ALL pending content in one session, prevents bottleneck
  - Runs narrative in loop: checks for pending → curates batch → repeat until empty
  - Prevents generation from outpacing curation
- **Continue on error**: If no posts ready, just skip this run
- **Max retries**: LLM-based curation can be flaky, allow retries

#### Posting Actor Config

**File**: `actors/posting_actor.toml`

```toml
[actor]
name = "Discord Posting Actor"
description = "Posts approved content to Discord with natural timing variation"

[actor.schedule]
type = "Interval"
seconds = 7200  # Base: Every 2 hours
jitter_percent = 25  # ±25% randomization (±30 minutes)
# Results in posts every 1.5-2.5 hours with natural variation

[actor.execution]
continue_on_error = true  # Continue if no posts available
stop_on_unrecoverable = true
max_retries = 2

[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/discord_poster.toml"
narrative_name = "discord_poster"

[platform]
type = "discord"
channel_id = "${POSTING_CHANNEL_ID}"  # Set via environment variable
```

**Key decisions**:
- **2-hour base interval with 25% jitter**: Simulates human posting behavior
  - Actual intervals: 1.5-2.5 hours randomly distributed
  - Prevents predictable/bot-like timing patterns
  - Average: ~12 posts/day (if all approved)
- **Continue on error**: If no posts ready, just skip this run
- **Environment variable**: Allows testing vs production channel switching

**Environment setup**:
```bash
export POSTING_CHANNEL_ID="1234567890"  # Production channel
export POSTING_CHANNEL_ID="0987654321"  # Test channel (for dry-run)
```

---

### Phase 4: Create Server Configuration

**Goal**: Single configuration file that registers all three actors

**File**: `actor_server.toml`

```toml
[server]
name = "Discord Content Pipeline"
description = "Automated content generation, curation, and posting"
check_interval_seconds = 60  # How often to check actor schedules
state_persistence_enabled = true

[server.circuit_breaker]
enabled = true
failure_threshold = 5
reset_timeout_seconds = 300  # 5 minutes

[server.observability]
metrics_enabled = true
health_endpoint = "http://localhost:8080/health"
log_level = "info"

# Three actors working together
[[actors]]
config_file = "actors/generation_actor.toml"

[[actors]]
config_file = "actors/curation_actor.toml"

[[actors]]
config_file = "actors/posting_actor.toml"

[database]
url = "${DATABASE_URL}"
pool_size = 10
connection_timeout_seconds = 30
```

**Key features**:
- **Circuit breaker**: Automatically disables failing actors after threshold
- **State persistence**: Tracks last execution time, success/failure
- **Health endpoint**: Monitor server status via HTTP
- **Shared database pool**: All actors use same connection pool

---

### Phase 5: Testing & Validation

#### Step 1: Dry-Run Validation (30 min)

**Goal**: Verify configurations load without errors

```bash
# Check server config parses
cargo run --bin actor-server -- --config actor_server.toml --dry-run

# Check individual actor configs
cargo run --bin actor-server -- --actor actors/generation_actor.toml --dry-run
cargo run --bin actor-server -- --actor actors/curation_actor.toml --dry-run
cargo run --bin actor-server -- --actor actors/posting_actor.toml --dry-run
```

**Expected output**:
- ✅ All TOML files parse successfully
- ✅ Narratives load from specified paths
- ✅ Database connection established
- ✅ Skills registered correctly
- ✅ No missing environment variables

#### Step 2: Single Actor Execution (1 hour)

**Goal**: Test each actor independently before running together

**Generation actor test**:
```bash
# Test with 1 iteration (5 posts) first
# Modify generation_carousel.toml: carousel.iterations = 1
just narrate generation_carousel.batch_generate

# Verify database
psql $DATABASE_URL -c "SELECT COUNT(*) FROM potential_discord_posts WHERE review_status = 'pending';"
# Expected: 5 new rows

# If successful, test full batch (10 iterations = 50 posts)
# Modify generation_carousel.toml: carousel.iterations = 10
just narrate generation_carousel.batch_generate

# Verify database
psql $DATABASE_URL -c "SELECT COUNT(*) FROM potential_discord_posts WHERE review_status = 'pending';"
# Expected: 50 new rows
```

**Curation actor test**:
```bash
just narrate curate_and_approve.curate_and_approve

# Verify database
psql $DATABASE_URL -c "SELECT COUNT(*) FROM approved_discord_posts WHERE review_status = 'pending';"
# Expected: 2-3 new rows
```

**Posting actor test** (use test channel):
```bash
export POSTING_CHANNEL_ID="<test-channel-id>"
just narrate discord_poster.discord_poster

# Verify Discord
# Check test channel for new post

# Verify database
psql $DATABASE_URL -c "SELECT COUNT(*) FROM approved_discord_posts WHERE review_status = 'posted';"
# Expected: 1 row updated
```

#### Step 3: Full Server Execution (2 hours)

**Goal**: Run all three actors together with monitoring

**Start server**:
```bash
# Export environment variables
export DATABASE_URL="postgresql://user:pass@localhost/botticelli"
export DISCORD_TOKEN="your-discord-bot-token"
export GEMINI_API_KEY="your-gemini-api-key"
export POSTING_CHANNEL_ID="<test-channel-id>"  # Use test channel first

# Enable debug logging
export RUST_LOG="info,botticelli_actor=debug,botticelli_server=debug"

# Start server
cargo run --bin actor-server -- --config actor_server.toml
```

**Monitor execution**:
```bash
# Watch logs in separate terminal
tail -f actor_server.log

# Check health endpoint
curl http://localhost:8080/health

# Monitor database
watch -n 60 'psql $DATABASE_URL -c "
  SELECT
    CASE
      WHEN review_status = '\''pending'\'' THEN '\''potential'\''
      WHEN review_status = '\''posted'\'' THEN '\''posted'\''
      ELSE '\''approved'\''
    END as stage,
    COUNT(*) as count
  FROM potential_discord_posts
  GROUP BY review_status
  UNION ALL
  SELECT '\''approved'\'' as stage, COUNT(*) FROM approved_discord_posts WHERE review_status = '\''pending'\'';
"'
```

**Expected behavior over 24 hours**:

| Time | Generation | Curation | Posting | Status |
|------|-----------|----------|---------|--------|
| 00:00 | 50 posts | - | - | 50 potential |
| 12:00 | - | 3 approved | - | 47 potential, 3 approved |
| 14:00 | - | - | 1 posted | 47 potential, 2 approved, 1 posted |
| 16:00 | - | - | 1 posted | 47 potential, 1 approved, 2 posted |
| 18:00 | - | - | 1 posted | 47 potential, 0 approved, 3 posted |
| 00:00 | 50 posts | 3 approved | - | 97 potential, 3 approved, 3 posted |

#### Step 4: End-to-End Validation (1 hour)

**Goal**: Verify complete pipeline from generation to Discord post

**Validation steps**:

1. **Generation completeness**:
   ```sql
   SELECT
     source_narrative,
     COUNT(*) as count
   FROM potential_discord_posts
   WHERE generated_at > NOW() - INTERVAL '1 day'
   GROUP BY source_narrative;
   ```
   Expected: ~10 posts per narrative type (feature, usecase, tutorial, community, problem_solution)

2. **Curation quality**:
   ```sql
   SELECT
     curation_score,
     LEFT(text_content, 100) as preview
   FROM approved_discord_posts
   WHERE selected_at > NOW() - INTERVAL '1 day'
   ORDER BY curation_score DESC;
   ```
   Expected: Top-scored posts (45-50 range)

3. **Posting success**:
   ```sql
   SELECT
     posted_at,
     LENGTH(text_content) as char_count,
     LEFT(text_content, 100) as preview
   FROM approved_discord_posts
   WHERE review_status = 'posted'
   ORDER BY posted_at DESC
   LIMIT 5;
   ```
   Expected: Posts at ~2 hour intervals

4. **Discord verification**:
   - Check test channel for posts
   - Verify formatting (Discord markdown, emojis)
   - Verify no duplicates
   - Verify appropriate pacing

**Success criteria**:
- ✅ 50 posts generated daily
- ✅ 2-3 posts curated every 12 hours
- ✅ 1 post published every 2 hours (when approved posts available)
- ✅ No duplicate posts
- ✅ All posts under 2000 characters
- ✅ Proper Discord formatting
- ✅ Zero actor crashes or unrecoverable errors

---

### Phase 6: Production Deployment

**Goal**: Deploy to production Discord channel with monitoring

**Pre-deployment checklist**:
- [ ] All Phase 5 tests passing
- [ ] Review all generated content for quality
- [ ] Verify posting intervals are appropriate
- [ ] Set up monitoring/alerting
- [ ] Configure production channel ID
- [ ] Document rollback procedure

**Production configuration changes**:

```bash
# Switch to production channel
export POSTING_CHANNEL_ID="<production-channel-id>"

# Reduce logging noise
export RUST_LOG="info,botticelli_actor=info,botticelli_server=info"
```

**Launch checklist**:
1. [ ] Start server with production config
2. [ ] Monitor first generation cycle (24 hours)
3. [ ] Review curated posts before first posting cycle
4. [ ] Monitor first 3 posts for quality/formatting
5. [ ] Check for duplicate posts
6. [ ] Verify no rate limit violations
7. [ ] Monitor for 7 days before considering stable

**Monitoring dashboards**:

```sql
-- Pipeline health (last 7 days)
SELECT
  DATE_TRUNC('day', generated_at) as day,
  COUNT(*) as potential_posts,
  COUNT(*) FILTER (WHERE review_status = 'pending') as pending,
  COUNT(*) FILTER (WHERE review_status = 'posted') as posted
FROM potential_discord_posts
WHERE generated_at > NOW() - INTERVAL '7 days'
GROUP BY day
ORDER BY day DESC;

-- Curation effectiveness
SELECT
  AVG(curation_score) as avg_score,
  MIN(curation_score) as min_score,
  MAX(curation_score) as max_score,
  COUNT(*) as total_approved
FROM approved_discord_posts
WHERE selected_at > NOW() - INTERVAL '7 days';

-- Posting frequency
SELECT
  DATE_TRUNC('hour', posted_at) as hour,
  COUNT(*) as posts_per_hour
FROM approved_discord_posts
WHERE review_status = 'posted'
  AND posted_at > NOW() - INTERVAL '7 days'
GROUP BY hour
ORDER BY hour DESC;
```

**Rollback procedure**:

```bash
# Emergency stop
pkill -f actor-server

# Pause posting only (keep generation/curation running)
psql $DATABASE_URL -c "UPDATE actor_server_state SET is_paused = true WHERE actor_name = 'Discord Posting Actor';"

# Resume posting
psql $DATABASE_URL -c "UPDATE actor_server_state SET is_paused = false WHERE actor_name = 'Discord Posting Actor';"
```

---

## Configuration Tuning

### After First Week

**Review metrics**:
- Generation quality (manual review sample)
- Curation accuracy (approved vs rejected)
- Posting engagement (Discord reactions/responses)

**Potential adjustments**:

1. **Generation frequency**:
   - Too many posts → Reduce carousel iterations (10 → 5)
   - Not enough variety → Increase iterations (10 → 15)

2. **Curation threshold**:
   - Too strict → Lower minimum score (35 → 30)
   - Too lenient → Raise minimum score (35 → 40)

3. **Posting interval**:
   - Too frequent → Increase interval (2h → 3h)
   - Too slow → Decrease interval (2h → 1.5h)

4. **Model selection**:
   - Quality issues → Switch to gemini-2.0-flash for generation
   - Rate limits → Stick with gemini-2.5-flash-lite

### Budget Multiplier Tuning

Current default: 80% of all rate limits

```toml
# botticelli.toml
[budget]
rpm = 0.8   # 80% of requests per minute
rpd = 0.8   # 80% of requests per day
tpm = 0.8   # 80% of tokens per minute
tpd = 0.8   # 80% of tokens per day
```

**If hitting rate limits**:
- Reduce multipliers to 0.6-0.7 (60-70%)
- Reduce carousel iterations
- Increase actor intervals

**If well below limits**:
- Increase multipliers to 0.9 (90%)
- Increase carousel iterations for more content

---

## Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 1: Database update command | COMPLETE | None |
| Phase 2: Update posting narrative | 30 min | Phase 1 |
| Phase 3: Create actor configs | 1 hour | Phase 2 |
| Phase 4: Create server config | 30 min | Phase 3 |
| Phase 5: Testing & validation | 4 hours | Phase 4 |
| Phase 6: Production deployment | Ongoing | Phase 5 |
| **Total to production** | **6 hours** | |

---

## Success Metrics

### Week 1 Goals

- ✅ Server runs 7 days without crashes
- ✅ 50 posts generated daily
- ✅ 4-6 posts curated and approved daily
- ✅ 4-6 posts published to Discord daily
- ✅ Zero duplicate posts
- ✅ Zero rate limit violations
- ✅ All posts under 2000 characters

### Week 2+ Goals

- Maintain >99% server uptime
- Average curation score >40/50
- Positive community engagement (reactions, replies)
- Low manual intervention rate (<5% of posts)

---

## Future Enhancements

### Short-term (Next Month)

1. **Time window constraints** - Only post during active hours (9am-9pm)
2. **Engagement feedback loop** - Track Discord reactions, adjust scoring
3. **Post editing** - Update posts based on feedback
4. **Multi-channel support** - Different content for announcements vs general

### Medium-term (3-6 Months)

1. **A/B testing** - Compare post variants
2. **Seasonal themes** - Adjust topics based on calendar
3. **Interactive curation** - Web UI for manual approval
4. **Content calendar** - Schedule posts for specific dates

### Long-term (6+ Months)

1. **Thread support** - Multi-message narratives
2. **Media generation** - Include images via DALL-E
3. **Cross-platform** - Extend to Twitter, Mastodon
4. **Analytics dashboard** - Comprehensive metrics visualization

---

## References

- [Actor Integration Progress](ACTOR_INTEGRATION_PROGRESS.md)
- [Discord Posting Strategy](crates/botticelli_narrative/narratives/discord/DISCORD_POSTING_STRATEGY.md)
- [Actor Architecture](ACTOR_ARCHITECTURE.md)
- [Actor Server Observability](ACTOR_SERVER_OBSERVABILITY.md)
