# Discord Content Posting Strategy

**Status**: Planning  
**Created**: 2025-11-24  
**Goal**: Automated content generation, curation, and posting pipeline using bot orchestration

## Overview

This document outlines a three-stage pipeline for autonomous Discord content posting:

1. **Generation Actor** - Populates `potential_posts` table using generate-critique-refine carousel
2. **Curation Actor** - Simulates human curation, selecting best posts for `approved_posts` table
3. **Posting Actor** - Posts approved content to Discord at semi-regular intervals

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Stage 1: Generation                          │
│                                                                   │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  Generate    │───>│   Critique   │───>│   Refine     │      │
│  │   (Act 1)    │    │   (Act 2)    │    │   (Act 3)    │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                                         │              │
│         └─────────────────────────────────────────┘              │
│                           │                                      │
│                           v                                      │
│                  ┌─────────────────┐                             │
│                  │ potential_posts │ <── Carousel (5-10 variants)│
│                  └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              v
┌─────────────────────────────────────────────────────────────────┐
│                    Stage 2: Curation                            │
│                                                                   │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  High-Quality Model (e.g., gemini-2.0-flash-thinking)│       │
│  │  - Analyzes all potential posts                      │       │
│  │  - Scores based on criteria (engagement, clarity)    │       │
│  │  - Selects top 2-3 for improvement                   │       │
│  │  - Refines selected posts                            │       │
│  └──────────────────────────────────────────────────────┘       │
│                           │                                      │
│                           v                                      │
│                  ┌─────────────────┐                             │
│                  │ approved_posts  │ <── Best 2-3 posts          │
│                  └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              v
┌─────────────────────────────────────────────────────────────────┐
│                    Stage 3: Publishing                          │
│                                                                   │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  Posting Actor                                        │       │
│  │  - Queries approved_posts (not yet posted)           │       │
│  │  - Checks interval/rate limits                       │       │
│  │  - Posts to Discord channel                          │       │
│  │  - Records in post_history                           │       │
│  │  - Updates content.post_count                        │       │
│  └──────────────────────────────────────────────────────┘       │
│                           │                                      │
│                           v                                      │
│                  ┌─────────────────┐                             │
│                  │ Discord Channel │                             │
│                  └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

## Database Schema

### Tables Required

**`potential_discord_posts` table** - Created via schema inference (auto-storage):

- Used by generation narratives via `target = "potential_discord_posts"`
- Schema automatically inferred from JSON output by ContentGenerationProcessor
- Acts as staging area for generated content
- No explicit migration needed - created on first successful generation

**Actual schema** (inferred from generation output):
```sql
-- potential_discord_posts table (auto-created via schema inference)
CREATE TABLE potential_discord_posts (
    text_content TEXT NOT NULL,
    source VARCHAR(255),
    tags TEXT[],
    content_type VARCHAR(50) NOT NULL,
    generated_at TIMESTAMP DEFAULT NOW() NOT NULL,
    source_narrative VARCHAR(255),
    source_act VARCHAR(255),
    generation_model VARCHAR(100),
    review_status VARCHAR(50),
    rating INTEGER
);

-- Query examples:
-- For curation:    SELECT * FROM potential_discord_posts ORDER BY generated_at DESC LIMIT 10
-- By review status: SELECT * FROM potential_discord_posts WHERE review_status = 'pending'
```

**`content` table** - Used for approved posts and posting workflows:

- Approved posts copied from `potential_discord_posts`
- Tag-based workflow stages: `["approved"]` → posted
- Existing table (migration 2025-11-23-215323-0000)

**Workflow:**
1. Generation → `potential_discord_posts` (auto-stored, review_status='pending')
2. Curation → Analyze posts, identify top 2-3
3. Manual approval → Copy to `content` table with tags=["approved"]
4. Posting → Update post_count, mark as posted

### Content Table Structure

