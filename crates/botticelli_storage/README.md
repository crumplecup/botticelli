# botticelli_storage

Content-addressable storage for media files in the Botticelli ecosystem.

## Overview

Provides filesystem-based content-addressable storage using SHA-256 hashing. Automatically deduplicates files and organizes them in a directory structure based on content hashes.

## Features

- **Content-addressable**: Files stored by SHA-256 hash
- **Automatic deduplication**: Same content = same file
- **Metadata tracking**: MIME types, original filenames
- **Thread-safe**: Safe concurrent access

## Usage

```rust
use botticelli_storage::{FileSystemStorage, MediaStorage};

// Create storage in a directory
let storage = FileSystemStorage::new("./media")?;

// Store content
let content = b"Hello, world!";
let reference = storage.store(
    content,
    "text/plain",
    Some("greeting.txt")
).await?;

// Retrieve content
let (data, metadata) = storage.retrieve(&reference).await?;
assert_eq!(data, content);
assert_eq!(metadata.mime_type, "text/plain");

// Delete content
storage.delete(&reference).await?;
```

## Directory Structure

```
./media/
├── ab/
│   └── cd/
│       └── abcd1234...5678  # Full SHA-256 hash as filename
└── metadata/
    └── ab/
        └── cd/
            └── abcd1234...5678.json  # Metadata file
```

## Dependencies

- `sha2` - SHA-256 hashing
- `serde` / `serde_json` - Metadata serialization
- `tokio` - Async filesystem operations

## Version

Current version: 0.2.0
