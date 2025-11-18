# botticelli_core

Core data structures and types for the Botticelli ecosystem.

## Overview

This crate provides the fundamental data types used throughout the Botticelli workspace for representing multimodal inputs, LLM outputs, and message structures.

## Core Types

### Input Types

Multimodal input data for LLM requests:

```rust
pub enum Input {
    Text(String),
    Image { data: Vec<u8>, mime_type: String },
    Audio { data: Vec<u8>, mime_type: String },
    Video { data: Vec<u8>, mime_type: String },
    Document { data: Vec<u8>, mime_type: String },
}
```

Supported MIME types:
- **Images**: `image/png`, `image/jpeg`, `image/webp`, `image/heic`, `image/heif`
- **Audio**: `audio/wav`, `audio/mp3`, `audio/aiff`, `audio/aac`, `audio/ogg`, `audio/flac`
- **Video**: `video/mp4`, `video/mpeg`, `video/mov`, `video/avi`, `video/x-flv`, `video/mpg`, `video/webm`, `video/wmv`, `video/3gpp`
- **Documents**: `application/pdf`, `text/plain`, `text/html`, `text/css`, `text/javascript`, `text/x-typescript`, `text/csv`, `text/markdown`, `text/x-python`, `text/x-java`, `text/x-c`, `text/x-c++`

### Output Types

LLM response data:

```rust
pub enum Output {
    Text(String),
    Image { data: Vec<u8>, mime_type: String },
    Audio { data: Vec<u8>, mime_type: String },
    Video { data: Vec<u8>, mime_type: String },
}
```

### Message Types

Structured conversation messages:

```rust
pub struct Message {
    pub role: MessageRole,
    pub parts: Vec<Input>,
}

pub enum MessageRole {
    User,
    Model,
    System,
}
```

### Request/Response Types

```rust
pub struct GenerateRequest {
    pub inputs: Vec<Input>,
    pub system_instruction: Option<String>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub model: String,
}

pub struct GenerateResponse {
    pub text: String,
    pub finish_reason: FinishReason,
}

pub enum FinishReason {
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Other(String),
}
```

## Usage Examples

### Creating Multimodal Inputs

```rust
use botticelli_core::Input;

// Text input
let text = Input::Text("Describe this image".to_string());

// Image input
let image_data = std::fs::read("photo.jpg")?;
let image = Input::Image {
    data: image_data,
    mime_type: "image/jpeg".to_string(),
};

// Combine multiple inputs
let inputs = vec![text, image];
```

### Building Requests

```rust
use botticelli_core::GenerateRequest;

let request = GenerateRequest {
    inputs: vec![Input::Text("Hello, world!".to_string())],
    system_instruction: Some("You are a helpful assistant.".to_string()),
    temperature: Some(0.7),
    max_output_tokens: Some(1024),
    model: "gemini-1.5-flash".to_string(),
};
```

### Working with Messages

```rust
use botticelli_core::{Message, MessageRole, Input};

let user_message = Message {
    role: MessageRole::User,
    parts: vec![Input::Text("What's the weather?".to_string())],
};

let model_message = Message {
    role: MessageRole::Model,
    parts: vec![Input::Text("I don't have access to weather data.".to_string())],
};
```

## Design Philosophy

### Pure Data Structures

This crate contains only data types with minimal logic. No I/O, no business logic, just types.

### Serialization

All types derive `Serialize` and `Deserialize` for easy persistence and transmission:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    // ...
}
```

### Type Safety

Rust's type system ensures correct usage at compile time:
- Can't accidentally pass audio data where video is expected
- Finish reasons are strongly typed, not strings
- Message roles are explicit enumerations

## Traits Implemented

All types implement standard traits where appropriate:
- `Debug` - For debugging output
- `Clone` - For easy copying
- `PartialEq`, `Eq` - For comparisons
- `Serialize`, `Deserialize` - For persistence
- `derive_more` traits where beneficial

## Dependencies

- `serde` - Serialization/deserialization
- `derive_more` - Derive macro utilities

## Version

Current version: 0.2.0

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