```rust
pub struct Content {
    pub id: i32,
    pub content_type: String,        // "discord_post", "announcement", etc.
    pub text_content: Option<String>, // The actual post content
    pub media_urls: Option<Vec<String>>,
    pub media_types: Option<Vec<String>>,
    pub source: Option<String>,      // "generation_actor", "curation_actor"
    pub priority: Option<i32>,       // Higher = more important
    pub tags: Option<Vec<String>>,   // ["potential"], ["approved"], etc.
    pub approved_at: Option<NaiveDateTime>,
    pub approved_by: Option<String>, // "curation_actor"
    pub scheduled_for: Option<NaiveDateTime>,
    pub expires_at: Option<NaiveDateTime>,
    pub post_count: Option<i32>,     // 0 = not posted, >0 = posted
    pub last_posted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub metadata: Option<serde_json::Value>,
}
```

## Stage 1: Generation Actor

### Purpose

Generate multiple post variants using five independent narratives that each follow the generate-critique-refine pattern. Each narrative auto-stores to the `potential_posts` table on completion.

### Key Design Decisions

**Multiple narratives in one TOML:**

- Five separate `[narrative.X]` sections in one file
- Each narrative has `template = "potential_posts"` for schema inference
- Each narrative has `target = "potential_posts"` to write to shared table
- Each has `toc = ["generate", "critique", "refine"]` - storage triggers after refine
- **Batch generator** with `carousel.iterations = 10` runs all five narratives 10 times
- Carousel mode loops through `toc` entries for specified iterations
- Shared resource definitions (`[media]`, `[acts]`) reduce duplication

**Context injection:**

- `BOTTICELLI_CONTEXT.md` loaded once via media reference
- Provides LLM with comprehensive Botticelli knowledge
- No need to repeat information in each prompt

**Model selection:**

- **gemini-2.5-flash-lite** for highest daily request limit
- Critical for batch generation (50 posts = 150 API calls: 50 generate + 50 critique + 50 refine)
- Temperature 0.8 for creative variety across iterations
- Max tokens 600 (Discord limit is 2000, but posts target ~1800)

**Five focus angles:**

1. **Feature showcase** - Highlight specific capabilities
2. **Use cases** - Real-world applications
3. **Tutorial** - How-to guides
4. **Community** - Engagement and participation
5. **Problem-solution** - Challenges Botticelli solves

### Narrative: `generation_carousel.toml`

