# TUI Troubleshooting Notes

## Current Issue: `just tui-demo` Failing

### The Problem

The `just tui-demo` recipe is failing because of a mismatch between expected and actual table names:

**Expected:** The justfile commands were written expecting table names like `guilds_gen_001`, `channel_posts_001`, `users_gen_001` (with a pattern to grep for)

**Actual:** The narratives create tables named after the `name` field in the `[narration]` section: `potential_guilds`, `potential_posts`, `potential_users`

**What happened:**
1. `example-guilds` ran successfully, creating a table named `potential_guilds`
2. The `tui-demo` recipe tried to find a table matching `guilds_gen%` pattern using psql
3. psql either failed (auth issues?) or found no matching table
4. The variable `$$TABLE` was empty or invalid
5. Line 337 (`cargo run --all-features -- tui "$$TABLE"`) failed with exit code 1

### Root Cause Analysis

The fundamental issue is a **lack of discoverability** of generated content tables:

1. **No standard output format** - `boticelli run` doesn't reliably output the table name in a machine-parseable way
2. **No query interface** - No CLI command to list or query what tables were created
3. **Fragile justfile patterns** - Relying on PostgreSQL pattern matching (`guilds_gen%`) is brittle and assumes naming conventions
4. **Workflow disconnect** - The generation step (`just example-guilds`) and consumption step (`just tui-demo`) are decoupled with no data handoff

### Solution: Fix the Underlying Issue Properly

We will implement **metadata tracking from the start** to build a robust, maintainable foundation.

## Comprehensive Fix Plan

The core insight: instead of bolting on discovery mechanisms to an untracked system, we should track content generation as a first-class concern. This provides a clean foundation for all downstream features.

### Phase 1: Metadata Tracking Infrastructure (Foundation)

**Goal:** Create a tracking table and integrate it into the content generation pipeline.

**Step 1: Database Schema**

Create migration `migrations/YYYYMMDDHHMMSS_create_content_generations/up.sql`:

```sql
-- Track all content generation attempts
CREATE TABLE content_generations (
    id SERIAL PRIMARY KEY,
    table_name TEXT NOT NULL,
    narrative_file TEXT NOT NULL,
    narrative_name TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    row_count INTEGER,
    generation_duration_ms INTEGER,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'failed')),
    error_message TEXT,
    created_by TEXT, -- optional: track which user/system generated
    
    -- Indexes for common queries
    CONSTRAINT content_generations_table_name_key UNIQUE (table_name)
);

CREATE INDEX idx_content_generations_generated_at ON content_generations(generated_at DESC);
CREATE INDEX idx_content_generations_status ON content_generations(status);
CREATE INDEX idx_content_generations_narrative_file ON content_generations(narrative_file);
```

**Step 2: Diesel Models**

Create `src/database/content_generation_models.rs`:

```rust
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = content_generations)]
pub struct ContentGeneration {
    pub id: i32,
    pub table_name: String,
    pub narrative_file: String,
    pub narrative_name: String,
    pub generated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub row_count: Option<i32>,
    pub generation_duration_ms: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_generations)]
pub struct NewContentGeneration {
    pub table_name: String,
    pub narrative_file: String,
    pub narrative_name: String,
    pub status: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = content_generations)]
pub struct UpdateContentGeneration {
    pub completed_at: Option<DateTime<Utc>>,
    pub row_count: Option<i32>,
    pub generation_duration_ms: Option<i32>,
    pub status: Option<String>,
    pub error_message: Option<String>,
}
```

**Step 3: Repository Layer**

Create `src/database/content_generation_repository.rs`:

```rust
use crate::{ContentGeneration, NewContentGeneration, UpdateContentGeneration, DatabaseResult};
use diesel::prelude::*;

pub trait ContentGenerationRepository {
    /// Record the start of a content generation
    fn start_generation(&self, new_gen: NewContentGeneration) -> DatabaseResult<ContentGeneration>;
    
    /// Update generation status on completion
    fn complete_generation(&self, table_name: &str, update: UpdateContentGeneration) -> DatabaseResult<ContentGeneration>;
    
    /// Get the most recently completed generation
    fn get_last_successful(&self) -> DatabaseResult<Option<ContentGeneration>>;
    
    /// List all generations with optional filtering
    fn list_generations(&self, status: Option<String>, limit: i64) -> DatabaseResult<Vec<ContentGeneration>>;
    
    /// Get specific generation by table name
    fn get_by_table_name(&self, table_name: &str) -> DatabaseResult<Option<ContentGeneration>>;
    
    /// Delete generation metadata (cleanup)
    fn delete_generation(&self, table_name: &str) -> DatabaseResult<()>;
}

pub struct PostgresContentGenerationRepository {
    pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl ContentGenerationRepository for PostgresContentGenerationRepository {
    fn start_generation(&self, new_gen: NewContentGeneration) -> DatabaseResult<ContentGeneration> {
        use crate::database::schema::content_generations;
        let mut conn = self.pool.get()?;
        
        diesel::insert_into(content_generations::table)
            .values(&new_gen)
            .get_result(&mut conn)
            .map_err(Into::into)
    }
    
    fn complete_generation(&self, table_name: &str, update: UpdateContentGeneration) -> DatabaseResult<ContentGeneration> {
        use crate::database::schema::content_generations::dsl;
        let mut conn = self.pool.get()?;
        
        diesel::update(dsl::content_generations.filter(dsl::table_name.eq(table_name)))
            .set(&update)
            .get_result(&mut conn)
            .map_err(Into::into)
    }
    
    fn get_last_successful(&self) -> DatabaseResult<Option<ContentGeneration>> {
        use crate::database::schema::content_generations::dsl;
        let mut conn = self.pool.get()?;
        
        dsl::content_generations
            .filter(dsl::status.eq("success"))
            .order(dsl::generated_at.desc())
            .first(&mut conn)
            .optional()
            .map_err(Into::into)
    }
    
    // ... implement remaining methods
}
```

**Step 4: Integrate into Executor**

Update narrative executor to track generation:

```rust
// In executor, before starting generation
let new_gen = NewContentGeneration {
    table_name: narration.name.clone(),
    narrative_file: narrative_path.to_string_lossy().to_string(),
    narrative_name: narration.name.clone(),
    status: "running".to_string(),
    created_by: None, // or get from env/config
};

let generation = content_repo.start_generation(new_gen)?;
let start_time = Instant::now();

// Run the generation
match execute_narration(&narration, &pool) {
    Ok(row_count) => {
        let duration_ms = start_time.elapsed().as_millis() as i32;
        content_repo.complete_generation(
            &narration.name,
            UpdateContentGeneration {
                completed_at: Some(Utc::now()),
                row_count: Some(row_count),
                generation_duration_ms: Some(duration_ms),
                status: Some("success".to_string()),
                error_message: None,
            },
        )?;
    }
    Err(e) => {
        content_repo.complete_generation(
            &narration.name,
            UpdateContentGeneration {
                completed_at: Some(Utc::now()),
                row_count: None,
                generation_duration_ms: Some(start_time.elapsed().as_millis() as i32),
                status: Some("failed".to_string()),
                error_message: Some(e.to_string()),
            },
        )?;
        return Err(e);
    }
}
```

**Benefits of this approach:**
- Single source of truth for generation metadata
- Atomic tracking (database transactions ensure consistency)
- Historical record of all generations
- Foundation for future features (cleanup, analytics, retry logic)
- No parsing of stdout or fragile file system scanning

### Phase 2: Content Query CLI (Builds on Phase 1)

**Goal:** Add user-facing commands to query and manage generated content.

**Implementation:**

Create `src/cli/content.rs`:

```rust
use clap::Subcommand;
use crate::{ContentGenerationRepository, DatabaseResult};

#[derive(Subcommand)]
pub enum ContentCommands {
    /// List all generated content tables
    List {
        #[arg(long)]
        status: Option<String>,
        
        #[arg(long, default_value = "20")]
        limit: i64,
        
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    
    /// Get the most recently generated table
    Last {
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    
    /// Show details about a specific table
    Info {
        table_name: String,
        
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    
    /// Clean up old generated tables
    Clean {
        #[arg(long)]
        older_than_days: Option<i64>,
        
        #[arg(long)]
        yes: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Human,
    Json,
    TableNameOnly,
}

pub fn handle_content_command(
    cmd: ContentCommands,
    repo: &dyn ContentGenerationRepository,
) -> DatabaseResult<()> {
    match cmd {
        ContentCommands::Last { format } => {
            let gen = repo.get_last_successful()?;
            match gen {
                Some(g) => match format {
                    OutputFormat::TableNameOnly => println!("{}", g.table_name),
                    OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&g)?),
                    OutputFormat::Human => {
                        println!("Last generated table: {}", g.table_name);
                        println!("  Narrative: {}", g.narrative_name);
                        println!("  File: {}", g.narrative_file);
                        println!("  Generated: {}", g.generated_at);
                        if let Some(rows) = g.row_count {
                            println!("  Rows: {}", rows);
                        }
                        if let Some(ms) = g.generation_duration_ms {
                            println!("  Duration: {}ms", ms);
                        }
                    }
                },
                None => {
                    eprintln!("No successful generations found");
                    std::process::exit(1);
                }
            }
        }
        ContentCommands::List { status, limit, format } => {
            let generations = repo.list_generations(status, limit)?;
            match format {
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&generations)?),
                OutputFormat::Human => {
                    println!("{:<20} {:<15} {:<10} {:<20}", "Table", "Status", "Rows", "Generated");
                    println!("{:-<70}", "");
                    for gen in generations {
                        println!(
                            "{:<20} {:<15} {:<10} {:<20}",
                            gen.table_name,
                            gen.status,
                            gen.row_count.map(|r| r.to_string()).unwrap_or_else(|| "-".to_string()),
                            gen.generated_at.format("%Y-%m-%d %H:%M")
                        );
                    }
                }
                OutputFormat::TableNameOnly => {
                    for gen in generations {
                        println!("{}", gen.table_name);
                    }
                }
            }
        }
        // ... implement Info and Clean
    }
    Ok(())
}
```

Update `src/bin/boticelli.rs` to wire in the new command:

```rust
#[derive(Subcommand)]
enum Commands {
    Run { ... },
    Tui { ... },
    #[command(subcommand)]
    Content(ContentCommands),
}

// In main()
Commands::Content(content_cmd) => {
    let repo = PostgresContentGenerationRepository::new(pool);
    handle_content_command(content_cmd, &repo)?;
}
```

**Usage examples:**
```bash
# Get last table for scripting
TABLE=$(boticelli content last --format=table-name-only)
boticelli tui "$TABLE"

# List all generations
boticelli content list

# List only successful ones
boticelli content list --status=success

# Get JSON for external tools
boticelli content list --format=json | jq '.[] | .table_name'
```

### Phase 3: Update Justfile (Simple Now)

**Goal:** Update justfile to use the new content query system.

```just
# Generate guilds and open TUI
tui-demo:
    @echo "Generating sample guilds..."
    cargo run --all-features -- run narratives/generate_guilds.toml
    @echo "Opening TUI..."
    TABLE=$$(cargo run --all-features -- content last --format=table-name-only) && \
    cargo run --all-features -- tui "$$TABLE"

# Generate all demo content
demo-content:
    cargo run --all-features -- run narratives/generate_guilds.toml
    cargo run --all-features -- run narratives/generate_posts.toml
    cargo run --all-features -- run narratives/generate_users.toml

# View any generated table
view-content TABLE:
    cargo run --all-features -- tui {{TABLE}}

# Clean up old generated content
clean-content:
    cargo run --all-features -- content clean --older-than-days=7

# List all generated content
list-content:
    cargo run --all-features -- content list
```

**Benefits:**
- No psql queries in justfile
- Clear, explicit commands
- Works with any table name
- Easy to extend

### Phase 4: Enhanced Run Output (Optional Polish)

**Goal:** Make `boticelli run` output more user-friendly and informative.

Add friendly output at the end of generation:

```rust
// After successful generation
println!("✓ Generated {} rows in table '{}'", row_count, table_name);
println!("  Duration: {}ms", duration_ms);
println!();
println!("View with: boticelli tui {}", table_name);
println!("Or list all: boticelli content list");
```

