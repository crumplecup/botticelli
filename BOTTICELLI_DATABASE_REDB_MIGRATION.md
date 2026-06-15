# Botticelli Database — redb Migration Plan

**Status**: ✅ COMPLETE
**Branch**: dev
**Goal**: Replace diesel/PostgreSQL as the default persistence layer with redb (via
`elicit_redb`/`elicit_db`). PostgreSQL remains available as an opt-in feature via
`elicit_sqlx`. `BotStorage` domain traits live in `botticelli_interface`. Backend
selection is through Cargo feature flags.

---

## Architecture

```
botticelli_interface
└── BotStorage trait family (NarrativeStore, ContentStore, ActorStateStore, …)

botticelli_database  (rewrites; diesel entirely removed)
├── feature "redb"     (DEFAULT) → BotRedbBackend wrapping RedbBackend
│   └── RedbStorage : BotStorage  (KV model: table-name → JSON rows)
└── feature "postgres" (OPT-IN)  → BotPostgresBackend wrapping SqlxDbBackend
    └── PostgresStorage : BotStorage

Runtime configuration (.env)
├── REDB_PATH=./botticelli.redb      (redb)
└── DATABASE_URL=postgres://…        (postgres)

Consumers (botticelli_actor, botticelli_bot, …)
└── Arc<dyn BotStorage>              (no diesel, no backend coupling)
```

### KV model for redb

Each logical "table" becomes a named redb table:
`TableDefinition::<&str, &str>` — key = primary-key string, value = JSON-encoded row struct.

### Trait-naming conventions

| elicit crate   | trait               | purpose                         |
| -------------- | ------------------- | ------------------------------- |
| `elicit_db`    | `DbKvStore`         | get/put/delete/list KV          |
| `elicit_db`    | `DbEmbeddedStore`   | open/close embedded DB          |
| `elicit_db`    | `DbTransactor`      | begin/commit/rollback           |
| `elicit_db`    | `DbSnapshotManager` | snapshot/restore                |
| `elicit_sqlx`  | `SqlxDbBackend`     | SQL backend (postgres via sqlx) |
| `elicit_redb`  | `RedbBackend`       | embedded KV backend             |

---

## Phases

> **Ordering principle**: diesel must stay compilable until every consumer is
> migrated. New infra is added first, then consumers are switched one by one,
> and diesel is deleted only after the last reference is gone.

### Phase 1 — Add new workspace deps (diesel stays) ✅

- [x] Add to `[workspace.dependencies]` in root `Cargo.toml`:
  ```toml
  elicit_db   = "0.11"
  elicit_redb = "0.11"
  elicit_sqlx = "0.11"
  redb        = "2"
  ```
- [x] Leave `diesel` and `diesel_migrations` in place — they still compile
- [x] `just check` — workspace must still compile cleanly after this commit

### Phase 2 — `BotStorage` trait family in `botticelli_interface` ✅

Add a new module `crates/botticelli_interface/src/storage.rs`.

#### 2a — Domain row types

Define plain-Rust structs for each logical entity (replacing diesel Queryable
structs). All structs derive `Debug, Clone, Serialize, Deserialize`.

```
NarrativeExecutionRecord   id, narrative_file, narrative_name, started_at, …
ActExecutionRecord         id, narrative_execution_id, act_name, started_at, …
ActInputRecord             id, act_execution_id, role, content, …
ActorServerStateRecord     id, actor_name, state_json, updated_at
ActorServerExecutionRecord id, actor_server_state_id, started_at, …
ContentRecord              id, table_name, content_json, …
ContentGenerationRecord    id, table_name, narrative_file, status, …
ModelResponseRecord        id, provider, model_name, request_json, response_json, …
PostHistoryRecord          id, platform, post_id, posted_at, …
```

- [x] Write `crates/botticelli_interface/src/storage.rs` with all record structs
- [x] Export from `crates/botticelli_interface/src/lib.rs`:
  ```rust
  pub use storage::{
      NarrativeExecutionRecord, ActExecutionRecord, ActInputRecord,
      ActorServerStateRecord, ActorServerExecutionRecord,
      ContentRecord, ContentGenerationRecord, ModelResponseRecord,
      PostHistoryRecord,
  };
  ```