```toml
# Discord Post Generation Carousel
# Five narratives that each generate-critique-refine a single post
# Each narrative auto-stores to potential_posts table at completion

# === Batch Generator: Run all five narratives 10 times (50 posts) ===
[narrative.batch_generate]
name = "generation_batch_50"
description = "Run all five generation narratives 10 times using carousel mode"

[narrative.batch_generate.carousel]
iterations = 10

[narrative.batch_generate.toc]
order = ["feature_showcase", "use_cases", "tutorial", "community", "problem_solution"]

# === Shared Model Configuration ===
[config.gemini]
model = "gemini-2.5-flash-lite"
temperature = 0.8
max_tokens = 600

# === Narrative 1: Feature Showcase ===
[narrative.feature_showcase]
name = "potential_posts_feature"
description = "Generate Discord post showcasing a Botticelli feature"
template = "potential_posts"

[narrative.feature_showcase.toc]
order = ["generate", "critique", "refine"]

# === Narrative 2: Use Cases ===
[narrative.use_cases]
name = "potential_posts_usecase"
description = "Generate Discord post about real-world Botticelli use cases"
template = "potential_posts"

[narrative.use_cases.toc]
order = ["generate", "critique", "refine"]

# === Narrative 3: Tutorial ===
[narrative.tutorial]
name = "potential_posts_tutorial"
description = "Generate Discord post with tutorial or how-to content"
template = "potential_posts"

[narrative.tutorial.toc]
order = ["generate", "critique", "refine"]

# === Narrative 4: Community ===
[narrative.community]
name = "potential_posts_community"
description = "Generate Discord post for community engagement"
template = "potential_posts"

[narrative.community.toc]
order = ["generate", "critique", "refine"]

# === Narrative 5: Problem-Solution ===
[narrative.problem_solution]
name = "potential_posts_problemsolution"
description = "Generate Discord post about problems Botticelli solves"
template = "potential_posts"

[narrative.problem_solution.toc]
order = ["generate", "critique", "refine"]

# === Shared Resources ===

# Load Botticelli context for the LLM
[media.context]
file = "./BOTTICELLI_CONTEXT.md"

# === Shared Act Definitions ===

[acts.generate]
[[acts.generate.input]]
type = "media"
reference = "media.context"

[[acts.generate.input]]
type = "text"
content = """
Read the Botticelli context document above. Use this information to create an engaging Discord post.

{% if narrative_name == "potential_posts_feature" %}
Focus: Feature showcase
Angle: Highlight a specific Botticelli capability
{% elif narrative_name == "potential_posts_usecase" %}
Focus: Real-world use cases
Angle: Show practical applications
{% elif narrative_name == "potential_posts_tutorial" %}
Focus: Tutorial or how-to guide
Angle: Educational, step-by-step
{% elif narrative_name == "potential_posts_community" %}
Focus: Community engagement
Angle: Encourage participation, questions, sharing
{% elif narrative_name == "potential_posts_problemsolution" %}
Focus: Problem-solution narrative
Angle: Common challenges Botticelli solves
{% endif %}

Requirements:
- Length: Under 1800 characters
- Format: Discord markdown (**, *, __, ~~, ||) with emojis
- Structure: Hook, explanation, example, call-to-action
- Tone: Enthusiastic but professional

Output only the post text as JSON:
{
  "text_content": "Your post here...",
  "content_type": "discord_post",
  "source": "generation_carousel",
  "tags": ["potential", "{% if narrative_name == "potential_posts_feature" %}feature{% elif narrative_name == "potential_posts_usecase" %}usecase{% elif narrative_name == "potential_posts_tutorial" %}tutorial{% elif narrative_name == "potential_posts_community" %}community{% else %}problem_solution{% endif %}"]
}
"""

[acts.critique]
[[acts.critique.input]]
type = "text"
content = """
Critique this Discord post:

{{generate}}

Evaluate:
1. Engagement (hook, CTA)
2. Clarity (structure, readability)
3. Accuracy (matches Botticelli context)
4. Discord formatting (markdown, emojis)
5. Length (under 1800 chars)

Provide specific improvements.
"""

[acts.refine]
[[acts.refine.input]]
type = "text"
content = """
Improve the post based on critique:

Original: {{generate}}

Critique: {{critique}}

Output the refined post as JSON:
{
  "text_content": "Improved post here...",
  "content_type": "discord_post",
  "source": "generation_carousel",
  "tags": ["potential", "{% if narrative_name == "potential_posts_feature" %}feature{% elif narrative_name == "potential_posts_usecase" %}usecase{% elif narrative_name == "potential_posts_tutorial" %}tutorial{% elif narrative_name == "potential_posts_community" %}community{% else %}problem_solution{% endif %}"]
}
"""
```

### Actor Configuration: `generation_actor.toml`

```toml
[actor]
name = "Content Generation Actor"
description = "Generates multiple post variants for curation"

[actor.schedule]
type = "Interval"
seconds = 86400  # Daily

[actor.execution]
narrative_file = "narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"  # Run with carousel mode (10 iterations × 5 = 50 posts)
stop_on_unrecoverable = true
continue_on_error = false

# Alternative configurations:
# narrative_name = "feature_showcase"  # Run just one narrative (1 post/day)
# Adjust iterations in batch_generate.carousel for different batch sizes
```

### Expected Output

After running `batch_generate` with 10 iterations, the `potential_posts` table contains 50 new rows:

- `content_type`: "discord_post"
- `tags`: ["potential", "feature"|"usecase"|"tutorial"|"community"|"problem_solution"]
- `text_content`: Refined post text (JSON extracted)
- `source`: "generation_carousel"
- `post_count`: 0
- `approved_at`: NULL

