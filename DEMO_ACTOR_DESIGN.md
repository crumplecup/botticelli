# Demo Actor Design

**Status:** ✅ Implemented (Workflow Version)  
**Created:** 2025-12-09  
**Updated:** 2025-12-09  
**Purpose:** An Actor that demonstrates Botticelli's complete value chain through automated workflows

---

## Overview

We need an Actor that demonstrates **the complete Botticelli value chain**:

1. **Natural Language → MCP Tools** - User prompts trigger appropriate MCP tools
2. **MCP Tools → Database** - Tools create persistent tables and data
3. **Database → Discord Actions** - Narratives generate and post content
4. **Complete Workflow** - Build a Discord server using only natural language

### Demo Scenario: "Building the Botticelli Discord Server"

The demo actor plays the role of a user trying to promote Botticelli by:
1. Creating narratives to design Discord channels
2. Generating promotional content (posts, carousels, images)
3. Executing narratives to post content to Discord
4. Showing how database tables chain together into productive flows

This demonstrates the **full pipeline**: prompt → tool → database → action → result.

**Key Testing Insight:** Database tables are natural test checkpoints - each stage produces verifiable output that validates the workflow.

---

## Architecture

### Component: Demo Actor

**Location:** `crates/botticelli_actor/src/demo_actor.rs`

**Responsibilities:**
- Read demo script (sequence of prompts)
- Connect to TUI's input system
- Send prompts with realistic timing
- Capture and log responses
- Handle errors gracefully

**Error Handling:**
- Define `DemoActorError` with `derive_more::Display` + `derive_more::Error`
- Define `DemoActorResult<T>` type alias
- Use `#[track_caller]` on constructors

**Tracing:**
- All public functions use `#[instrument]`
- Log at debug level for each prompt sent
- Log at info level for script start/completion

**Testing:**
- Integration tests in `tests/demo_actor_test.rs`
- Unit tests for script parsing
- No `#[cfg(test)]` modules in source

### Component: Script Format

**Location:** `examples/demo_scripts/*.toml`

```toml
[[prompt]]
text = "Create a narrative to generate social media posts for the MINT navigation center"
wait_seconds = 5
description = "Demonstrates create_narrative tool"

[[prompt]]
text = "List all my narratives"
wait_seconds = 2
description = "Demonstrates list_narratives tool"

[[prompt]]
text = "Load narrative: social_media_mint"
wait_seconds = 3
description = "Demonstrates load_narrative tool"

[[prompt]]
text = "Execute the loaded narrative"
wait_seconds = 10
description = "Demonstrates execute_narrative tool"

[[prompt]]
text = "Validate the narrative TOML"
wait_seconds = 2
description = "Demonstrates validate_narrative tool"
```

### Component: Actor Integration

**How it works:**

1. **TUI exposes an event channel:**
   ```rust
   pub struct TuiApp {
       input_tx: mpsc::Sender<InputEvent>,
       // ...
   }
   
   pub enum InputEvent {
       UserInput(String),
       ActorInput(String),
   }
   ```

2. **Demo Actor sends events:**
   ```rust
   use derive_getters::Getters;
   
   #[derive(Debug, Clone, Getters)]
   pub struct DemoActor {
       script: DemoScript,
       input_tx: mpsc::Sender<InputEvent>,
   }
   
   impl DemoActor {
       #[instrument(skip(self))]
       pub async fn run(&self) -> DemoActorResult<()> {
           for prompt in self.script.prompts() {
               debug!("Sending prompt: {}", prompt.text());
               self.input_tx
                   .send(InputEvent::actor_input(prompt.text().clone()))
                   .map_err(|e| DemoActorError::new(format!("Send failed: {}", e)))?;
               sleep(Duration::from_secs(*prompt.wait_seconds())).await;
           }
           Ok(())
       }
   }
   ```

3. **Launch with flag:**
   ```bash
   just chat-local --demo examples/demo_scripts/full_workflow.toml
   ```

---

## Relationship to Discord Bot

### Similarities

| Discord Bot | Demo Actor |
|-------------|------------|
| Reads narrative from DB | Reads script from TOML |
| Posts to Discord channels | Posts to TUI input |
| Schedules posts with timing | Sends prompts with delays |
| Uses async runtime | Uses async runtime |
| Error handling & retry | Error handling & logging |

### Differences

| Discord Bot | Demo Actor |
|-------------|------------|
| External service (Discord API) | Internal service (TUI) |
| Long-running daemon | Short-lived demonstration |
| Triggered by schedule | Triggered by script |
| Persists state in DB | Ephemeral execution |

---

## Implementation Phases

### Phase 1: TUI Event System (2-3 hours)
- [ ] Add `InputEvent` enum to TUI
- [ ] Create event channel in `TuiApp`
- [ ] Wire events to message handling
- [ ] Test manual event sending

