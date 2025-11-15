# Media Storage Architecture Migration Plan

## Executive Summary

This document outlines the migration from storing binary media (images, audio, video) directly in PostgreSQL to a scalable, tiered storage architecture that separates metadata from content.

## Current Problems

### 1. Binary Data in PostgreSQL

- **Denormalization**: `act_inputs` table stores the same media in multiple formats:
  - `source_base64` (TEXT) - Base64-encoded string
  - `source_binary` (BYTEA) - Raw binary data
  - `source_url` (TEXT) - External URL reference
- **Performance**: Binary blobs mixed with structured data hurt query performance
- **Size Limits**: PostgreSQL row size limits (8KB inline, 1GB compressed) are impractical for video
- **Backup Issues**: Database backups become enormous and slow with binary data

### 2. Scalability Blockers

- Video files (100MB-10GB+) will overwhelm the database
- No CDN integration, streaming support, or multi-region distribution
- Database migrations with large binary data are extremely slow
- No clear path to horizontal scaling

### 3. Access Pattern Mismatch

- **Text queries** (narrative history, execution status) need ACID, transactions, joins
- **Media retrieval** needs caching, CDN, range requests, streaming
- PostgreSQL optimized for former, poor fit for latter
- Different caching strategies needed for each

## Proposed Architecture

### Tiered Storage Strategy

**Tier 1: PostgreSQL - Metadata Only**

Store only references and metadata about media files:

```sql
CREATE TABLE media_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_type TEXT NOT NULL,           -- 'image', 'audio', 'video'
    mime_type TEXT NOT NULL,            -- 'image/png', 'video/mp4', etc.
    size_bytes BIGINT NOT NULL,
    content_hash TEXT NOT NULL,         -- SHA-256 for deduplication
    storage_backend TEXT NOT NULL,      -- 's3', 'filesystem', 'gcs', etc.
    storage_path TEXT NOT NULL,         -- backend-specific reference
    uploaded_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMP,
    access_count INT DEFAULT 0,
    
    -- Optional metadata
    width INT,                          -- For images/video
    height INT,                         -- For images/video
    duration_seconds FLOAT,             -- For audio/video
    
    CONSTRAINT unique_content UNIQUE (content_hash)
);

CREATE INDEX idx_media_content_hash ON media_references(content_hash);
CREATE INDEX idx_media_type ON media_references(media_type);
CREATE INDEX idx_media_storage ON media_references(storage_backend, storage_path);
```

**Tier 2: Object Storage - Binary Content**

- **Small images (< 1MB)**: Optional PostgreSQL BYTEA for simplicity
- **Large images, all audio/video**: Object storage (S3, filesystem, etc.)

### Storage Backend Hierarchy

**Phase 1: Filesystem Storage (Immediate)**

- Content-addressable storage by hash
- Directory structure: `/var/boticelli/media/{type}/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}`
- Automatic deduplication via hash
- Simple backup with rsync
- Good for single-server deployments

**Phase 2: S3/MinIO (Production)**

- Infinitely scalable
- CDN integration (CloudFront, Cloudflare)
- Versioning, lifecycle policies, multi-region
- Cost-effective cold storage tiers
- Direct browser uploads via presigned URLs

**Phase 3: Hybrid Strategy (Optimization)**

- Smart routing by file size and access patterns
- Hot data in fast storage, cold data in archive tiers
- Automatic migration based on access frequency

## Implementation Plan

### Step 1: Create Storage Abstraction ✅ COMPLETE

**Status**: Implemented and tested  
**Date Completed**: 2025-01-15

**1.1 Define Core Trait** ✅

Created `src/storage/mod.rs` with:
- `MediaStorage` trait defining the storage backend interface
- `MediaMetadata` struct for media information during storage
- `MediaReference` struct for retrieving stored media
- `MediaType` enum for image/audio/video discrimination

**Key Design Decisions**:
- Made `uuid` a required dependency (not optional) since it's needed for media references
- Used `#[track_caller]` in error constructors for automatic location tracking
- All trait methods are async to support both local and remote storage backends
- `get_url()` returns `Option<String>` to support backends without direct URL access