**How it works:**

1. Run batch generator: `just narrate generation_carousel.batch_generate`
2. Carousel mode loops 10 times through the toc:
   - **Iteration 1**: feature → usecase → tutorial → community → problem_solution (5 posts)
   - **Iteration 2**: feature → usecase → tutorial → community → problem_solution (5 posts)
   - ... (8 more iterations)
   - **Iteration 10**: feature → usecase → tutorial → community → problem_solution (5 posts)
3. Each narrative completes its 3-act cycle (generate → critique → refine)
4. Auto-storage triggers after each refine, parsing JSON and inserting row
5. Result: **50 posts** in `potential_posts` table with diverse focus angles

**Flexibility:**

- **Batch of 50**: `just narrate generation_carousel.batch_generate` (10 iterations × 5 narratives)
- **Single post**: `just narrate generation_carousel.feature_showcase` (1 post)
- **Custom batch size**: Adjust `iterations` in carousel config
- **Daily generation**: Run batch_generate once per day via actor

## Stage 2: Curation Actor

### Purpose

Simulate human curation by using a high-quality model to analyze potential posts, select the best, and improve them for approval.

### Narrative: `curation_pipeline.toml`

```toml
[narrative]
name = "discord_post_curation"
description = "Curate and approve best posts from potential pool"

[toc]
order = ["fetch_potential", "analyze_batch", "select_best", "refine_selected", "approve"]

[config.gemini]
model = "gemini-2.5-flash"  # Higher quality for curation
temperature = 0.3  # More conservative
max_tokens = 2000

# Fetch unreviewed potential posts
[tables.potential]
table_name = "content"
where_clause = "'potential' = ANY(tags) AND NOT 'approved' = ANY(tags)"
order_by = "created_at DESC"
limit = 10
alias = "potential"

[acts.fetch_potential]
[[acts.fetch_potential.input]]
type = "table"
reference = "tables.potential"

[acts.analyze_batch]
[[acts.analyze_batch.input]]
type = "text"
content = """
You are a content curator for a technical Discord community.

Analyze these potential posts:

{{fetch_potential}}

For each post, score (1-10) on:
1. **Engagement** - Hook quality, CTA effectiveness
2. **Clarity** - Structure, readability, focus
3. **Value** - Information quality, usefulness
4. **Tone** - Appropriateness for audience
5. **Polish** - Grammar, formatting, emoji usage

Provide scores in JSON format:
[
  {
    "id": 1,
    "scores": {"engagement": 8, "clarity": 9, "value": 7, "tone": 8, "polish": 9},
    "total": 41,
    "strengths": ["..."],
    "weaknesses": ["..."]
  },
  ...
]
"""

[acts.select_best]
[[acts.select_best.input]]
type = "text"
content = """
Based on the analysis:

{{analyze_batch}}

Select the TOP 2-3 posts with highest total scores.
Consider diversity - avoid too-similar posts.

Output the selected post IDs and rationale for each selection.
"""

[acts.refine_selected]
[[acts.refine_selected.input]]
type = "text"
content = """
For each selected post, create an improved version.

Original posts and analysis:
{{analyze_batch}}

Selected posts:
{{select_best}}

For each selected post:
1. Address identified weaknesses
2. Enhance strengths
3. Ensure Discord best practices
4. Keep under 1800 characters

Output refined versions clearly labeled by ID.
"""

[acts.approve]
[[acts.approve.input]]
type = "bot_command"
platform = "database"
command = "content.update_by_ids"
args = {
    ids = "{{extract_ids_from(select_best)}}",
    updates = {
        tags = ["approved"],
        text_content = "{{extract_refined_from(refine_selected)}}",
        approved_at = "NOW()",
        approved_by = "curation_actor"
    }
}
```

### Actor Configuration: `curation_actor.toml`

