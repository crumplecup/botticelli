# Friendly Narrative Syntax Design

## Overview

This document proposes a more user-friendly syntax for narratives that allows simple string references to pre-defined resources (bot commands, tables, media files) with sensible defaults, while still allowing users to drop down to verbose syntax when they need fine-grained control.

## Current Situation

**Verbose syntax (current)**:
```toml
[acts.fetch_stats]
[[acts.fetch_stats.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "1234567890" }

[[acts.fetch_stats.input]]
type = "image"
mime = "image/png"
url = "https://example.com/chart.png"

[[acts.fetch_stats.input]]
type = "text"
content = "Analyze this data"
```

**Problems**:
- Verbose and repetitive
- High barrier to entry for beginners
- Lots of boilerplate for simple cases
- TOML table syntax is confusing

## Proposed Friendly Syntax

### Example from bot_syntax.toml

```toml
[narrative]
name = "bot_syntax"
description = "Demonstrates bot syntax"

[toc]
order = ["fetch_stats", "discuss", "something_else"]

# Define bot commands once
[bots.get_discord_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

[bots.do_something_else]
platform = "discord"
command = "server.something_else"
user_id = "0987654321"

# Reference them by name in acts
[acts]
fetch_stats = "bots.get_discord_stats"
discuss = "Tell me what you think about our stats."
something_else = "bots.do_something_else"
```

**Benefits**:
- Bot commands defined once, referenced many times
- Simple string references instead of verbose tables
- Still clear and readable
- Easy to understand for beginners

### Extended Friendly Syntax Design

#### 1. Bot Commands

**Define once, use many times**:

```toml
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

[bots.get_channels]
platform = "discord"
command = "channels.list"
guild_id = "1234567890"

[acts]
fetch_stats = "bots.get_stats"
fetch_channels = "bots.get_channels"
analyze = "Compare these: {{fetch_stats}} and {{fetch_channels}}"
```

#### 2. Table References

**Define queries once**:

```toml
[tables.recent_posts]
table_name = "social_posts_20241120"
where = "status = 'approved'"
limit = 50
format = "markdown"

[tables.user_stats]
table_name = "user_activity"
columns = ["user_id", "post_count", "last_active"]
limit = 100

[acts]
load_posts = "tables.recent_posts"
load_users = "tables.user_stats"
analyze = "Analyze these posts and user activity patterns"
```

#### 3. Media Files

**Define media sources once with sensible defaults**:

```toml
[media.logo]
file = "./images/logo.png"
# Default mime = "image/png" inferred from extension

[media.chart]
url = "https://example.com/chart.jpg"
# Default mime = "image/jpeg" inferred from extension

[media.interview]
file = "./audio/interview.mp3"
# Default mime = "audio/mp3" inferred from extension

[media.report]
file = "./docs/report.pdf"
# Default mime = "application/pdf" inferred from extension

[acts]
analyze_logo = "media.logo"
describe_chart = "media.chart"
transcribe = "media.interview"
summarize_report = "media.report"
```

#### 4. Multi-Input Acts (Mixed References)

**Combine different resource types**:

```toml
[media.screenshot]
file = "./screenshot.png"

[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "123"

[acts]
# Multi-input act: bot command + media + text
analyze_dashboard = [
    "bots.get_stats",
    "media.screenshot",
    "Compare the stats with what you see in the screenshot"
]

# Or use structured syntax for fine control
[acts.detailed_analysis]
[[acts.detailed_analysis.input]]
ref = "bots.get_stats"

[[acts.detailed_analysis.input]]
ref = "media.screenshot"

[[acts.detailed_analysis.input]]
type = "text"
content = "Provide detailed analysis"
```

#### 5. Override Defaults When Needed

**Start simple, add details as needed**:

```toml
[media.logo]
file = "./logo.png"
# Uses default mime = "image/png"

[media.complex_image]
file = "./image.png"
mime = "image/png"  # Explicit override
# Could add other overrides like cache settings

[acts]
simple = "media.logo"  # Uses defaults
complex = "media.complex_image"  # Uses overrides
```

## Implementation Design

### 1. Root-Level Resource Sections

Add new optional sections at the root level:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TomlNarrativeFile {
    pub narrative: TomlNarrative,
    pub toc: TomlToc,
    pub acts: HashMap<String, TomlAct>,
    
    // NEW: Resource definition sections
    #[serde(default)]
    pub bots: HashMap<String, TomlBotDefinition>,
    
    #[serde(default)]
    pub tables: HashMap<String, TomlTableDefinition>,
    
    #[serde(default)]
    pub media: HashMap<String, TomlMediaDefinition>,
}
```

### 2. Resource Definition Structures

```rust
/// Bot command definition
#[derive(Debug, Clone, Deserialize)]
pub struct TomlBotDefinition {
    pub platform: String,
    pub command: String,
    #[serde(flatten)]
    pub args: HashMap<String, serde_json::Value>,
}

/// Table query definition
#[derive(Debug, Clone, Deserialize)]
pub struct TomlTableDefinition {
    pub table_name: String,
    pub columns: Option<Vec<String>>,
    #[serde(rename = "where")]
    pub where_clause: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub format: Option<String>,
    pub sample: Option<u32>,
}

/// Media source definition
#[derive(Debug, Clone, Deserialize)]
pub struct TomlMediaDefinition {
    pub url: Option<String>,
    pub file: Option<String>,
    pub base64: Option<String>,
    pub mime: Option<String>,  // Optional, will be inferred
    pub filename: Option<String>,  // For documents
}
```

### 3. Enhanced TomlAct to Support References

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlAct {
    /// Simple text act: `act_name = "prompt"`
    Simple(String),
    
    /// Array of references/inputs for multi-input acts
    Array(Vec<TomlActInput>),
    
    /// Structured act with configuration
    Structured(TomlActConfig),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlActInput {
    /// Reference to a resource: "bots.name" or "tables.name" or "media.name"
    Reference(String),
    
    /// Inline input definition (existing structured syntax)
    Inline(TomlInput),
}
```

### 4. Reference Resolution Logic

