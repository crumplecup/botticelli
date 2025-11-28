# Bot Server Implementation - Next Steps

**Status**: Core architecture implemented, needs debugging and narrative integration  
**Date**: 2025-11-27

## Current State

### ✅ Completed
- Multi-narrative TOML support (5 narratives per file)
- JSON schema mismatch handling (flexible insertion)
- Budget multiplier system (voluntary rate limiting)
- Bot server architecture (GenerationBot, CurationBot, PostingBot)
- CLI command (`just bot-server`)
- Configuration structure in `botticelli.toml`

### ❌ Blocking Issues
1. **Compilation errors** in bot server implementation
2. **Missing narratives** - curation and posting narratives not created yet
3. **Database schema** - `approved_posts` table not defined
4. **Discord integration** - posting bot needs Discord API connection
5. **Error handling** - bot failure/restart logic incomplete

---

## Phase 1: Fix Compilation Errors

**Priority**: CRITICAL  
**Effort**: 1-2 hours

### Issues to Resolve

1. **SkillContext trait bounds**
   - Location: `crates/botticelli_narrative/src/bot_server.rs`
   - Error: `SkillContext` not `Send`
   - Fix: Add `+ Send + Sync` bounds or use `Arc<Mutex<>>`

2. **NarrativeProvider lifetime issues**
   - Error: Temporary value dropped while borrowed
   - Fix: Store loaded narrative in struct, not temporary

3. **Type mismatches**
   - `select_posts` return type
   - `post_to_discord` signature
   - Fix: Align with actual database/Discord API

### Implementation Steps

```rust
// 1. Fix SkillContext bounds
pub trait SkillContext: Send + Sync {
    // ...
}

// 2. Store narrative in bot struct
pub struct CurationBot {
    narrative: Arc<ComposedNarrative>,
    // ...
}

// 3. Fix async signatures
async fn select_posts(
    &self,
    conn: &mut PgConnection,
    limit: i64,
) -> Result<Vec<ContentRow>, BotError>
```

---

## Phase 2: Create Curation Narrative

**Priority**: HIGH  
**Effort**: 2-3 hours

### Requirements

- Read from `potential_discord_posts` (status = 'generated')
- Score/rank posts by quality metrics
- Select top N posts (configurable, default 5)
- Move selected posts to `approved_posts`
- Update status to 'approved'

### Narrative Structure

```toml
# crates/botticelli_narrative/narratives/discord/curation.toml

[shared]
model = "gemini-2.0-flash-exp"
budget.rpm = 0.8

[narrative.evaluate]
description = "Evaluate post quality and assign scores"
toc = ["load_candidates", "score_posts", "rank_results"]

[act.load_candidates]
# Query potential_posts with status='generated'

[act.score_posts]
# Prompt LLM to evaluate quality (1-10 scale)
# Criteria: relevance, engagement, clarity, tone

[act.rank_results]
# Sort by score, select top N
# Insert into approved_posts
```

### Success Criteria

- Processes batch of posts in one run
- Continues until queue empty
- Logs scoring rationale
- Handles edge cases (no posts, all low quality)

---

## Phase 3: Create Posting Narrative

**Priority**: HIGH  
**Effort**: 3-4 hours

### Requirements

- Read from `approved_posts` (status = 'approved')
- Format for Discord (handle embeds, links, mentions)
- Post via Discord API
- Update status to 'posted'
- Log post URL/message ID

### Narrative Structure

```toml
# crates/botticelli_narrative/narratives/discord/posting.toml

[shared]
model = "gemini-2.0-flash-exp"

[narrative.post]
description = "Format and post approved content to Discord"
toc = ["load_next_post", "format_message", "send_to_discord"]

[act.load_next_post]
# Query approved_posts ORDER BY generated_at LIMIT 1

[act.format_message]
# Convert to Discord message format
# Handle character limits (2000 chars)
# Add embeds if needed

[act.send_to_discord]
# Use botticelli_social Discord API
# Post to configured channel
# Record message_id
```