```toml
[actor]
name = "Content Curation Actor"
description = "Curates best posts from potential pool"

[actor.schedule]
type = "Interval"
seconds = 43200  # Every 12 hours

[actor.execution]
narrative_file = "narratives/discord/curation_pipeline.toml"
stop_on_unrecoverable = true
max_retries = 3

[actor.config]
min_posts_for_review = 5  # Wait until at least 5 potential posts
target_approved_count = 2  # Approve 2-3 posts per run
```

### Expected Output

After each run:

- 2-3 posts have tags updated to `["approved"]`
- `approved_at` timestamp set
- `approved_by` set to "curation_actor"
- `text_content` updated with refined version
- Remaining potential posts stay in pool

## Stage 3: Posting Actor

### Purpose

Post approved content to Discord at semi-regular intervals with rate limiting.

### Narrative: `discord_poster.toml`

```toml
[narrative]
name = "discord_content_poster"
description = "Post approved content to Discord channel"

[toc]
order = ["fetch_approved", "check_interval", "select_next", "post", "record"]

# Fetch approved but not-yet-posted content
[tables.approved]
table_name = "content"
where_clause = "'approved' = ANY(tags) AND post_count = 0"
order_by = "priority DESC, approved_at ASC"
limit = 1
alias = "next_post"

[acts.fetch_approved]
[[acts.fetch_approved.input]]
type = "table"
reference = "tables.approved"

[acts.check_interval]
[[acts.check_interval.input]]
type = "bot_command"
platform = "database"
command = "post_history.get_last_post"
args = {
    actor_name = "posting_actor",
    platform = "discord"
}

[[acts.check_interval.input]]
type = "text"
content = """
Last post time: {{check_interval}}
Current time: {{now()}}
Minimum interval: 120 minutes

Calculate: Can we post now? (yes/no)
If no, provide wait time remaining.
"""

[acts.select_next]
[[acts.select_next.input]]
type = "text"
content = """
Interval check result: {{check_interval}}

Next approved post:
{{fetch_approved}}

Decision:
- If interval OK and post available: PROCEED
- If interval too recent: SKIP (report next allowed time)
- If no posts available: SKIP (report reason)

Output: {"action": "PROCEED" | "SKIP", "reason": "..."}
"""

[acts.post]
[[acts.post.input]]
type = "bot_command"
platform = "discord"
command = "message.send"
args = {
    channel_id = "${POSTING_CHANNEL_ID}",
    content = "{{extract_content_from(select_next)}}"
}
required = true

[acts.record]
[[acts.record.input]]
type = "bot_command"
platform = "database"
command = "content.mark_posted"
args = {
    content_id = "{{extract_id_from(select_next)}}",
    actor_name = "posting_actor",
    platform = "discord",
    channel_id = "${POSTING_CHANNEL_ID}",
    post_id = "{{extract_message_id_from(post)}}",
    posted_at = "NOW()"
}
```

### Actor Configuration: `posting_actor.toml`

```toml
[actor]
name = "Discord Posting Actor"
description = "Posts approved content to Discord at intervals"

[actor.schedule]
type = "Interval"
seconds = 7200  # Check every 2 hours

[actor.execution]
narrative_file = "narratives/discord/discord_poster.toml"
continue_on_error = true  # Skip if no posts ready

[actor.config]
min_post_interval_minutes = 120  # 2 hours between posts
max_posts_per_day = 8
time_window_start = "09:00"  # Only post 9am-9pm
time_window_end = "21:00"
timezone = "America/New_York"
randomize_schedule = true  # Add ±30min variance

[actor.platform]
type = "discord"
channel_id = "${POSTING_CHANNEL_ID}"
```

### Expected Output

When conditions met:

1. Message posted to Discord channel
2. `content.post_count` incremented
3. `content.last_posted_at` updated
4. Row inserted into `post_history` with:
   - `content_id`
   - `actor_name`: "posting_actor"
   - `platform`: "discord"
   - `channel_id`
   - `post_id` (Discord message ID)
   - `posted_at` timestamp