```rust
impl TomlNarrativeFile {
    /// Resolve a reference string to an Input
    fn resolve_reference(&self, reference: &str) -> Result<Input, String> {
        // Parse reference: "bots.name", "tables.name", "media.name"
        let parts: Vec<&str> = reference.split('.').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid reference format: {}", reference));
        }
        
        let (category, name) = (parts[0], parts[1]);
        
        match category {
            "bots" => {
                let bot_def = self.bots.get(name)
                    .ok_or_else(|| format!("Bot not found: {}", name))?;
                Ok(Input::BotCommand {
                    platform: bot_def.platform.clone(),
                    command: bot_def.command.clone(),
                    args: bot_def.args.clone(),
                    required: false,
                    cache_duration: None,
                })
            }
            
            "tables" => {
                let table_def = self.tables.get(name)
                    .ok_or_else(|| format!("Table not found: {}", name))?;
                Ok(Input::Table {
                    table_name: table_def.table_name.clone(),
                    columns: table_def.columns.clone(),
                    where_clause: table_def.where_clause.clone(),
                    limit: table_def.limit,
                    offset: table_def.offset,
                    order_by: table_def.order_by.clone(),
                    alias: Some(name.to_string()),
                    format: parse_table_format(&table_def.format),
                    sample: table_def.sample,
                })
            }
            
            "media" => {
                let media_def = self.media.get(name)
                    .ok_or_else(|| format!("Media not found: {}", name))?;
                
                // Detect media type and infer MIME if not provided
                let (media_type, source) = self.detect_media_type_and_source(media_def)?;
                let mime = media_def.mime.clone()
                    .or_else(|| infer_mime_type(&source));
                
                match media_type {
                    "image" => Ok(Input::Image { mime, source }),
                    "audio" => Ok(Input::Audio { mime, source }),
                    "video" => Ok(Input::Video { mime, source }),
                    "document" => Ok(Input::Document {
                        mime,
                        source,
                        filename: media_def.filename.clone(),
                    }),
                    _ => Err(format!("Unknown media type: {}", media_type)),
                }
            }
            
            _ => Err(format!("Unknown reference category: {}", category)),
        }
    }
    
    /// Detect media type from source
    fn detect_media_type_and_source(
        &self,
        media_def: &TomlMediaDefinition,
    ) -> Result<(&'static str, MediaSource), String> {
        let source = if let Some(url) = &media_def.url {
            MediaSource::Url(url.clone())
        } else if let Some(file) = &media_def.file {
            MediaSource::Binary(std::fs::read(file).map_err(|e| {
                format!("Failed to read file {}: {}", file, e)
            })?)
        } else if let Some(base64) = &media_def.base64 {
            MediaSource::Base64(base64.clone())
        } else {
            return Err("Media definition missing source (url, file, or base64)".to_string());
        };
        
        // Infer media type from extension or MIME
        let media_type = if let Some(mime) = &media_def.mime {
            match mime.split('/').next() {
                Some("image") => "image",
                Some("audio") => "audio",
                Some("video") => "video",
                Some("application") | Some("text") => "document",
                _ => return Err(format!("Cannot determine media type from MIME: {}", mime)),
            }
        } else {
            // Infer from file extension
            let path = media_def.file.as_ref()
                .or(media_def.url.as_ref())
                .ok_or("Cannot infer media type without file path or MIME")?;
            
            infer_media_type_from_extension(path)?
        };
        
        Ok((media_type, source))
    }
}

/// Infer MIME type from file extension
fn infer_mime_type(source: &MediaSource) -> Option<String> {
    let path = match source {
        MediaSource::Url(url) => url.as_str(),
        MediaSource::Binary(_) => return None,  // Can't infer from binary
        MediaSource::Base64(_) => return None,  // Can't infer from base64
    };
    
    let extension = std::path::Path::new(path)
        .extension()?
        .to_str()?
        .to_lowercase();
    
    Some(match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mp3",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        _ => return None,
    }.to_string())
}

/// Infer media type category from extension
fn infer_media_type_from_extension(path: &str) -> Result<&'static str, String> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| format!("Cannot determine file extension from: {}", path))?
        .to_lowercase();
    
    Ok(match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "image",
        "mp3" | "wav" | "ogg" | "webm" => "audio",
        "mp4" | "avi" | "mov" => "video",
        "pdf" | "txt" | "md" | "json" => "document",
        _ => return Err(format!("Unknown file extension: {}", extension)),
    })
}
```

### 5. Enhanced TomlAct Conversion

```rust
impl TomlAct {
    pub fn to_act_config(&self, narrative_file: &TomlNarrativeFile) -> Result<ActConfig, String> {
        match self {
            // Simple text prompt (unchanged)
            TomlAct::Simple(text) => {
                // Check if it's a reference
                if text.starts_with("bots.") || text.starts_with("tables.") || text.starts_with("media.") {
                    let input = narrative_file.resolve_reference(text)?;
                    Ok(ActConfig {
                        inputs: vec![input],
                        model: None,
                        temperature: None,
                        max_tokens: None,
                    })
                } else {
                    // Regular text prompt
                    if text.trim().is_empty() {
                        return Err("Act prompt cannot be empty".to_string());
                    }
                    Ok(ActConfig::from_text(text.clone()))
                }
            }
            
            // Array of references/inputs (NEW)
            TomlAct::Array(items) => {
                let mut inputs = Vec::new();
                for item in items {
                    match item {
                        TomlActInput::Reference(ref_str) => {
                            let input = narrative_file.resolve_reference(ref_str)?;
                            inputs.push(input);
                        }
                        TomlActInput::Inline(toml_input) => {
                            let input = toml_input.to_input()?;
                            inputs.push(input);
                        }
                    }
                }
                Ok(ActConfig {
                    inputs,
                    model: None,
                    temperature: None,
                    max_tokens: None,
                })
            }
            
            // Structured act (unchanged)
            TomlAct::Structured(config) => config.to_act_config(),
        }
    }
}
```