**Optional:** Add `--quiet` flag for scripting:
```rust
#[derive(Parser)]
struct RunArgs {
    narrative_file: PathBuf,
    
    #[arg(long)]
    quiet: bool,  // Only output errors
}
```

## Implementation Order

1. **Sprint 1: Foundation (Database + Models)**
   - Create migration for `content_generations` table
   - Create Diesel models (ContentGeneration, NewContentGeneration, UpdateContentGeneration)
   - Create repository trait and PostgreSQL implementation
   - Add unit tests for repository methods
   - Run `diesel migration run` and verify schema
   - **Commit: "Add content generation tracking infrastructure"** ✓

2. **Sprint 2: Executor Integration**
   - Update narrative executor to call repository on start/complete
   - Handle success and failure cases with proper metadata
   - Add integration test: generate content and verify tracking record exists
   - Verify tracking works end-to-end with test narrative
   - **Commit: "Integrate content tracking into narrative executor"** ✓

3. **Sprint 3: Content CLI Commands**
   - Create `src/cli/content.rs` with ContentCommands enum
   - Implement `content last` command (most critical for justfile)
   - Implement `content list` command
   - Implement `content info` command
   - Add tests for CLI command handlers
   - **Commit: "Add content query CLI commands"** ✓

4. **Sprint 4: Justfile and Documentation**
   - Update `tui-demo` and other justfile recipes
   - Test end-to-end: `just tui-demo` should work
   - Update TUI_GUIDE.md with new workflow
   - Add examples to README.md if needed
   - **Commit: "Update justfile to use content tracking"** ✓

5. **Sprint 5: Polish and Cleanup (Optional)**
   - Implement `content clean` command
   - Add `--quiet` flag to `run` command
   - Add friendly success messages with next-step hints
   - Add bash completion for content commands
   - **Commit: "Add content cleanup and polish"** ✓

## Testing Strategy

### Unit Tests (`tests/content_generation_repository_test.rs`)

```rust
#[test]
fn test_start_generation() {
    let repo = create_test_repo();
    let new_gen = NewContentGeneration {
        table_name: "test_table".to_string(),
        narrative_file: "test.toml".to_string(),
        narrative_name: "test_narrative".to_string(),
        status: "running".to_string(),
        created_by: None,
    };
    
    let result = repo.start_generation(new_gen).unwrap();
    assert_eq!(result.status, "running");
    assert_eq!(result.table_name, "test_table");
}

#[test]
fn test_complete_generation_success() {
    let repo = create_test_repo();
    // Start generation first...
    
    let update = UpdateContentGeneration {
        completed_at: Some(Utc::now()),
        row_count: Some(42),
        generation_duration_ms: Some(1234),
        status: Some("success".to_string()),
        error_message: None,
    };
    
    let result = repo.complete_generation("test_table", update).unwrap();
    assert_eq!(result.status, "success");
    assert_eq!(result.row_count, Some(42));
}

#[test]
fn test_get_last_successful() {
    let repo = create_test_repo();
    // Generate multiple tables...
    
    let last = repo.get_last_successful().unwrap();
    assert!(last.is_some());
    // Should be most recent
}
```

### Integration Tests (`tests/narrative_tracking_integration_test.rs`)

```rust
#[test]
fn test_narrative_execution_creates_tracking_record() {
    let pool = create_test_db_pool();
    let repo = PostgresContentGenerationRepository::new(pool.clone());
    
    // Execute a test narrative
    execute_narrative("narratives/test_simple.toml", &pool).unwrap();
    
    // Verify tracking record exists
    let last = repo.get_last_successful().unwrap();
    assert!(last.is_some());
    let gen = last.unwrap();
    assert_eq!(gen.status, "success");
    assert!(gen.row_count.is_some());
}

#[test]
fn test_failed_generation_records_error() {
    let pool = create_test_db_pool();
    let repo = PostgresContentGenerationRepository::new(pool.clone());
    
    // Execute a narrative that should fail
    let result = execute_narrative("narratives/test_invalid.toml", &pool);
    assert!(result.is_err());
    
    // Verify failure was recorded
    let all = repo.list_generations(None, 10).unwrap();
    let failed = all.iter().find(|g| g.status == "failed");
    assert!(failed.is_some());
    assert!(failed.unwrap().error_message.is_some());
}
```