## Implementation Plan

### Phase 1: Database Preparation (Day 1) - ✅ COMPLETED

**Status**: Using schema inference instead of explicit table creation

**Tasks:**

1. ✅ Content table already exists (migration 2025-11-23-215323-0000)
2. ✅ Use schema inference for `potential_discord_posts` table:
   - No explicit migration needed
   - ContentGenerationProcessor automatically creates table from JSON output
   - Schema inferred on first successful generation run
   - Used by `target = "potential_discord_posts"` in narratives
3. 🔲 Add helper queries to `botticelli_database` (deferred - not needed for Phase 2):
   - `get_potential_posts()` - Filter by tags
   - `get_approved_posts()` - Filter by tags + post_count
   - `mark_as_approved()` - Update tags + metadata
   - `mark_as_posted()` - Update post_count + last_posted_at
   - mark_as_reviewed() - we need a way to ignore and remove old content
4. 🔲 Add bot commands for database operations (deferred - Phase 3):
   - `database.content.update_by_ids`
   - `database.content.mark_posted`
   - `database.post_history.get_last_post`

**Implementation approach:**

Instead of creating an explicit `potential_posts` table template, we rely on Botticelli's schema inference feature:
- Generation narratives output JSON with specific fields
- ContentGenerationProcessor parses JSON and infers column types
- Table `potential_discord_posts` created automatically on first run
- Subsequent generations insert into existing table

**Actual schema** (auto-created):
```sql
CREATE TABLE potential_discord_posts (
    text_content TEXT NOT NULL,
    source VARCHAR(255),
    tags TEXT[],
    content_type VARCHAR(50) NOT NULL,
    generated_at TIMESTAMP DEFAULT NOW() NOT NULL,
    source_narrative VARCHAR(255),
    source_act VARCHAR(255),
    generation_model VARCHAR(100),
    review_status VARCHAR(50),
    rating INTEGER
);
```

**Benefits of schema inference:**
- No migration file needed
- Schema automatically matches JSON output structure
- Flexible - adding fields is just adding to JSON output
- Self-documenting - schema reflects actual generation output

### Phase 2: Generation Actor (Day 2-3)

**Tasks:**

1. ✅ Create `generation_carousel.toml` narrative with 5 narratives + batch generator
2. ✅ Create `BOTTICELLI_CONTEXT.md` for LLM context
3. Test narratives:
   - Individual: `just narrate generation_carousel.feature_showcase` (1 post)
   - All five once: Run batch_generate with `iterations = 1` (5 posts)
   - **Batch of 50**: `just narrate generation_carousel.batch_generate` (50 posts)
4. Verify auto-storage to `potential_posts` table (50 rows after batch)
5. Create `generation_actor.toml` config
6. Test with `actor-server` in dry-run mode
7. Deploy and run for 2-3 days

**Files created:**

- ✅ `crates/botticelli_narrative/narratives/discord/generation_carousel.toml`
- ✅ `crates/botticelli_narrative/narratives/discord/BOTTICELLI_CONTEXT.md`
- `actors/generation_actor.toml` (to create)

**Success criteria:**

- 50 potential posts inserted per batch run (10 iterations × 5 narratives)
- Posts have diverse content across focus angles and iterations
- All posts under 1800 characters
- Posts use Discord markdown correctly
- JSON extraction and storage works correctly
- Tags properly differentiate post types
- Carousel mode successfully loops through all narratives

### Phase 3: Curation Actor (Day 4-5) - ✅ COMPLETED

**Status**: Implemented with workaround for Botticelli response handling issue

**Tasks:**

1. ✅ Create `curate_posts_final.toml` narrative
2. ✅ Implement analysis and scoring logic (LLM-based)
3. ✅ Test with real potential posts (10 posts from potential_discord_posts table)
4. 🔲 Create `curation_actor.toml` config (pending)
5. 🔲 Deploy and run for 2-3 days (pending)