## Complete Example

### Friendly Syntax with All Features

```toml
[narrative]
name = "comprehensive_analysis"
description = "Demonstrate all friendly syntax features"

[toc]
order = ["fetch_data", "analyze_visual", "cross_reference", "recommend"]

# Bot command definitions
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

[bots.get_channels]
platform = "discord"
command = "channels.list"
guild_id = "1234567890"

# Table query definitions
[tables.recent_posts]
table_name = "social_posts_20241120"
where = "status = 'approved'"
limit = 50
format = "markdown"

[tables.engagement_metrics]
table_name = "post_metrics"
columns = ["post_id", "views", "reactions", "shares"]
order_by = "views DESC"
limit = 20

# Media file definitions
[media.logo]
file = "./images/logo.png"
# MIME inferred as image/png

[media.screenshot]
url = "https://example.com/dashboard.jpg"
# MIME inferred as image/jpeg

[media.report]
file = "./docs/annual_report.pdf"
# MIME inferred as application/pdf

# Acts using friendly syntax
[acts]
# Simple text
fetch_data = "bots.get_stats"

# Multiple inputs using array
analyze_visual = [
    "media.screenshot",
    "bots.get_stats",
    "Compare the visual dashboard with the actual stats"
]

# Table reference
cross_reference = "tables.recent_posts"

# Mix of everything
recommend = [
    "tables.engagement_metrics",
    "bots.get_channels",
    "Based on our top performing posts and channel activity, recommend content strategy"
]

# Can still drop down to verbose syntax when needed
[acts.complex_analysis]
model = "gemini-2.0-flash-exp"
temperature = 0.3
max_tokens = 2000

[[acts.complex_analysis.input]]
ref = "tables.recent_posts"

[[acts.complex_analysis.input]]
ref = "media.report"

[[acts.complex_analysis.input]]
type = "text"
content = "Provide detailed quarterly analysis"
```

### Equivalent Verbose Syntax (For Comparison)

```toml
[narrative]
name = "comprehensive_analysis"
description = "Same functionality, verbose syntax"

[toc]
order = ["fetch_data", "analyze_visual", "cross_reference", "recommend"]

[acts.fetch_data]
[[acts.fetch_data.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "1234567890" }

[acts.analyze_visual]
[[acts.analyze_visual.input]]
type = "image"
mime = "image/jpeg"
url = "https://example.com/dashboard.jpg"

[[acts.analyze_visual.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "1234567890" }

[[acts.analyze_visual.input]]
type = "text"
content = "Compare the visual dashboard with the actual stats"

[acts.cross_reference]
[[acts.cross_reference.input]]
type = "table"
table_name = "social_posts_20241120"
where = "status = 'approved'"
limit = 50
format = "markdown"

[acts.recommend]
[[acts.recommend.input]]
type = "table"
table_name = "post_metrics"
columns = ["post_id", "views", "reactions", "shares"]
order_by = "views DESC"
limit = 20

[[acts.recommend.input]]
type = "bot_command"
platform = "discord"
command = "channels.list"
args = { guild_id = "1234567890" }

[[acts.recommend.input]]
type = "text"
content = "Based on our top performing posts and channel activity, recommend content strategy"

[acts.complex_analysis]
model = "gemini-2.0-flash-exp"
temperature = 0.3
max_tokens = 2000

[[acts.complex_analysis.input]]
type = "table"
table_name = "social_posts_20241120"
where = "status = 'approved'"
limit = 50
format = "markdown"

[[acts.complex_analysis.input]]
type = "document"
mime = "application/pdf"
file = "./docs/annual_report.pdf"

[[acts.complex_analysis.input]]
type = "text"
content = "Provide detailed quarterly analysis"
```