### Manual Testing Checklist

**After Sprint 1 (Database):**
- [ ] Migration runs successfully: `diesel migration run`
- [ ] Schema is correct: `psql -d boticelli -c "\d content_generations"`
- [ ] Can insert test record manually via psql

**After Sprint 2 (Executor):**
- [ ] Generate content: `cargo run -- run narratives/generate_guilds.toml`
- [ ] Check tracking table: `psql -d boticelli -c "SELECT * FROM content_generations;"`
- [ ] Verify record has correct table_name, status='success', row_count
- [ ] Test failure case: run invalid narrative, verify status='failed' recorded

**After Sprint 3 (CLI):**
- [ ] `cargo run -- content last` shows correct table
- [ ] `cargo run -- content last --format=table-name-only` outputs only table name
- [ ] `cargo run -- content list` shows all generations
- [ ] `cargo run -- content info potential_guilds` shows details
- [ ] Error handling: `cargo run -- content last` when no generations exist

**After Sprint 4 (Justfile):**
- [ ] `just tui-demo` works end-to-end without errors
- [ ] `just list-content` shows generated tables
- [ ] `just view-content potential_guilds` opens TUI
- [ ] Can run multiple times: `just tui-demo` twice in a row works

**Edge Cases:**
- [ ] Table names with underscores work
- [ ] Concurrent generations don't conflict (run two in parallel)
- [ ] Database connection errors are handled gracefully
- [ ] Empty database (no generations) doesn't crash

## Success Criteria

✅ **Must have (for minimum viable fix):**
- [ ] `content_generations` table exists and is tracked in migrations
- [ ] Narrative executor records start/success/failure to tracking table
- [ ] `boticelli content last --format=table-name-only` returns most recent table name
- [ ] `just tui-demo` works end-to-end without psql dependencies
- [ ] All tests pass (unit + integration)
- [ ] Zero clippy warnings

✅ **Should have (for production readiness):**
- [ ] `boticelli content list` shows all generations with metadata
- [ ] `boticelli content info <table>` shows detailed information
- [ ] JSON output format for all content commands
- [ ] Documentation updated (TUI_GUIDE.md, README.md)
- [ ] Error messages are clear and actionable
- [ ] Repository pattern allows easy testing and mocking

✅ **Nice to have (future enhancements):**
- [ ] `boticelli content clean` removes old tables automatically
- [ ] `boticelli run --quiet` for scripting (minimal output)
- [ ] Bash completion for content commands
- [ ] Rich table information (size, indexes, etc.)
- [ ] Analytics: average generation time, success rate
- [ ] Web UI for browsing generated content

## Architecture Decisions

### Why Track in Database vs Files?

**Decision:** Use PostgreSQL table for tracking, not files or stdout parsing.

**Rationale:**
1. **Atomic operations** - Database transactions ensure consistent state
2. **Queryable** - SQL makes it easy to filter, sort, aggregate
3. **Single source of truth** - No file-vs-database sync issues
4. **Performance** - Indexed queries are fast even with many generations
5. **Historical data** - Natural to keep long-term records
6. **Concurrent safe** - Database handles multiple writers correctly

**Trade-offs:**
- Requires database migration (acceptable, we already use Diesel)
- Slightly more complex than stdout parsing (but much more robust)

### Why Repository Pattern?

**Decision:** Use trait-based repository pattern for data access.

**Rationale:**
1. **Testability** - Easy to mock for unit tests
2. **Flexibility** - Could swap PostgreSQL for SQLite or mock in tests
3. **Clean architecture** - Business logic separated from persistence
4. **Following best practices** - Consistent with existing codebase patterns

### Why Separate `status` Field vs Boolean?

**Decision:** Use TEXT enum `('running', 'success', 'failed')` vs `success BOOLEAN`.