**Files created:**

- ✅ `crates/botticelli_narrative/narratives/discord/curate_posts_final.toml` - Working curation narrative
- ✅ `crates/botticelli_narrative/narratives/discord/test_with_skip.toml` - Test demonstrating skip_content_generation behavior
- ✅ `crates/botticelli_narrative/narratives/discord/test_with_jinja.toml` - Test documenting response handling bug
- 🔲 `actors/curation_actor.toml` (to create)

**Implementation notes:**

Database schema uses `potential_discord_posts` table (created via schema inference, not explicit migration):
- Generated posts auto-stored by generation narratives
- Columns: text_content, source, tags, content_type, generated_at, source_narrative, source_act, generation_model, review_status, rating

Curation approach:
- Uses `skip_content_generation = true` to preserve LLM analysis text (not extract JSON)
- Queries up to 10 posts from `potential_discord_posts` table
- LLM scores each post on: Engagement, Clarity, Value, Tone, Polish (1-10 each, max 50 total)
- Recommends top 2-3 posts with reasoning
- Output: 663-character text analysis (not structured data)

**Bug discovered:**

Botticelli has a response handling issue where certain text patterns cause responses to be lost (0 characters) when `skip_content_generation = true` is set. This affects prompts with:
- Multi-line text with specific patterns
- Complex instructions that may confuse the response parser

**Workaround applied:**

Use concise, single-line prompts without complex formatting. Working example:
```toml
content = "Analyze the Discord posts from the table. Score each on Engagement, Clarity, Value, Tone, and Polish (1-10 each). Recommend the top 2 posts with highest scores."
```

**Important clarification:**

Botticelli does NOT have a template engine (no Jinja, Tera, Handlebars, etc.). The `{% if %}` and `{{ }}` syntax seen in some TOML files is NOT processed by Botticelli - it's literal text sent to the LLM as natural language instructions. The LLM is sophisticated enough to understand these pseudo-template patterns as instructions.

**Success criteria:**

- ✅ Analyzes 10 potential posts per run
- ✅ Scores posts on multiple criteria
- ✅ Recommends top 2-3 posts with reasoning
- ✅ Response successfully captured (663 chars)
- 🔲 Approved posts moved to content table (manual workflow needed)
- 🔲 Maintains diversity in selected posts (needs verification)

**Next steps:**

1. Create manual workflow or bot command to copy approved posts to `content` table
2. Add tags update: `["approved", "curated"]` for selected posts
3. Create `curation_actor.toml` for automated runs
4. Test end-to-end pipeline

### Phase 4: Posting Actor (Day 6-7)

**Tasks:**

1. Create `discord_poster.toml` narrative
2. Implement interval checking
3. Add time window constraints
4. Test posting to test channel
5. Create `posting_actor.toml` config
6. Deploy to production channel

**Files to create:**

- `crates/botticelli_narrative/narratives/discord/discord_poster.toml`
- `actors/posting_actor.toml`

**Success criteria:**

- Posts only approved content
- Respects minimum interval (2 hours)
- Stays within time window
- Records post history correctly
- Handles "no posts available" gracefully

### Phase 5: Integration & Monitoring (Day 8-10)

**Tasks:**

1. Run all three actors simultaneously
2. Monitor pipeline health:
   - Generation rate (posts/day)
   - Approval rate (approved/potential)
   - Posting frequency
3. Add observability:
   - Dashboard for content table stats
   - Alerts for pipeline stalls
4. Tune parameters:
   - Adjust intervals
   - Refine scoring criteria
   - Optimize prompts

**Monitoring queries:**