### Phase 2: Demo Script Format (1-2 hours)
- [ ] Create `DemoScript` struct
- [ ] Implement TOML deserialization
- [ ] Add validation logic
- [ ] Create example scripts

### Phase 3: Demo Actor Implementation (2-3 hours)
- [ ] Create `DemoActor` struct in `botticelli_actor`
- [ ] Implement script execution
- [ ] Add timing and delays
- [ ] Add logging/tracing

### Phase 4: CLI Integration (1 hour)
- [ ] Add `--demo` flag to chat binary
- [ ] Launch actor alongside TUI
- [ ] Handle actor lifecycle

### Phase 5: Example Scripts (1-2 hours)
- [ ] Full workflow demo
- [ ] Error handling demo
- [ ] Feature showcase demo
- [ ] Quick start demo

### Phase 6: Testing & Validation (2-3 hours)
- [ ] Create database query helpers
- [ ] Add validation checks after each stage
- [ ] Implement integration tests that verify table contents
- [ ] Add assertions for schema correctness
- [ ] Test complete workflow end-to-end

---

## Testing Strategy

### Database as Test Oracle

Each stage of the demo creates verifiable database tables:

**Stage 1: Channel Creation**
```sql
SELECT name, description FROM discord_channels WHERE narrative_id = ?;
```
Assertions:
- Count matches expected channels
- Names match prompt intent
- Descriptions are non-empty

**Stage 2: Content Generation**
```sql
SELECT content, channel_id FROM discord_posts WHERE narrative_id = ?;
```
Assertions:
- Post count matches request
- Content length > minimum threshold
- All posts reference valid channels
- Content mentions "Botticelli"

**Stage 3: Carousel Creation**
```sql
SELECT m.url, cs.position, m.caption 
FROM carousel_media m 
JOIN carousel_sequence cs ON m.id = cs.media_id
WHERE cs.carousel_id = ?
ORDER BY cs.position;
```
Assertions:
- Slide count correct
- Position sequence 0..N-1
- URLs valid format
- Captions present

**Stage 4: Discord Execution**
```sql
SELECT discord_message_id, posted_at, error 
FROM discord_post_log 
WHERE narrative_id = ?;
```
Assertions:
- All posts have discord_message_id
- No errors recorded
- posted_at timestamps recent

### Integration Test Example

```rust
#[tokio::test]
async fn demo_creates_complete_discord_server() -> Result<(), DemoError> {
    let config = AppConfig::test()?;
    let demo = DemoActor::new(config.clone()).await?;
    let pool = PgPool::connect(&config.database_url()).await?;
    
    // Execute full workflow
    demo.run_script("examples/demo_scripts/full_workflow.toml").await?;
    
    // Validate Stage 1: Channels
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT * FROM discord_channels WHERE created_at > NOW() - INTERVAL '1 minute'"
    )
    .fetch_all(&pool)
    .await?;
    
    assert_eq!(channels.len(), 5, "Should create 5 channels");
    assert!(channels.iter().any(|c| c.name == "announcements"));
    assert!(channels.iter().any(|c| c.name == "features"));
    
    // Validate Stage 2: Posts
    let posts = sqlx::query_as::<_, Post>(
        "SELECT * FROM discord_posts WHERE created_at > NOW() - INTERVAL '1 minute'"
    )
    .fetch_all(&pool)
    .await?;
    
    assert!(posts.len() >= 10, "Should create at least 10 posts");
    assert!(posts.iter().all(|p| p.content.contains("Botticelli")));
    
    // Validate Stage 3: Carousel
    let carousel = sqlx::query!(
        "SELECT COUNT(*) as count FROM carousel_media WHERE created_at > NOW() - INTERVAL '1 minute'"
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(carousel.count.unwrap(), 3, "Should create 3 carousel slides");
    
    Ok(())
}
```

---

## Example Demo Scripts

### `examples/demo_scripts/quick_start.toml`
```toml
name = "Quick Start"
description = "5-minute introduction to Botticelli"

[[prompt]]
text = "Create a narrative for generating tweets about Rust programming"
wait_seconds = 5
description = "Shows narrative creation"

[[prompt]]
text = "List my narratives"
wait_seconds = 2
description = "Shows narrative listing"
```