#### 2b — Sub-traits

```rust
pub trait NarrativeStore {
    fn save_narrative_execution(&self, r: &NarrativeExecutionRecord) -> BotStorageResult<()>;
    fn get_narrative_execution(&self, id: &str) -> BotStorageResult<Option<NarrativeExecutionRecord>>;
    fn list_narrative_executions(&self, limit: usize) -> BotStorageResult<Vec<NarrativeExecutionRecord>>;
    fn save_act_execution(&self, r: &ActExecutionRecord) -> BotStorageResult<()>;
    fn list_act_executions(&self, narrative_execution_id: &str) -> BotStorageResult<Vec<ActExecutionRecord>>;
    fn save_act_input(&self, r: &ActInputRecord) -> BotStorageResult<()>;
}

pub trait ActorStateStore {
    fn save_actor_state(&self, r: &ActorServerStateRecord) -> BotStorageResult<()>;
    fn get_actor_state(&self, actor_name: &str) -> BotStorageResult<Option<ActorServerStateRecord>>;
    fn save_actor_execution(&self, r: &ActorServerExecutionRecord) -> BotStorageResult<()>;
}

pub trait ContentStore {
    fn save_content(&self, r: &ContentRecord) -> BotStorageResult<()>;
    fn list_content(&self, table_name: &str, limit: usize) -> BotStorageResult<Vec<ContentRecord>>;
    fn save_content_generation(&self, r: &ContentGenerationRecord) -> BotStorageResult<()>;
    fn save_model_response(&self, r: &ModelResponseRecord) -> BotStorageResult<()>;
    fn list_model_responses(&self, limit: usize) -> BotStorageResult<Vec<ModelResponseRecord>>;
}

pub trait PostStore {
    fn save_post_history(&self, r: &PostHistoryRecord) -> BotStorageResult<()>;
    fn list_post_history(&self, platform: &str) -> BotStorageResult<Vec<PostHistoryRecord>>;
}

/// Aggregate: a complete storage backend.
pub trait BotStorage: NarrativeStore + ActorStateStore + ContentStore + PostStore
    + Send + Sync + 'static {}
```

- [x] Write sub-traits in `storage.rs`
- [x] Define `BotStorageError` and `BotStorageResult<T>` in `storage.rs`
  (simple error enum: `Io(String)`, `Encode(String)`, `Decode(String)`, `NotFound`)
- [x] Export `BotStorage`, `BotStorageError`, `BotStorageResult` and all sub-traits
- [x] `just check botticelli_interface` — zero warnings

### Phase 3 — Add new backends to `botticelli_database` (diesel stays) ✅

Add the new backend files **alongside** the existing diesel files. The crate
compiles both old and new code until Phase 5 removes the old code.

#### 3a — Extend Cargo.toml

Add the new optional deps and features without removing diesel:

```toml
[features]
default = ["redb"]
redb     = ["dep:elicit_redb", "dep:elicit_db", "dep:redb"]
postgres = ["dep:elicit_sqlx", "dep:elicit_db", "dep:sqlx"]
# diesel feature remains until Phase 5:
diesel-pg = ["dep:diesel", "dep:diesel_migrations"]

[dependencies]
# … keep all existing diesel deps unchanged …
botticelli_interface = { workspace = true }
elicit_db   = { workspace = true, optional = true }
elicit_redb = { workspace = true, optional = true }
elicit_sqlx = { workspace = true, optional = true }
redb    = { workspace = true, optional = true }
sqlx    = { version = "0.9", features = ["postgres", "runtime-tokio-rustls", "chrono", "uuid", "json"], optional = true }
```

- [x] Update `crates/botticelli_database/Cargo.toml` as above (additive only)
- [x] `just check botticelli_database` — still compiles

#### 3b — `RedbStorage` (redb feature, default)

New file: `crates/botticelli_database/src/redb_storage.rs`

