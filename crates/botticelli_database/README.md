# botticelli_database

PostgreSQL integration for the Botticelli ecosystem.

## Overview

Provides PostgreSQL persistence for narrative executions, content generation, and Discord data. Uses Diesel ORM for type-safe database operations.

## Features

- **Narrative persistence**: Store and retrieve narrative executions
- **Content generation**: Manage generated content with review workflow
- **Discord integration**: Store guilds, channels, members, roles, messages
- **Schema reflection**: Inspect existing database schemas
- **Schema inference**: Generate schemas from JSON data
- **Migrations**: Diesel migrations for schema management

## Usage

### Setup Database

```bash
# Create database
createdb botticelli

# Run migrations
diesel migration run
```

### Narrative Repository

```rust
use botticelli_database::PostgresNarrativeRepository;
use diesel::pg::PgConnection;

let mut conn = PgConnection::establish(&database_url)?;
let repo = PostgresNarrativeRepository::new(&mut conn)?;

// Save execution
let id = repo.save_execution(&execution).await?;

// List executions
let filter = ExecutionFilter {
    narrative_name: Some("my-narrative".to_string()),
    status: Some(ExecutionStatus::Completed),
    limit: Some(10),
    offset: None,
};
let summaries = repo.list_executions(&filter).await?;
```

### Content Generation

```rust
use botticelli_database::PostgresContentGenerationRepository;

let repo = PostgresContentGenerationRepository::new(&mut conn)?;

// Store generated content
let row = NewContentGenerationRow {
    table_name: "social_posts",
    json_data: serde_json::to_value(&post)?,
    narrative_name: Some("generate-post"),
    act_name: Some("draft"),
    //...
};
let id = repo.create(row).await?;

// List pending content for review
let pending = repo.list_by_status("pending", 10).await?;
```

### Schema Operations

```rust
use botticelli_database::{reflect_table_schema, infer_schema};

// Reflect existing schema
let schema = reflect_table_schema(&mut conn, "users")?;
println!("Columns: {:#?}", schema.columns);

// Infer schema from JSON
let json_data = json!([
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
]);
let inferred = infer_schema(&json_data)?;

// Create table from inferred schema
create_inferred_table(&mut conn, "people", &inferred, None, None)?;
```

## Database Schema

### Narrative Tables

- `narrative_executions` - Execution records
- `act_executions` - Individual act results

### Content Generation Tables

- `content_generation_tables` - Metadata about generated content tables
- `content_generation` - Generated content records
- Dynamic tables created per template

### Discord Tables

- `guilds` - Discord servers
- `channels` - Server channels
- `members` - Server members
- `roles` - Server roles
- `messages` - Chat messages

## Configuration

```toml
# botticelli.toml
[database]
host = "localhost"
port = 5432
user = "botticelli_user"
password = "secret"
database = "botticelli"
```

Or via environment variables:
```bash
DATABASE_USER=botticelli_user
DATABASE_PASSWORD=secret
DATABASE_NAME=botticelli
```

## Dependencies

- `diesel` - ORM and query builder
- `diesel_migrations` - Schema migrations
- `chrono` - DateTime handling
- `uuid` - UUID support
- `serde` / `serde_json` - JSON serialization

## Version

Current version: 0.2.0
