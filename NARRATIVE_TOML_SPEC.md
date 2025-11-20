# Narrative TOML Specification

This document defines the TOML configuration format for multi-act narrative execution.

## Overview

A narrative TOML file consists of three main sections:
1. `[narrative]` - Metadata about the narrative
2. `[toc]` - Table of contents defining execution order
3. `[acts]` - Act definitions with prompts and optional configurations

## Basic Structure

```toml
[narrative]
name = "narrative_name"
description = "What this narrative does"

[toc]
order = ["act1", "act2", "act3"]

[acts]
act1 = "Simple text prompt"
act2 = "Another text prompt"
```

## Section Reference

### `[narrative]` - Metadata

Required fields:
- `name` (string): Unique identifier for this narrative
- `description` (string): Human-readable description

Optional fields:
- `template` (string): Name of database table to use as schema source for content generation (see [Content Generation](#content-generation))
- `skip_content_generation` (boolean): Skip automatic content generation to custom tables (default: `false`)

### `[toc]` - Table of Contents

Required fields:
- `order` (array of strings): Act names in execution order

Acts execute sequentially in this order, with each act seeing previous outputs as conversation context.

### `[acts]` - Act Definitions

Acts can be defined in two ways:

#### Simple Text Acts (Backward Compatible)

```toml
[acts]
act_name = "Text prompt goes here"
```

This creates an act with:
- Single text input
- No model override (uses executor default)
- No temperature/max_tokens overrides

#### Structured Acts (Full Configuration)

Use TOML's array-of-tables syntax (`[[...]]`) for multimodal inputs:

```toml
[acts.act_name]
model = "..."       # Optional: model override
temperature = 0.7   # Optional: temperature override (0.0 - 1.0)
max_tokens = 1000   # Optional: max tokens override

[[acts.act_name.input]]
type = "text"
content = "First input"

[[acts.act_name.input]]
type = "image"
mime = "image/png"
url = "https://example.com/image.png"
```

## Input Types

Each input is defined as an array-of-tables entry using `[[acts.act_name.input]]`.

### Text Input

```toml
[[acts.act_name.input]]
type = "text"
content = "The text content"
```

### Image Input

```toml
# From URL
[[acts.act_name.input]]
type = "image"
mime = "image/png"
url = "https://example.com/image.png"

# From base64
[[acts.act_name.input]]
type = "image"
mime = "image/jpeg"
base64 = "iVBORw0KGgo..."

# From file path
[[acts.act_name.input]]
type = "image"
mime = "image/png"
file = "/path/to/image.png"
```

Supported MIME types: `image/png`, `image/jpeg`, `image/webp`, `image/gif`

### Audio Input

```toml
[[acts.act_name.input]]
type = "audio"
mime = "audio/mp3"
url = "https://example.com/audio.mp3"
```

Supported MIME types: `audio/mp3`, `audio/wav`, `audio/ogg`, `audio/webm`

### Video Input

```toml
[[acts.act_name.input]]
type = "video"
mime = "video/mp4"
url = "https://example.com/video.mp4"
```

Supported MIME types: `video/mp4`, `video/webm`, `video/avi`, `video/mov`

### Document Input

```toml
[[acts.act_name.input]]
type = "document"
mime = "application/pdf"
url = "https://example.com/doc.pdf"
filename = "doc.pdf"  # Optional
```

Supported MIME types: `application/pdf`, `text/plain`, `text/markdown`, `application/json`

## Source Types

Media sources are specified by including one of these fields:

- `url = "https://..."` - Fetch from URL
- `base64 = "..."` - Embedded base64 data
- `file = "/path/..."` - Load from local file

The source type is inferred from which field is present.

## Configuration Overrides

### Model Override

Specify which LLM model to use for this act:

```toml
[acts.vision_task]
inputs = [...]
model = "gemini-pro-vision"
```

Common values:
- `"gpt-4"`, `"gpt-4-turbo"`, `"gpt-3.5-turbo"`
- `"claude-3-opus-20240229"`, `"claude-3-5-sonnet-20241022"`
- `"gemini-pro"`, `"gemini-pro-vision"`

### Temperature Override

Controls randomness/creativity (0.0 = deterministic, 1.0 = creative):

```toml
[acts.creative_task]
inputs = [...]
temperature = 0.9  # High creativity
```

```toml
[acts.analytical_task]
inputs = [...]
temperature = 0.2  # Low randomness, more focused
```

### Max Tokens Override

Limits the response length:

```toml
[acts.brief_summary]
inputs = [...]
max_tokens = 200  # Short response
```

## Complete Examples

### Example 1: Simple Text-Only Narrative (mint.toml style)

```toml
[narrative]
name = "mint"
description = "Generate social media content"

[toc]
order = ["act1", "act2", "act3"]

[acts]
act1 = "Create social media posts for MINT homeless shelter"
act2 = "Critique the posts for quality and impact"
act3 = "Improve the posts based on critique"
```

### Example 2: Vision Analysis with Model Override

```toml
[narrative]
name = "logo_review"
description = "Analyze a logo design"

[toc]
order = ["analyze", "suggest_improvements"]

[acts.analyze]
model = "gemini-pro-vision"
temperature = 0.3

[[acts.analyze.input]]
type = "text"
content = "Analyze this logo for visual appeal, memorability, and brand alignment"

[[acts.analyze.input]]
type = "image"
mime = "image/png"
url = "https://example.com/logo.png"

[acts.suggest_improvements]
temperature = 0.7

[[acts.suggest_improvements.input]]
type = "text"
content = "Suggest 5 specific improvements to make this logo more effective"
```

### Example 3: Multi-Modal Act

```toml
[acts.comprehensive_analysis]
model = "claude-3-opus-20240229"
temperature = 0.3
max_tokens = 2000

[[acts.comprehensive_analysis.input]]
type = "text"
content = "Analyze these materials together"

[[acts.comprehensive_analysis.input]]
type = "image"
mime = "image/png"
url = "https://example.com/chart.png"

[[acts.comprehensive_analysis.input]]
type = "document"
mime = "application/pdf"
url = "https://example.com/report.pdf"

[[acts.comprehensive_analysis.input]]
type = "audio"
mime = "audio/mp3"
url = "https://example.com/interview.mp3"
```

### Example 4: Per-Act Model Selection

```toml
[narrative]
name = "multi_model_analysis"
description = "Use different models for different strengths"

[toc]
order = ["creative", "analytical", "technical"]

# GPT-4 for creative tasks
[acts.creative]
model = "gpt-4"
temperature = 0.9

[[acts.creative.input]]
type = "text"
content = "Brainstorm 10 innovative features"

# Claude for analytical tasks
[acts.analytical]
model = "claude-3-opus-20240229"
temperature = 0.3

[[acts.analytical.input]]
type = "text"
content = "Analyze the feasibility of each feature"

# Gemini for technical tasks
[acts.technical]
model = "gemini-pro"
temperature = 0.2

[[acts.technical.input]]
type = "text"
content = "Create a technical implementation plan"
```

## Best Practices

1. **Context Passing**: Each act sees all previous outputs. Design prompts accordingly.

2. **Temperature Guidelines**:
   - 0.0-0.3: Analytical, factual, deterministic tasks
   - 0.4-0.7: Balanced tasks
   - 0.8-1.0: Creative, exploratory tasks

3. **Model Selection**:
   - Vision tasks: `gemini-pro-vision`, `gpt-4-vision-preview`
   - Audio transcription: `whisper-large-v3`
   - Document analysis: `claude-3-opus-20240229`
   - Creative writing: `gpt-4`, `claude-3-opus-20240229`
   - Fast tasks: `gpt-3.5-turbo`, `claude-3-haiku-20240307`

4. **Mixing Formats**: You can mix simple and structured acts in the same narrative:
   ```toml
   [acts]
   simple_act = "Just text"

   [acts.complex_act]
   model = "gpt-4"

   [[acts.complex_act.input]]
   type = "text"
   content = "Complex prompt"
   ```

5. **Act Naming**: Use descriptive act names that indicate their purpose.

6. **TOML Syntax**: Use array-of-tables `[[acts.act_name.input]]` for multiple inputs. This is idiomatic TOML and much more readable than inline tables.

## Content Generation

Botticelli can automatically generate structured content into custom database tables for review and approval workflows. This is useful for creating large batches of reviewable content like social media posts, Discord server content, product descriptions, etc.

### Template-Based Content Generation

When you specify a `template` field in the narrative metadata, Botticelli uses an existing database table as the schema template:

```toml
[narrative]
name = "generate_discord_guilds"
description = "Generate Discord server configurations"
template = "discord_guilds"  # Use discord_guilds table schema
```

**How it works:**
1. Botticelli queries the specified table's schema (column names and types)
2. Injects schema information into your prompts (see [Template Injection](#template-injection))
3. Creates a new custom table named `{narrative_name}_{timestamp}` with the same schema
4. Extracts JSON from LLM responses
5. Inserts generated content with automatic metadata columns:
   - `source_narrative` - The narrative name
   - `source_act` - Which act generated this row
   - `generation_model` - Which LLM model was used
   - `status` - Workflow status (e.g., "draft", "reviewed", "approved")
   - `generated_at` - Timestamp of generation

**Example:**

```toml
[narrative]
name = "generate_social_posts"
description = "Generate social media posts for review"
template = "social_media_posts"  # Reuse existing schema

[toc]
order = ["generate"]

[acts]
generate = """
Generate 10 engaging social media posts about Rust programming.
Each post should have: title, body, hashtags, platform.
"""
```

This creates a table like `generate_social_posts_20241120_153045` with content ready for review in the TUI.

### Schema Inference Mode

If you omit the `template` field, Botticelli automatically infers the schema from the first JSON response:

```toml
[narrative]
name = "generate_product_descriptions"
description = "Generate product descriptions"
# No template specified - schema will be inferred

[toc]
order = ["generate"]

[acts]
generate = """
Generate 5 product descriptions in JSON format with these fields:
- name (string)
- description (text)
- price (numeric)
- category (string)
"""
```

**How inference works:**
1. First act generates JSON content
2. Botticelli analyzes the JSON structure
3. Infers appropriate SQL column types (INTEGER, TEXT, NUMERIC, etc.)
4. Creates a custom table with the inferred schema
5. Inserts the content with metadata columns

### Opting Out of Content Generation

To disable automatic content generation (for narratives that shouldn't create tables):

```toml
[narrative]
name = "simple_analysis"
description = "Analyze data without creating tables"
skip_content_generation = true  # Don't generate content tables

[toc]
order = ["analyze"]

[acts]
analyze = "Analyze this data and provide insights"
```

Use `skip_content_generation = true` when:
- You just want LLM responses without database storage
- The narrative is for one-off analysis, not batch content generation
- You're prototyping prompts and don't need persistence

### Template Injection

When using `template`, Botticelli automatically injects schema information into your prompts. For example, if your prompt is:

```toml
[acts]
generate = "Generate 5 Discord server configurations"
```

And you specified `template = "discord_guilds"`, the actual prompt sent to the LLM becomes:

```
Generate 5 Discord server configurations

Please provide your response as a JSON array where each object conforms to this schema:

{
  "id": BIGINT (Discord snowflake ID),
  "name": VARCHAR(100) (Guild name),
  "description": TEXT (Guild description),
  "owner_id": BIGINT (Owner user ID),
  "member_count": INTEGER (Total members),
  "verification_level": SMALLINT (0-4 security level),
  ...
}
```

This ensures the LLM generates content matching your database schema.

### Content Generation Workflow

1. **Generate**: Run narrative to create content table
2. **Review**: Use TUI (`botticelli tui {table_name}`) to review generated content
3. **Approve**: Mark items as "approved" in TUI
4. **Export**: Export approved content for publishing
5. **Iterate**: Refine narrative prompts based on quality

Example workflow:

```bash
# Generate content
botticelli run --narrative generate_posts.toml --save

# Review in TUI (shows all generated tables)
botticelli content last
# Output: generate_posts_20241120_153045

# Launch TUI for review
botticelli tui generate_posts_20241120_153045

# Export approved content
# (Future feature: export to JSON/CSV)
```

### Best Practices

1. **Use Templates for Consistency**: If generating multiple batches, use `template` to ensure consistent schema
2. **Request JSON Format**: Explicitly ask the LLM to respond in JSON format
3. **Specify Field Types**: In your prompts, indicate data types (e.g., "id should be a number")
4. **Generate in Batches**: Request 5-20 items per act for manageable review sessions
5. **Skip When Not Needed**: Use `skip_content_generation = true` for non-batch narratives

## See Also

- `CONTENT_GENERATION.md` - Detailed guide on content generation workflows
- `DISCORD_NARRATIVE.md` - Discord-specific schema templates and examples
- `narrations/mint.toml` - Simple text-only example
- `narrations/showcase.toml` - Comprehensive feature demonstration