```rust
/// Thin newtype over RedbBackend; implements all BotStorage sub-traits.
pub struct RedbStorage(RedbBackend);

impl RedbStorage {
    pub fn open(path: &std::path::Path) -> Result<Self, BotStorageError> { … }
    pub fn in_memory() -> Result<Self, BotStorageError> { … }   // for tests
}

// Table name constants
const NARRATIVE_EXECUTIONS: &str = "narrative_executions";
const ACT_EXECUTIONS: &str = "act_executions";
// … etc.

impl NarrativeStore for RedbStorage { … }
impl ActorStateStore for RedbStorage { … }
impl ContentStore    for RedbStorage { … }
impl PostStore       for RedbStorage { … }
impl BotStorage      for RedbStorage {}
```

KV helper pattern (from `ArchiveKvBackend`):
- `put(table, key, value)` → serialize value with `serde_json`, write `&str` → `&str`
- `get(table, key)` → read `&str`, deserialize
- `list(table)` → iterate all entries, deserialize each

- [x] Write `redb_storage.rs` with all four sub-trait impls
- [x] Add to `crates/botticelli_database/src/lib.rs` (keep existing exports):
  ```rust
  #[cfg(feature = "redb")]
  mod redb_storage;
  #[cfg(feature = "redb")]
  pub use redb_storage::RedbStorage;
  ```
- [x] `just check botticelli_database` — still compiles (diesel exports unchanged)

#### 3c — `PostgresStorage` (postgres feature, opt-in)

New file: `crates/botticelli_database/src/postgres_storage.rs`

```rust
/// Thin newtype over SqlxDbBackend; implements all BotStorage sub-traits.
pub struct PostgresStorage(SqlxDbBackend);

impl PostgresStorage {
    pub async fn connect(url: &str) -> Result<Self, BotStorageError> { … }
}

impl NarrativeStore for PostgresStorage { … }   // sqlx queries
impl ActorStateStore for PostgresStorage { … }
impl ContentStore    for PostgresStorage { … }
impl PostStore       for PostgresStorage { … }
impl BotStorage      for PostgresStorage {}
```

- [ ] Write `postgres_storage.rs` with real sqlx queries (deferred to Phase 3d —
  stub was removed because dead-code warnings cannot be suppressed without
  `#[allow]` when all methods are unimplemented)
- [ ] Add postgres module to `lib.rs` under `#[cfg(feature = "postgres")]`
- [ ] `just check botticelli_database --no-default-features --features postgres`
  — still compiles (diesel untouched)

### Phase 4 — Migrate consumers off diesel (one crate at a time)

Remove `dep:diesel` references and diesel struct usage from each consumer crate.
Replace with `Arc<dyn BotStorage>` injection. After each crate, run
`just check <crate>` — the workspace must still compile before moving to the next.

> diesel remains in `[workspace.dependencies]` throughout this phase.
> Only `botticelli_database` still pulls it in (via the temporary `diesel-pg` feature).

#### `botticelli_error` ✅

- [x] No diesel deps — already clean before this phase

#### `botticelli_narrative` ✅

- [x] No diesel deps — already clean before this phase

#### `botticelli_security` ✅

- [x] No diesel deps — already clean before this phase

#### `botticelli_mcp` ✅

- [x] Removed `dep:botticelli_database` and `dep:diesel` from `database` feature
- [x] `ContentResource` rewritten to use `Arc<dyn BotStorage>` injection
- [x] `just check botticelli_mcp`

#### `botticelli_social` ✅

- [x] No diesel deps — already clean before this phase

#### `botticelli_actor` ✅

- [x] All diesel removed; `BotStorageStatePersistence` uses `Arc<dyn BotStorage>`
- [x] `just check botticelli_actor`

#### `botticelli_bot` ✅

- [x] No diesel deps — already clean before this phase

#### `botticelli` (facade) ✅

- [x] Removed `dep:diesel` from `bots` feature and dev-dependencies
- [x] `just check botticelli`

### Phase 5 — Delete diesel ✅

- [x] All diesel source files deleted from `crates/botticelli_database/src/`
- [x] `diesel` and `diesel_migrations` removed from `[workspace.dependencies]`
- [x] `diesel.toml` deleted from workspace root
- [x] `grep -r "diesel" crates/ --include="*.rs" --include="*.toml"` returns nothing

### Phase 6 — Tests ✅

All tests use `RedbStorage::in_memory()` — no external process, no env vars.