**Rationale:**
1. **Distinguishes in-progress** - Can detect stalled/orphaned generations
2. **Extensible** - Easy to add new states ('cancelled', 'retrying')
3. **Clear intent** - More readable than boolean + nullable fields
4. **Query friendly** - `WHERE status = 'success'` is clearer than `WHERE success = true AND completed_at IS NOT NULL`

## Current State

**Implementation Status:**
- [x] **Sprint 1 (Database): Complete ✅**
- [x] **Sprint 2 (Executor): Complete ✅**
- [x] **Sprint 3 (CLI): Complete ✅**
- [x] **Sprint 4 (Justfile): Complete ✅**
  - [x] Updated tui-demo to use content tracking ✅ Committed: a963f7d
  - [x] Added content-generations helper recipe
  - [x] Added content-last helper recipe
  - [x] Added tui-last helper recipe
  - [x] Removed hardcoded table name dependencies
- [ ] Sprint 5 (Polish): Optional enhancements

**Sprint 4 Summary:**
- ✅ Updated tui-demo to dynamically discover latest table
- ✅ Uses `content last --format=table-name-only` for automation
- ✅ Added 3 new justfile recipes for content management
- ✅ Robust error handling and user-friendly messages
- ✅ No more hardcoded "guilds_gen_001" table names
- ✅ Works with any content generation narrative
- ✅ All tests passing

**✅ ORIGINAL ISSUE RESOLVED:**

The TUI hardcoded table name issue is now fixed! The workflow is:

1. **Generate content**: `just example-guilds` or any content narrative
2. **Review with TUI**: `just tui-demo` or `just tui-last`
   - Automatically finds the most recent generation
   - No hardcoded table names
   - Works reliably every time

**New Commands Available:**
- `just tui-demo` - Generate guilds and launch TUI (all-in-one)
- `just tui-last` - Launch TUI on most recent generation
- `just content-last` - Show details of last generation
- `just content-generations` - List all tracked generations

**Completed Files:**
- ✅ `migrations/2025-11-17-022706-0000_create_content_generations/up.sql`
- ✅ `migrations/2025-11-17-022706-0000_create_content_generations/down.sql`
- ✅ `src/database/content_generation_models.rs`
- ✅ `src/database/content_generation_repository.rs`
- ✅ `src/database/schema.rs`
- ✅ `src/database/mod.rs`
- ✅ `src/lib.rs`
- ✅ `tests/content_generation_repository_test.rs`
- ✅ `tests/narrative_processor_integration_test.rs`
- ✅ `src/narrative/content_generation.rs`
- ✅ `src/cli.rs`
- ✅ `src/main.rs`
- ✅ `justfile` - Updated with content tracking
- ✅ `TUI_TROUBLESHOOTING.md` (this file)

**Ready to Push:**
- Commits: 883709e, fb5beba, bd2e015, 4992f39, 21ee1b5, 9694489, 3a5e352, 4c817f4, c84563d, a963f7d
- All 4 sprints complete and tested
- Issue resolved and production-ready

**Files To Create:**
- `migrations/YYYYMMDDHHMMSS_create_content_generations/up.sql`
- `migrations/YYYYMMDDHHMMSS_create_content_generations/down.sql`
- `src/database/content_generation_models.rs`
- `src/database/content_generation_repository.rs`
- `src/cli/content.rs`
- `tests/content_generation_repository_test.rs`
- `tests/narrative_tracking_integration_test.rs`

**Files To Modify:**
- `src/database/mod.rs` - Export new models and repository
- `src/lib.rs` - Export new types at crate level
- `src/bin/boticelli.rs` - Add Content command and handler
- `src/narrative/executor.rs` (or wherever execution happens) - Integrate tracking
- `justfile` - Update recipes to use `content last`
- `TUI_GUIDE.md` - Document new workflow
- `diesel.toml` - Ensure schema tracking is correct

### Next Immediate Action

**Start Sprint 1: Create Database Migration**

```bash
# Create migration
diesel migration generate create_content_generations

# Edit up.sql with schema from Phase 1
# Edit down.sql with DROP TABLE

# Run migration
diesel migration run

# Verify
diesel migration list
psql -d boticelli -c "\d content_generations"
```

After migration is verified, commit before moving to models.