**1.2 Create Module Structure** ✅

```
src/storage/
├── mod.rs              # ✅ Trait definitions, public API
├── error.rs           # ✅ Storage-specific errors
├── filesystem.rs       # ⏳ TODO: FileSystemStorage implementation  
├── postgres.rs         # ⏳ TODO: PostgresStorage (for small files)
├── s3.rs              # ⏳ TODO: S3Storage (future)
└── hybrid.rs          # ⏳ TODO: HybridStorage (smart routing)
```

**Error Handling**:
- Created `StorageError` and `StorageErrorKind` following project patterns
- Integrated into crate-level `BoticelliErrorKind` enum
- Added automatic `From` conversion via `derive_more`

**1.3 Add to lib.rs exports** ✅

Re-exported public types:
```rust
pub use storage::{
    MediaMetadata, MediaReference, MediaStorage, MediaType, StorageError, StorageErrorKind,
};
```

**Dependencies Added**:
```toml
sha2 = "0.10"          # For content hashing
uuid = "1.18"          # Made non-optional for media reference IDs
```

**Tests**: All existing tests pass (9/9)

---

### Step 2: Implement Filesystem Storage ✅ COMPLETE

**Status**: Implemented and tested  
**Date Completed**: 2025-01-15

**2.1 Core Implementation** ✅

Created `src/storage/filesystem.rs` with full `FileSystemStorage` implementation:

**Key Features**:
- **Content-addressable storage**: Files stored by SHA-256 hash
- **Automatic deduplication**: Same content → same hash → same file location
- **Atomic writes**: Uses temp file + rename pattern for crash safety
- **Organized structure**: Two-level subdirectories (`images/ab/cd/abcdef...`) prevent directory bloat
- **Hash verification**: Detects corruption on retrieval
- **Structured logging**: Uses `tracing` for debug/info logs

**Directory Structure**:
```text
{base_path}/
├── images/
│   └── {hash[0:2]}/
│       └── {hash[2:4]}/
│           └── {full_hash}
├── audio/
│   └── {hash[0:2]}/
│       └── {hash[2:4]}/
│           └── {full_hash}
└── video/
    └── {hash[0:2]}/
        └── {hash[2:4]}/
            └── {full_hash}
```