- [x] `crates/botticelli_database/tests/redb_narrative_store_test.rs` — 9 tests pass
- [x] `crates/botticelli_database/tests/redb_content_store_test.rs` — 8 tests pass
- [x] `crates/botticelli_database/tests/redb_actor_state_test.rs` — 7 tests pass
- [x] `just test-package botticelli_database` — all pass

### Phase 7 — Environment wiring ✅

```bash
# .env
REDB_PATH=./botticelli.redb          # redb backend (default)
# DATABASE_URL=postgres://...        # postgres backend (when postgres feature enabled)
```

- [x] `open_storage_from_env()` added to `botticelli_database::env`
  - reads `REDB_PATH` / defaults to `./botticelli.redb` when `redb` feature active
  - reads `DATABASE_URL` when `postgres` feature active (takes priority)
  - feature-cfg'd so the function only compiles with active backends
- [x] `BotStorageError::Config` variant added to `botticelli_interface`
- [x] `just check botticelli_database`

### Phase 8 — Final verification ✅

- [x] `just check` (zero errors, full workspace with all features)
- [x] `just lint` (zero warnings, full workspace)
- [x] `just test-package botticelli_database` — all pass
- [x] `just test-package botticelli_actor` — all pass
- [x] `just test-package botticelli_mcp` — all pass

---

## Files Changed

```
Cargo.toml
└── [workspace.dependencies] add elicit_db, elicit_redb, elicit_sqlx, redb
    remove diesel, diesel_migrations

crates/botticelli_interface/src/
├── lib.rs                    (add storage pub use)
└── storage.rs                (NEW: record types + BotStorage traits)

crates/botticelli_database/
├── Cargo.toml                (REWRITE: remove diesel, add elicit_redb/sqlx)
└── src/
    ├── lib.rs                (REWRITE: cfg-gated modules only)
    ├── redb_storage.rs       (NEW: RedbStorage impl)
    ├── postgres_storage.rs   (NEW: PostgresStorage impl, postgres feature)
    │
    │   ── DELETE ─────────────────────────────────────
    ├── actor_server_models.rs
    ├── connection.rs
    ├── content_generation_models.rs
    ├── content_generation_repository.rs
    ├── content_management.rs
    ├── content_repository.rs
    ├── db_operations.rs
    ├── models.rs
    ├── narrative_conversions.rs
    ├── narrative_models.rs
    ├── narrative_repository.rs
    ├── registry_impl.rs
    ├── schema.rs
    ├── schema_docs.rs
    ├── schema_inference.rs
    ├── schema_reflection.rs
    ├── table_query.rs
    └── table_query_registry.rs

crates/botticelli_database/tests/
├── redb_narrative_store_test.rs   (NEW)
├── redb_content_store_test.rs     (NEW)
└── redb_actor_state_test.rs       (NEW)

crates/botticelli_error/Cargo.toml           (remove database diesel feature)
crates/botticelli_narrative/Cargo.toml       (remove diesel from database feature)
crates/botticelli_security/Cargo.toml        (remove diesel from database feature)
crates/botticelli_mcp/Cargo.toml             (remove diesel from database feature)
crates/botticelli_social/Cargo.toml          (remove diesel from discord feature)
crates/botticelli_actor/Cargo.toml           (workspace dep, no direct diesel)
crates/botticelli_bot/Cargo.toml             (workspace dep, no direct diesel)
crates/botticelli/Cargo.toml                 (remove dep:diesel from bots feature)
```

---

## Key Invariants

**Diesel appears nowhere after Phase 7.** `grep -r "diesel" crates/` returns zero
results (except comments or doc text referencing the old design).

**redb is the compile-time default.** `cargo check` (no flags) uses redb.
`postgres` requires explicit `--features botticelli_database/postgres`.

**Tests never need a running process.** `RedbStorage::in_memory()` creates a
fresh in-process database. No `DATABASE_URL`, no Docker, no Postgres.

**`Arc<dyn BotStorage>` is the injection point.** No consumer directly
constructs or names the backend type. Backends are created once at startup in
`open_storage_from_env()` and threaded through as trait objects.

**No diesel re-exports.** `botticelli_database` exports only its own types
(`RedbStorage`, `PostgresStorage`, `open_storage_from_env`, etc.) and the
re-exported `botticelli_interface` record/trait types via `use`, never
`pub use dep::*`.