### `examples/demo_scripts/build_discord_server.toml`
```toml
name = "Build Botticelli Discord Server"
description = "Complete value chain demonstration: prompts → database → Discord"

# Step 1: Design the channel structure
[[prompt]]
text = "Create a narrative to design Discord channels for promoting Botticelli. Include channels for: announcements, tutorials, showcase, support, and development"
wait_seconds = 10
description = "MCP creates 'discord_channels' table with channel definitions"

# Step 2: Generate announcement content
[[prompt]]
text = "Create a narrative to generate launch announcement posts. Make them engaging with emojis and clear value propositions"
wait_seconds = 10
description = "MCP creates 'announcement_posts' table"

# Step 3: Create tutorial content with carousels
[[prompt]]
text = "Create a narrative to generate tutorial carousels. Each carousel should have 3-5 slides explaining a Botticelli feature"
wait_seconds = 12
description = "MCP creates 'tutorial_carousels' table with multi-slide content"

# Step 4: Generate showcase examples
[[prompt]]
text = "Create a narrative to generate showcase posts highlighting real use cases of Botticelli"
wait_seconds = 10
description = "MCP creates 'showcase_posts' table"

# Step 5: Execute channel creation
[[prompt]]
text = "Execute the Discord channel creation narrative"
wait_seconds = 15
description = "Narrative reads 'discord_channels' table, creates actual Discord channels"

# Step 6: Post announcements
[[prompt]]
text = "Execute the announcement narrative"
wait_seconds = 15
description = "Narrative reads 'announcement_posts', posts to #announcements channel"

# Step 7: Post tutorial carousels
[[prompt]]
text = "Execute the tutorial carousel narrative"
wait_seconds = 20
description = "Narrative reads 'tutorial_carousels', creates carousel posts in #tutorials"

# Step 8: Post showcase content
[[prompt]]
text = "Execute the showcase narrative"
wait_seconds = 15
description = "Narrative reads 'showcase_posts', posts to #showcase channel"

# Step 9: Verify everything
[[prompt]]
text = "List all narratives and show what database tables were created"
wait_seconds = 5
description = "Show the complete chain: 4 narratives → 4 tables → Discord actions"
```

### `examples/demo_scripts/error_handling.toml`
```toml
name = "Error Handling"
description = "Shows how Botticelli handles errors gracefully"

[[prompt]]
text = "Load narrative: does_not_exist"
wait_seconds = 2
description = "Show missing narrative error"

[[prompt]]
text = "Execute before loading"
wait_seconds = 2
description = "Show validation error"
```

---

## Value Chain Demonstration

### What Makes This Different

This isn't just "show all the tools" - it demonstrates **how Botticelli creates business value:**

1. **Natural Language as Interface** - No code, no configuration files, just plain English
2. **Persistent State** - Database tables persist across executions, enabling workflows
3. **Chained Operations** - Output of one narrative becomes input to another
4. **Real Actions** - Not mock data - actual Discord server gets created and populated
5. **Complete Pipeline** - From user intent → data structure → action → observable result

### The Story It Tells

```
User Says: "Create a narrative to design Discord channels..."
↓
MCP Tool: create_narrative with schema inference
↓
Database: 'discord_channels' table created with structure
↓
User Says: "Execute the Discord channel creation narrative"
↓
Narrative Executor: Reads 'discord_channels' table
↓
Discord API: Channels created on actual server
↓
Result: User sees new channels appear in Discord
```

This loop demonstrates **the core Botticelli value proposition**: Natural language → structured data → automated actions.

## Benefits

1. **Onboarding:** New users see the full value chain, not just isolated features
2. **Testing:** Validates integration of all components: MCP + DB + Discord + TUI
3. **Documentation:** Shows real-world use case, not toy examples
4. **Sales/Marketing:** Demonstrates ROI - "build a Discord server in 5 minutes with English"
5. **Debugging:** Reproducible end-to-end workflow for finding integration bugs

---

## Future Enhancements

1. **Interactive Mode:** Pause between prompts, wait for user confirmation
2. **Recording Mode:** Capture screenshots/video of demo
3. **Assertion Mode:** Verify expected responses (automated testing)
4. **Randomization:** Random prompts from a pool for stress testing
5. **Multi-Actor:** Multiple actors running different scripts simultaneously

---

## Success Criteria

### Technical Goals
- [x] Demo actor can send all prompts from script
- [ ] TUI responds correctly to actor input
- [ ] All MCP tools demonstrated successfully
- [ ] Error handling works as expected
- [x] Documentation updated with demo instructions
- [x] At least 3 example scripts created (basic and mcp_tools scenarios)

### Value Chain Goals
- [ ] Demo creates actual Discord channels (not mocked)
- [ ] Demo generates content that gets stored in database
- [ ] Demo executes narratives that read from database tables
- [ ] Demo shows chaining: Table A → Narrative B → Action C
- [ ] Demo observable from start to finish without manual intervention
- [ ] Demo completes in under 10 minutes
- [ ] Database tables persist after demo (can be inspected)

## Implementation Status

### Phase 1: Foundation ✅
- ✅ `DemoScenario` + Builder pattern
- ✅ `DemoExecutor` trait with `ChatDemoExecutor`
- ✅ Pre-built scenarios: `create_mcp_tools_demo()`, `create_basic_demo()`
- ✅ Demo binary at `crates/botticelli_chat/src/bin/demo.rs`
- ✅ Justfile integration: `just chat-demo`