**Implementation Highlights**:
- All operations async using `tokio::fs`
- Returns `None` for `get_url()` (filesystem doesn't support direct URLs)
- Creates parent directories automatically
- Handles both I/O errors and hash mismatches appropriately

**2.2 Comprehensive Test Suite** ✅

Added 7 tests covering all functionality:
1. `test_store_and_retrieve` - Basic store/retrieve cycle
2. `test_deduplication` - Verifies same content uses same file
3. `test_hash_verification` - Detects corrupted files on read
4. `test_delete` - File deletion and existence checks
5. `test_not_found` - Proper error handling for missing files
6. `test_content_addressable_structure` - Validates directory layout
7. `test_no_direct_urls` - Confirms URL generation returns None

**Test Results**: All 16 tests pass (9 existing + 7 new)

**2.3 Integration** ✅

- Exported `FileSystemStorage` from `storage` module
- Re-exported in `lib.rs` for public API
- Implemented `std::str::FromStr` for `MediaType` (fixes clippy warning)

**Clippy**: No warnings

---

### Step 3: Database Migration (1 day)

**3.1 Create Migration**

```bash
diesel migration generate add_media_references
```

**3.2 Migration Up SQL**

```sql
-- migrations/YYYY-MM-DD-HHMMSS_add_media_references/up.sql

-- Create media_references table
CREATE TABLE media_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_type TEXT NOT NULL CHECK (media_type IN ('image', 'audio', 'video')),
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    content_hash TEXT NOT NULL,
    storage_backend TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    uploaded_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMP,
    access_count INT DEFAULT 0,
    
    -- Optional metadata
    width INT,
    height INT,
    duration_seconds REAL,
    
    CONSTRAINT unique_content UNIQUE (content_hash)
);

CREATE INDEX idx_media_content_hash ON media_references(content_hash);
CREATE INDEX idx_media_type ON media_references(media_type);
CREATE INDEX idx_media_storage ON media_references(storage_backend, storage_path);

-- Add foreign key to act_inputs
ALTER TABLE act_inputs 
ADD COLUMN media_ref_id UUID REFERENCES media_references(id) ON DELETE SET NULL;

CREATE INDEX idx_act_inputs_media_ref ON act_inputs(media_ref_id);

-- Add comments
COMMENT ON TABLE media_references IS 'Metadata for media files stored outside the database';
COMMENT ON COLUMN act_inputs.media_ref_id IS 'Reference to media stored in media_references table';
```

**3.3 Migration Down SQL**

```sql
-- migrations/YYYY-MM-DD-HHMMSS_add_media_references/down.sql

DROP INDEX IF EXISTS idx_act_inputs_media_ref;
ALTER TABLE act_inputs DROP COLUMN IF EXISTS media_ref_id;

DROP INDEX IF EXISTS idx_media_storage;
DROP INDEX IF EXISTS idx_media_type;
DROP INDEX IF EXISTS idx_media_content_hash;

DROP TABLE IF EXISTS media_references;
```

### Step 4: Update Repository Layer (2-3 days)

**4.1 Add MediaStorage to NarrativeRepository Trait**

```rust
// src/narrative/repository.rs

pub trait NarrativeRepository: Send + Sync {
    // ... existing methods
    
    /// Store media using configured storage backend
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &MediaMetadata,
    ) -> BoticelliResult<MediaReference>;
    
    /// Retrieve media by reference
    async fn load_media(
        &self,
        reference: &MediaReference,
    ) -> BoticelliResult<Vec<u8>>;
    
    /// Get media reference by content hash (deduplication)
    async fn get_media_by_hash(
        &self,
        content_hash: &str,
    ) -> BoticelliResult<Option<MediaReference>>;
}
```

**4.2 Update DatabaseNarrativeRepository**

```rust
// src/database/narrative_repository.rs

pub struct DatabaseNarrativeRepository {
    pool: Arc<PgPool>,
    storage: Arc<dyn MediaStorage>,  // Add this
}

impl DatabaseNarrativeRepository {
    pub fn new(pool: Arc<PgPool>, storage: Arc<dyn MediaStorage>) -> Self {
        Self { pool, storage }
    }
}

impl NarrativeRepository for DatabaseNarrativeRepository {
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &MediaMetadata,
    ) -> BoticelliResult<MediaReference> {
        use sha2::{Sha256, Digest};
        
        // Compute hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        
        // Check if already exists
        if let Some(existing) = self.get_media_by_hash(&hash).await? {
            return Ok(existing);
        }
        
        // Store in backend
        let reference = self.storage.store(data, metadata).await?;
        
        // Save reference to database
        use crate::database::schema::media_references;
        
        diesel::insert_into(media_references::table)
            .values(&NewMediaReferenceRow {
                id: reference.id,
                media_type: reference.media_type.to_string(),
                mime_type: &reference.mime_type,
                size_bytes: reference.size_bytes,
                content_hash: &reference.content_hash,
                storage_backend: &reference.storage_backend,
                storage_path: &reference.storage_path,
                width: metadata.width.map(|w| w as i32),
                height: metadata.height.map(|h| h as i32),
                duration_seconds: metadata.duration_seconds,
            })
            .execute(&mut self.pool.get()?)?;
        
        Ok(reference)
    }
    
    // ... implement other methods
}
```

**4.3 Update Conversion Functions**

```rust
// src/database/narrative_conversions.rs

async fn input_to_row(
    input: &Input,
    order: i32,
    repository: &dyn NarrativeRepository,
) -> BoticelliResult<NewActInputRow> {
    let mut row = NewActInputRow {
        input_order: order,
        input_type: input_type_string(input),
        ..Default::default()
    };
    
    match input {
        Input::Text { content } => {
            row.text_content = Some(content.clone());
        }
        Input::Image { mime, source } => {
            row.mime_type = Some(mime.clone());
            let media_ref = store_media_source(source, MediaType::Image, mime, repository).await?;
            row.media_ref_id = Some(media_ref.id);
        }
        Input::Audio { mime, source } => {
            row.mime_type = Some(mime.clone());
            let media_ref = store_media_source(source, MediaType::Audio, mime, repository).await?;
            row.media_ref_id = Some(media_ref.id);
        }
        Input::Video { mime, source } => {
            row.mime_type = Some(mime.clone());
            let media_ref = store_media_source(source, MediaType::Video, mime, repository).await?;
            row.media_ref_id = Some(media_ref.id);
        }
        // ... other variants
    }
    
    Ok(row)
}

async fn store_media_source(
    source: &MediaSource,
    media_type: MediaType,
    mime_type: &str,
    repository: &dyn NarrativeRepository,
) -> BoticelliResult<MediaReference> {
    let data = match source {
        MediaSource::Binary(bytes) => bytes.clone(),
        MediaSource::Base64(base64) => {
            use base64::{Engine, engine::general_purpose::STANDARD};
            STANDARD.decode(base64)?
        }
        MediaSource::Url(_) => {
            // For URLs, we might want to fetch and store, or just keep the URL
            // For now, return an error - handle in next phase
            return Err(BoticelliError::new(BoticelliErrorKind::Storage(
                StorageError::new("URL media sources not yet supported for storage".to_string())
            )));
        }
    };
    
    let metadata = MediaMetadata {
        media_type,
        mime_type: mime_type.to_string(),
        filename: None,
        width: None,
        height: None,
        duration_seconds: None,
    };
    
    repository.store_media(&data, &metadata).await
}
```

### Step 5: Data Migration Script (1-2 days)

**5.1 Create Migration Tool**

```rust
// src/bin/migrate_media.rs

//! One-time migration tool to move existing binary data to new storage backend

use boticelli::{DatabaseNarrativeRepository, FileSystemStorage, MediaType, MediaMetadata};
use diesel::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let database_url = std::env::var("DATABASE_URL")?;
    let storage_path = std::env::var("MEDIA_STORAGE_PATH")
        .unwrap_or_else(|_| "/var/boticelli/media".to_string());
    
    // Setup
    let pool = create_pool(&database_url)?;
    let storage = Arc::new(FileSystemStorage::new(storage_path)?);
    let repo = DatabaseNarrativeRepository::new(pool.clone(), storage);
    
    // Find all act_inputs with binary data
    let inputs_with_binary = find_inputs_with_binary(&pool)?;
    
    println!("Found {} inputs with binary data to migrate", inputs_with_binary.len());
    
    let mut migrated = 0;
    let mut skipped = 0;
    let mut failed = 0;
    
    for input_row in inputs_with_binary {
        match migrate_input(&input_row, &repo).await {
            Ok(true) => migrated += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                eprintln!("Failed to migrate input {}: {}", input_row.id, e);
                failed += 1;
            }
        }
        
        if (migrated + skipped + failed) % 100 == 0 {
            println!("Progress: {} migrated, {} skipped, {} failed", 
                     migrated, skipped, failed);
        }
    }
    
    println!("\nMigration complete!");
    println!("  Migrated: {}", migrated);
    println!("  Skipped:  {}", skipped);
    println!("  Failed:   {}", failed);
    
    Ok(())
}

async fn migrate_input(
    input_row: &ActInputRow,
    repo: &DatabaseNarrativeRepository,
) -> Result<bool, Box<dyn std::error::Error>> {
    // If already has media_ref_id, skip
    if input_row.media_ref_id.is_some() {
        return Ok(false);
    }
    
    // Get binary data (prefer source_binary, fall back to source_base64)
    let data = if let Some(binary) = &input_row.source_binary {
        binary.clone()
    } else if let Some(base64) = &input_row.source_base64 {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.decode(base64)?
    } else {
        return Ok(false); // No binary data to migrate
    };
    
    // Determine media type
    let media_type = match input_row.input_type.as_str() {
        "image" => MediaType::Image,
        "audio" => MediaType::Audio,
        "video" => MediaType::Video,
        _ => return Ok(false),
    };
    
    let metadata = MediaMetadata {
        media_type,
        mime_type: input_row.mime_type.clone().unwrap_or_default(),
        filename: input_row.filename.clone(),
        width: None,
        height: None,
        duration_seconds: None,
    };
    
    // Store in new backend
    let media_ref = repo.store_media(&data, &metadata).await?;
    
    // Update act_inputs row
    use boticelli::database::schema::act_inputs;
    diesel::update(act_inputs::table.find(input_row.id))
        .set(act_inputs::media_ref_id.eq(media_ref.id))
        .execute(&mut repo.pool.get()?)?;
    
    Ok(true)
}
```

**5.2 Run Migration**

```bash
# Build migration tool
cargo build --release --bin migrate_media --features database

# Run migration
DATABASE_URL=postgres://user:pass@localhost/boticelli \
MEDIA_STORAGE_PATH=/var/boticelli/media \
./target/release/migrate_media
```

### Step 6: Update CLI and Configuration (1 day)

**6.1 Add Storage Configuration to boticelli.toml**

```toml
[storage]
backend = "filesystem"
base_path = "/var/boticelli/media"

# Future: S3 configuration
# [storage.s3]
# bucket = "boticelli-media"
# region = "us-east-1"
# access_key_id = "${AWS_ACCESS_KEY_ID}"
# secret_access_key = "${AWS_SECRET_ACCESS_KEY}"
```

**6.2 Update CLI Initialization**

```rust
// src/main.rs or wherever CLI setup happens

fn create_repository(config: &Config) -> BoticelliResult<Arc<dyn NarrativeRepository>> {
    let pool = create_database_pool(&config.database_url)?;
    
    // Create storage backend based on configuration
    let storage: Arc<dyn MediaStorage> = match &config.storage.backend {
        "filesystem" => {
            Arc::new(FileSystemStorage::new(&config.storage.base_path)?)
        }
        "postgres" => {
            Arc::new(PostgresStorage::new(pool.clone())?)
        }
        other => {
            return Err(BoticelliError::new(
                BoticelliErrorKind::Config(ConfigError::new(
                    format!("Unknown storage backend: {}", other)
                ))
            ));
        }
    };
    
    Ok(Arc::new(DatabaseNarrativeRepository::new(pool, storage)))
}
```

### Step 7: Remove Old Columns (After Migration Validated)

**7.1 Create Cleanup Migration**

```bash
diesel migration generate remove_old_media_columns
```

**7.2 Migration SQL**

```sql
-- migrations/YYYY-MM-DD-HHMMSS_remove_old_media_columns/up.sql

-- Remove old binary storage columns
ALTER TABLE act_inputs DROP COLUMN IF EXISTS source_base64;
ALTER TABLE act_inputs DROP COLUMN IF EXISTS source_binary;

-- Keep source_url for external references
-- ALTER TABLE act_inputs DROP COLUMN IF EXISTS source_url;  -- Commented out

-- Add check constraint to ensure either media_ref_id or text_content is set
ALTER TABLE act_inputs ADD CONSTRAINT check_input_content 
    CHECK (
        media_ref_id IS NOT NULL 
        OR text_content IS NOT NULL 
        OR source_url IS NOT NULL
    );
```

### Step 8: S3 Implementation (Future Phase)

**8.1 Add Dependencies**

```toml
[dependencies]
aws-sdk-s3 = { version = "1.0", optional = true }

[features]
storage-s3 = ["aws-sdk-s3"]
```

**8.2 Implement S3Storage**

```rust
// src/storage/s3.rs

#[cfg(feature = "storage-s3")]
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
}

#[cfg(feature = "storage-s3")]
impl MediaStorage for S3Storage {
    async fn store(&self, data: &[u8], metadata: &MediaMetadata) 
        -> BoticelliResult<MediaReference> 
    {
        let hash = Self::compute_hash(data);
        let key = self.get_key(&hash, metadata.media_type);
        
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.to_vec().into())
            .content_type(&metadata.mime_type)
            .send()
            .await?;
        
        Ok(MediaReference {
            id: Uuid::new_v4(),
            content_hash: hash,
            storage_backend: "s3".to_string(),
            storage_path: format!("s3://{}/{}", self.bucket, key),
            size_bytes: data.len() as i64,
            media_type: metadata.media_type,
            mime_type: metadata.mime_type.clone(),
        })
    }
    
    async fn get_url(&self, reference: &MediaReference, expires_in: Duration) 
        -> BoticelliResult<Option<String>> 
    {
        // Generate presigned URL
        let expires_in = expires_in.as_secs() as u32;
        let key = reference.storage_path.strip_prefix(&format!("s3://{}/", self.bucket))
            .unwrap_or(&reference.storage_path);
        
        let presigned = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(
                aws_sdk_s3::presigning::PresigningConfig::expires_in(
                    std::time::Duration::from_secs(expires_in as u64)
                )?
            )
            .await?;
        
        Ok(Some(presigned.uri().to_string()))
    }
    
    // ... implement other methods
}
```

## Testing Plan

### Unit Tests

- `FileSystemStorage`: store, retrieve, delete operations
- `PostgresStorage`: small file handling
- Hash computation and deduplication
- Path generation and collision handling

### Integration Tests

- End-to-end narrative execution with media
- Migration script with sample data
- Storage backend switching
- Content deduplication

### Performance Tests

- Compare query performance before/after migration
- Test large file handling (100MB+ videos)
- Concurrent access patterns
- Cache effectiveness

## Rollback Plan

If issues arise, we can roll back:

1. **Before removing old columns**: Data exists in both places, just switch back to old code
2. **After removing old columns**: Restore from backup and re-run migration
3. **Feature flag**: Keep old behavior behind `legacy-storage` feature flag

## Success Metrics

- ✅ Database size reduced by >90% (binary data removed)
- ✅ Query performance improved (metadata queries faster)
- ✅ Video support enabled (files >1GB)
- ✅ Backup/restore time reduced significantly
- ✅ Zero data loss during migration
- ✅ All existing tests pass

## Timeline

- **Step 1-2**: Storage abstraction + filesystem (4-6 days)
- **Step 3**: Database migration (1 day)
- **Step 4**: Repository layer updates (2-3 days)
- **Step 5**: Data migration script (1-2 days)
- **Step 6**: CLI/config updates (1 day)
- **Testing & validation**: (2-3 days)
- **Step 7**: Remove old columns (1 day)

**Total**: ~12-17 days for complete migration

**Step 8** (S3): Future phase, 3-5 days additional

## Open Questions

1. **URL handling**: Should we fetch and store URLs, or keep as references?
2. **Caching strategy**: Add Redis layer for hot media?
3. **Cleanup**: When to garbage collect orphaned media?
4. **Access control**: Need signed URLs or ACLs?
5. **Thumbnails**: Generate and store thumbnails for images/videos?
6. **Transcoding**: Support multiple video formats/qualities?

## Dependencies

```toml
[dependencies]
sha2 = "0.10"          # For content hashing
tokio = { version = "1", features = ["fs"] }  # Async file I/O
uuid = { version = "1", features = ["v4"] }    # Media reference IDs
base64 = "0.21"        # Base64 decoding

# Optional
aws-sdk-s3 = { version = "1.0", optional = true }
```

## References

- PostgreSQL BYTEA documentation: <https://www.postgresql.org/docs/current/datatype-binary.html>
- Content-addressable storage: <https://en.wikipedia.org/wiki/Content-addressable_storage>
- S3 best practices: <https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html>