```sql
-- Pipeline health
SELECT
    COALESCE(ARRAY_TO_STRING(tags, ','), 'untagged') as stage,
    COUNT(*) as count
FROM content
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY tags;

-- Posting frequency
SELECT
    DATE_TRUNC('day', posted_at) as day,
    COUNT(*) as posts
FROM post_history
WHERE actor_name = 'posting_actor'
GROUP BY day
ORDER BY day DESC
LIMIT 7;

-- Approval rate
SELECT
    COUNT(*) FILTER (WHERE 'approved' = ANY(tags))::FLOAT /
    NULLIF(COUNT(*), 0) * 100 as approval_rate_pct
FROM content
WHERE created_at > NOW() - INTERVAL '7 days'
  AND 'potential' = ANY(tags);
```

## Configuration Parameters

### Tunable Values

**Generation Actor:**

- Carousel iterations: 10 (adjust for batch size: 10 × 5 = 50 posts)
- Individual narratives: 5 (feature, usecase, tutorial, community, problem_solution)
- Temperature: 0.8 (higher = more creative)
- Model: **gemini-2.5-flash-lite** (highest daily request limit for batch processing)
- Max tokens: 600 (sufficient for Discord posts)

**Curation Actor:**

- Review batch size: 10 posts
- Selection count: 2-3 posts per run
- Minimum score threshold: 35/50 (adjust for quality bar)
- Temperature: 0.3 (lower = more consistent)
- Model: gemini-2.0-flash-thinking-exp (highest quality)

**Posting Actor:**

- Check interval: 2 hours
- Min post interval: 2 hours (rate limit)
- Max posts per day: 8
- Time window: 9am-9pm
- Randomization: ±30 minutes

### Environment Variables

```bash
# Required
DATABASE_URL="postgresql://user:pass@localhost/botticelli"
DISCORD_TOKEN="your-discord-bot-token"
GEMINI_API_KEY="your-gemini-api-key"

# Actor configuration
POSTING_CHANNEL_ID="1234567890"  # Production channel
TEST_CHANNEL_ID="0987654321"     # Testing channel

# Observability
RUST_LOG="info,botticelli_actor=debug"
```

## Safety Mechanisms

### Rate Limiting

- Minimum 2-hour interval between posts
- Maximum 8 posts per 24-hour period
- Time window constraints (9am-9pm)

### Quality Control

- Two-stage curation (analysis + selection)
- Score threshold for approval
- Manual override capability via tags

### Failure Handling

- `continue_on_error = true` for posting actor
- Retry logic for transient failures
- Dead letter queue for persistent failures

### Rollback Plan

- Manual pause: Set `is_paused = true` in `actor_server_state`
- Emergency stop: `pkill -f actor-server`
- Content removal: `UPDATE content SET tags = ARRAY['removed'] WHERE id = ?`

## Success Metrics

### Pipeline Health

- **Generation**: 50 potential posts/day (via carousel batch mode)
- **Curation**: 20-30% approval rate → 10-15 approved posts/day
- **Posting**: 4-8 posts/day to Discord (paced distribution)

### Quality Metrics

- Engagement: Reactions per post (track via Discord API)
- Retention: Low delete/edit rate
- Diversity: Tags distribution across topics

### System Metrics

- Actor uptime: >99%
- Execution success rate: >95%
- Average latency: <5 minutes from approval to post

## Future Enhancements

### Phase 2 Features

1. **Engagement feedback loop** - Use Discord reactions to refine scoring
2. **A/B testing** - Post variants and measure engagement
3. **Seasonal themes** - Adjust topics based on calendar
4. **Multi-channel support** - Different content for different channels

### Phase 3 Features

1. **Interactive curation** - Web UI for manual approval
2. **Content calendar** - Schedule posts for specific dates
3. **Thread support** - Multi-message narratives
4. **Media generation** - Include images via DALL-E/Midjourney

## References

- [Actor Architecture](../../ACTOR_ARCHITECTURE.md)
- [Actor Server Observability](../../ACTOR_SERVER_OBSERVABILITY.md)
- [Discord Schema](../../DISCORD_SCHEMA.md)
- [Narrative TOML Spec](../../NARRATIVE_TOML_SPEC.md)
- [Botticelli Context](./BOTTICELLI_CONTEXT.md)