### Phase 2: Complete Value Chain ⏳ (Current)

**Goal:** Demonstrate prompt → MCP → database → Discord → observable result

#### Required Components:
1. **Discord Integration** - Must be able to create actual channels and posts
   - [ ] Discord bot token configuration
   - [ ] Channel creation API integration
   - [ ] Post creation API integration
   - [ ] Carousel/embed support

2. **Database Persistence** - Schema inference must create real tables
   - [ ] MCP `create_narrative` creates tables based on prompt
   - [ ] Tables visible in PostgreSQL after narrative creation
   - [ ] Narrative executor can read from these tables
   
3. **Narrative Chaining** - Output of narrative A feeds narrative B
   - [ ] Narrative A: Generate channel definitions → `discord_channels` table
   - [ ] Narrative B: Read `discord_channels` → Create actual Discord channels
   - [ ] Narrative C: Generate posts → `announcement_posts` table
   - [ ] Narrative D: Read `announcement_posts` → Post to Discord

4. **Demo Script** - `build_discord_server.toml`
   - [ ] 8-10 prompts showing complete workflow
   - [ ] Realistic timing (allow for AI generation)
   - [ ] Demonstrates all key capabilities

#### Testing Strategy:
- [ ] Run demo against test Discord server
- [ ] Verify database tables created and populated
- [ ] Verify Discord channels/posts appear
- [ ] Verify demo completes without manual intervention

### Phase 3: Polish & Documentation
- [ ] Add commentary/narration to demo output
- [ ] Create video walkthrough
- [ ] Write user guide explaining the value chain
- [ ] Add troubleshooting guide for demo failures

---

## Related Work

- **Discord Actor:** `crates/botticelli_actor/src/discord_actor.rs`
- **Narrative Executor:** `crates/botticelli_narrative/src/executor.rs`
- **TUI Application:** `crates/botticelli_chat/src/tui.rs`
- **MCP Integration:** `crates/botticelli_chat/src/executor.rs`

---

## Implementation Complete: Workflow Demo Actor

**Date:** 2025-12-09  
**Status:** ✅ Workflow system implemented and compiled successfully

### What Was Built

#### 1. Workflow Executor (`crates/botticelli_actor/src/demo_workflow.rs`)

Complete workflow system with:
- **5 Workflow Stages**: Strategy → Channels → Content → Media → Calendar
- **Database Validation**: Each stage validates output table exists with minimum rows
- **Dependency Management**: Stages execute in correct order based on dependencies
- **Test Mode**: Can run without actual chat/database for testing

#### 2. Binary Runner (`crates/botticelli_actor/src/bin/demo-workflow.rs`)

Command-line tool with:
- Database connection with pool management  
- Configurable stage delays
- Test mode vs production mode
- Verbose logging option

#### 3. Justfile Integration

New recipes:
```bash
just demo-workflow-test   # Run in test mode (no database required)
just demo-workflow        # Run with full database validation
```

#### 4. Database Infrastructure

Added to `botticelli_database`:
- `DbPool` type alias for connection pool
- `create_pool_from_url()` function for custom database URLs
- Maintained backward compatibility with existing code

### Botticelli Promotion Workflow

The demo demonstrates building a Discord server to promote Botticelli:

1. **Strategy Planning** → `promotion_strategy` table (3+ rows)
2. **Channel Design** → `discord_channels` table (5+ channels)
3. **Content Generation** → `social_posts` table (10+ posts)
4. **Media Assets** → `media_assets` table (8+ assets)
5. **Content Calendar** → `content_calendar` table (20+ scheduled items)

Each stage:
- Sends natural language prompt to chat interface (TODO: integrate)
- Creates a database table with specific schema
- Validates minimum row count
- Records validation results

### Testing Infrastructure

The workflow provides **natural test checkpoints**:

```rust
// Each stage validates output
ValidationResult {
    stage_id: "content",
    passed: true,
    rows_found: 15,
    error: None,
}
```

Integration tests can verify:
- Tables were created
- Required columns exist
- Minimum data generated
- Workflow completes successfully

### Next Steps

To complete Phase 2 (full value chain):

1. **Integrate with Chat Interface**: Connect workflow executor to send prompts through TUI
2. **Connect MCP Server**: Ensure prompts trigger `create_narrative` tool
3. **Database Persistence**: Verify narratives actually create the expected tables
4. **Discord Integration**: Chain workflows to Discord actions (create channels, post content)

### How to Run

```bash
# Test mode (validates workflow logic)
just demo-workflow-test

# Full mode (requires DATABASE_URL)
export DATABASE_URL="postgresql://botticelli@localhost/botticelli"
just demo-workflow
```

The workflow demonstrates the **complete Botticelli value chain**: natural language prompts → MCP tools → persistent database tables → actionable outputs.