### Discord Integration Points

```rust
// In posting_bot.rs
use botticelli_social::discord::{DiscordClient, MessageBuilder};

async fn post_to_discord(
    &self,
    post: &ContentRow,
) -> Result<MessageId, BotError> {
    let client = DiscordClient::new(self.discord_token.clone());
    let message = MessageBuilder::new()
        .content(post.text_content.clone())
        .build();
    
    client.send_message(self.channel_id, message).await
}
```

---

## Phase 4: Database Schema Updates

**Priority**: HIGH  
**Effort**: 1 hour

### Create Migration

```sql
-- migrations/YYYY-MM-DD-HHMMSS_create_approved_posts/up.sql

CREATE TABLE approved_posts (
    id SERIAL PRIMARY KEY,
    text_content TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'approved',
    score INTEGER,
    selection_reason TEXT,
    generated_at TIMESTAMP NOT NULL,
    approved_at TIMESTAMP NOT NULL DEFAULT NOW(),
    posted_at TIMESTAMP,
    message_id BIGINT,
    channel_id BIGINT,
    guild_id BIGINT
);

CREATE INDEX idx_approved_posts_status ON approved_posts(status);
CREATE INDEX idx_approved_posts_posted_at ON approved_posts(posted_at);
```

### Schema Module

```rust
// crates/botticelli_database/src/schema.rs
table! {
    approved_posts (id) {
        id -> Int4,
        text_content -> Text,
        status -> Varchar,
        score -> Nullable<Int4>,
        selection_reason -> Nullable<Text>,
        generated_at -> Timestamp,
        approved_at -> Timestamp,
        posted_at -> Nullable<Timestamp>,
        message_id -> Nullable<Int8>,
        channel_id -> Nullable<Int8>,
        guild_id -> Nullable<Int8>,
    }
}
```

---

## Phase 5: Error Handling & Observability

**Priority**: MEDIUM  
**Effort**: 2-3 hours

### Bot Health Monitoring

```rust
#[derive(Debug, Clone)]
pub struct BotHealth {
    pub status: BotStatus,
    pub last_run: DateTime<Utc>,
    pub runs_completed: u64,
    pub errors_count: u64,
    pub last_error: Option<String>,
}

pub enum BotStatus {
    Running,
    Idle,
    Error,
    Stopped,
}
```

### Failure Recovery

1. **Transient failures** (network, rate limit)
   - Exponential backoff retry
   - Max 3 retries per operation

2. **Persistent failures** (bad data, config error)
   - Log error details
   - Mark content as 'failed'
   - Continue with next item

3. **Critical failures** (database down, auth failure)
   - Stop bot gracefully
   - Alert admin (log CRITICAL)
   - Require manual restart

### Logging Strategy

```rust
// Use structured tracing
#[instrument(skip(self, conn))]
async fn process_batch(&self, conn: &mut PgConnection) {
    info!(bot = "generation", "Starting batch");
    
    match self.generate_content(conn).await {
        Ok(count) => {
            info!(bot = "generation", posts_created = count, "Batch complete");
        }
        Err(e) => {
            error!(bot = "generation", error = ?e, "Batch failed");
            self.health.lock().await.record_error(e);
        }
    }
}
```

---

## Phase 6: Configuration & Deployment

**Priority**: MEDIUM  
**Effort**: 2 hours

### Bot Configuration

```toml
# botticelli.toml

[bots.generation]
enabled = true
schedule = "0 */6 * * *"  # Every 6 hours
narrative = "./crates/botticelli_narrative/narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"
iterations = 10

[bots.curation]
enabled = true
schedule = "0 */12 * * *"  # Every 12 hours
narrative = "./crates/botticelli_narrative/narratives/discord/curation.toml"
narrative_name = "evaluate"
process_until_empty = true

[bots.posting]
enabled = true
interval_hours = 24
jitter_hours = 6
narrative = "./crates/botticelli_narrative/narratives/discord/posting.toml"
narrative_name = "post"
channel_id = 1234567890
```