**Lines of code**: Friendly = ~65 lines, Verbose = ~95 lines (32% reduction)

## Migration Path

### Backward Compatibility

The friendly syntax is 100% backward compatible:

1. **Existing narratives continue to work** - No breaking changes
2. **Opt-in** - Users can adopt friendly syntax gradually
3. **Mix and match** - Can use friendly syntax for some acts, verbose for others
4. **Drop-down escape hatch** - Can always use verbose syntax for full control

### Migration Examples

**Before (verbose)**:
```toml
[acts.analyze_logo]
[[acts.analyze_logo.input]]
type = "image"
mime = "image/png"
file = "./logo.png"
```

**After (friendly)**:
```toml
[media.logo]
file = "./logo.png"

[acts]
analyze_logo = "media.logo"
```

**Before (verbose bot command)**:
```toml
[acts.fetch_stats]
[[acts.fetch_stats.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "123" }
```

**After (friendly)**:
```toml
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "123"

[acts]
fetch_stats = "bots.get_stats"
```

## Benefits Summary

### For Beginners
- ✅ Less intimidating syntax
- ✅ Fewer concepts to learn
- ✅ Clear resource definitions
- ✅ Examples are easier to understand
- ✅ Sensible defaults reduce cognitive load

### For Power Users
- ✅ DRY (Don't Repeat Yourself) - define once, use many times
- ✅ Easier refactoring - change definition in one place
- ✅ Cleaner narratives - less boilerplate
- ✅ Still have full control when needed via verbose syntax
- ✅ Better organization - resources grouped by type

### For Maintainability
- ✅ Easier to read and understand narratives
- ✅ Resource definitions are self-documenting
- ✅ Reduces copy-paste errors
- ✅ Centralized configuration (bot commands, tables, media)
- ✅ Backward compatible - no breaking changes

## Implementation Checklist

### Phase 1: Foundation (Week 1)
- [ ] Add `bots`, `tables`, `media` sections to `TomlNarrativeFile`
- [ ] Create resource definition structs
- [ ] Implement reference resolution logic
- [ ] Add MIME type inference
- [ ] Unit tests for reference parsing

### Phase 2: Integration (Week 2)
- [ ] Update `TomlAct` to support references and arrays
- [ ] Implement `resolve_reference` method
- [ ] Update act conversion logic
- [ ] Integration tests with example narratives
- [ ] Backward compatibility tests

### Phase 3: Documentation (Week 3)
- [ ] Update `NARRATIVE_TOML_SPEC.md` with friendly syntax
- [ ] Create migration guide
- [ ] Add examples using friendly syntax
- [ ] Update Discord community server plan examples
- [ ] Create comparison guide (friendly vs verbose)

## Open Questions

1. **Naming**: Are `bots`, `tables`, `media` the right names? Alternatives: `commands`, `queries`, `files`?

2. **Reference syntax**: Is `bots.name` clear enough? Alternative: `@bots.name` or `$bots.name`?

3. **Error messages**: How to provide helpful errors when references are misspelled?

4. **Validation**: Should we validate that all referenced resources are defined?

5. **Scope**: Should resource definitions be per-narrative or could they be shared across files?

## Conclusion

The friendly syntax proposal from `bot_syntax.toml` is **absolutely viable** and provides significant benefits:

- **Dramatically improves user experience** for common cases
- **Maintains full backward compatibility** with existing narratives
- **Provides escape hatch** to verbose syntax when needed
- **Reduces code by ~30%** in typical narratives
- **Better organization** through centralized resource definitions

**Recommendation**: Implement this as an **enhancement to the existing syntax**, not a replacement. Users can choose the level of verbosity they need based on their use case and comfort level.