### Deployment Checklist

- [ ] All tests passing (`just check-all`)
- [ ] Narratives validated (`just narrate <narrative>`)
- [ ] Database migrations applied (`diesel migration run`)
- [ ] Environment variables set (`.env` file)
- [ ] Discord bot token configured
- [ ] Channel IDs configured
- [ ] Budget limits appropriate for tier
- [ ] Logging level set (INFO for production)

---

## Phase 7: Testing & Validation

**Priority**: HIGH  
**Effort**: 3-4 hours

### Unit Tests

```rust
#[tokio::test]
async fn test_generation_bot_creates_posts() {
    let bot = GenerationBot::new(/* ... */);
    let count = bot.process_batch(&mut conn).await.unwrap();
    assert!(count > 0);
    
    let posts = get_potential_posts(&mut conn).await.unwrap();
    assert_eq!(posts.len(), count as usize);
}
```

### Integration Tests

1. **End-to-end pipeline**
   - Start all three bots
   - Verify generation → curation → posting flow
   - Check database state transitions

2. **Schedule accuracy**
   - Verify cron execution timing
   - Test jitter randomization

3. **Error recovery**
   - Simulate API failures
   - Verify retry logic
   - Check graceful degradation

### Manual Testing

```bash
# Test each bot individually
just bot-server --only generation
just bot-server --only curation
just bot-server --only posting

# Test full pipeline
just bot-server

# Monitor logs
tail -f logs/bot-server.log | grep -E "generation|curation|posting"
```

---

## Phase 8: Documentation

**Priority**: LOW  
**Effort**: 2 hours

### User Guide

- How to configure bots
- How to customize narratives
- How to monitor health
- Troubleshooting common issues

### Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                  Bot Server                         │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐   ┌──────────────┐   ┌─────────┐│
│  │ Generation   │   │  Curation    │   │ Posting ││
│  │ Bot          │   │  Bot         │   │ Bot     ││
│  │              │   │              │   │         ││
│  │ Every 6hrs   │   │ Every 12hrs  │   │ Daily   ││
│  │ 10 topics    │   │ Process all  │   │ +jitter ││
│  └──────┬───────┘   └──────┬───────┘   └────┬────┘│
│         │                  │                 │     │
└─────────┼──────────────────┼─────────────────┼─────┘
          │                  │                 │
          ▼                  ▼                 ▼
    ┌─────────────────────────────────────────────┐
    │           PostgreSQL Database               │
    ├─────────────────────────────────────────────┤
    │  potential_posts  →  approved_posts         │
    │   (generated)         (approved → posted)   │
    └─────────────────────────────────────────────┘
```

---

## Success Metrics

### Operational
- All 3 bots running without crashes (24+ hrs)
- <1% error rate per bot
- Average 50+ posts/day generated
- Average 5+ posts/day approved
- Average 1 post/day published

### Quality
- Curation selects genuinely good posts (manual review)
- Posted content appropriate for channel
- No duplicate posts
- Consistent posting schedule

### Performance
- Generation: <5min per batch
- Curation: <10min for 50 posts
- Posting: <30sec per post

---

## Open Questions

1. **Curation criteria** - What makes a "good" post?
   - Engagement potential?
   - Information density?
   - Brand alignment?

2. **Failure thresholds** - When to alert/stop?
   - 3 consecutive failures?
   - 10 failures per day?

3. **Content diversity** - Prevent topic clustering?
   - Track recent topics?
   - Enforce minimum topic spacing?

4. **Human oversight** - Manual approval step?
   - Flag for review instead of auto-post?
   - Admin dashboard?

---

## Next Immediate Actions

1. **Fix compilation errors** (1-2 hrs)
2. **Run `just check-all`** to verify builds
3. **Create `approved_posts` table** migration
4. **Implement curation narrative** (basic version)
5. **Test generation → curation flow**
6. **Document learnings** in this file

Once these are complete, reassess and continue with posting bot integration.
