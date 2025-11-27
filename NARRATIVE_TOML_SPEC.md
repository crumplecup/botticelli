# Narrative TOML Specification

This document defines the TOML configuration format for multi-act narrative execution.

## Overview

A narrative TOML file consists of several sections:

**Core sections** (required):
1. `[narrative]` - Metadata about the narrative
2. `[toc]` - Table of contents defining execution order
3. `[acts]` - Act definitions with prompts and optional configurations

**Resource sections** (optional - for friendly syntax):
4. `[bots]` - Bot command definitions that can be referenced by name
5. `[tables]` - Table query definitions that can be referenced by name
6. `[media]` - Media file definitions that can be referenced by name

The resource sections enable a **friendly syntax** where you define resources once and reference them by name, reducing boilerplate and making narratives easier to read and maintain.

## Basic Structure

**Minimal narrative** (classic syntax):
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

**With friendly syntax** (define once, reference many times):
```toml
[narrative]
name = "bot_demo"
description = "Demonstrate friendly syntax"

[toc]
order = ["fetch_stats", "analyze"]

# Define bot command once
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

# Define media once
[media.chart]
file = "./chart.png"

# Reference them in acts
[acts]
fetch_stats = "bots.get_stats"
analyze = ["media.chart", "Analyze this chart with {{fetch_stats}}"]
```

## Friendly Syntax (Recommended)

The friendly syntax allows you to **define resources once and reference them by name**, dramatically reducing boilerplate and making narratives easier to read and maintain.

### Quick Example

**Without friendly syntax** (verbose):
```toml
[acts.fetch_stats]
[[acts.fetch_stats.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "123" }

[acts.analyze]
[[acts.analyze.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
args = { guild_id = "123" }

[[acts.analyze.input]]
type = "text"
content = "What do you think?"
```

**With friendly syntax** (concise):
```toml
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "123"

[acts]
fetch_stats = "bots.get_stats"
analyze = ["bots.get_stats", "What do you think?"]
```

### Resource Types

#### `[bots.name]` - Bot Command Definitions

Define bot commands that can be referenced in acts:

```toml
[bots.get_server_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

[bots.list_channels]
platform = "discord"
command = "channels.list"
guild_id = "1234567890"

[bots.send_message]
platform = "discord"
command = "channels.create_message"
guild_id = "1234567890"
channel_id = "9876543210"
content = "Hello from Botticelli!"

# Example: Drop large bot command results from history
[bots.fetch_all_members]
platform = "discord"
command = "members.list"
guild_id = "1234567890"
history_retention = "drop"  # Remove after processing to save tokens
```

Fields:
- `platform` (string): Platform name (e.g., "discord", "slack")
- `command` (string): Command to execute (e.g., "server.get_stats")
- `history_retention` (string, optional): Controls conversation history retention (default: "full")
  - `"full"` - Retain entire result (default)
  - `"summary"` - Replace with `[Bot command: platform.command]` after processing
  - `"drop"` - Remove from history after processing
- Additional fields are passed as command arguments (flattened)

Reference in acts: `"bots.get_server_stats"`

**Template Injection:**

Bot command arguments can use template syntax to inject outputs from previous acts:

```toml
[bots.send_message]
platform = "discord"
command = "channels.create_message"
channel_id = "123456"
content = "{{select_best}}"  # Inject output from "select_best" act

[acts]
select_best = "Choose the best option from these candidates..."
send_message = ["bots.send_message"]  # Will use output from select_best
```

Template placeholders:
- `{{previous}}` - Output from immediately previous act
- `{{act_name}}` - Output from specific named act

Templates are resolved when the bot command executes, allowing dynamic content based on LLM responses.

**Available Discord Commands:**

Read operations (no security policy required):
- `server.get` - Get guild information
- `server.get_stats` - Get guild statistics
- `channels.list` - List all channels
- `channels.get` - Get channel details
- `roles.list` - List all roles
- `members.list` - List guild members
- `members.get` - Get member details

Write operations (require security policy):
- `channels.create` - Create a new channel
- `channels.create_message` - Send a message to a channel
- `channels.edit` - Edit channel properties
- `channels.delete` - Delete a channel
- `roles.create` - Create a new role
- `roles.edit` - Edit role properties
- `roles.delete` - Delete a role

**Security Note:** Write operations require a security policy to be defined in the narrative. See [Security Policies](#security-policies) for details.

#### `[tables.name]` - Table Query Definitions

Define database table queries that can be referenced in acts:

```toml
[tables.recent_posts]
table_name = "social_posts_20241120"
where_clause = "status = 'approved'"
limit = 50
format = "markdown"
alias = "approved_posts"

[tables.user_stats]
table_name = "user_activity"
columns = ["user_id", "post_count", "last_active"]
order_by = "post_count DESC"
limit = 100
format = "json"
alias = "top_users"

# Example: Optimize token usage in multi-act narratives
[tables.large_dataset]
table_name = "analytics_data"
limit = 1000
format = "json"
history_retention = "summary"  # Replace with [Table: analytics_data, 1000 rows] after processing
```

Fields:
- `table_name` (string, required): Name of the table to query
- `columns` (array of strings, optional): Specific columns to select (default: all)
- `where_clause` (string, optional): WHERE clause for filtering (use `where_clause` or `where`)
- `limit` (integer, optional): Maximum number of rows (default: 10)
- `offset` (integer, optional): Offset for pagination
- `order_by` (string, optional): ORDER BY clause for sorting results
- `format` (string, optional): Output format - "json", "markdown", or "csv" (default: "json")
- `alias` (string, optional): Alias for referencing results in future acts (e.g., `{{alias}}`)
- `sample` (integer, optional): Random sample N rows (mutually exclusive with order_by)
- `history_retention` (string, optional): Controls how this input is retained in conversation history (default: "full")
  - `"full"` - Retain entire input unchanged (default)
  - `"summary"` - Replace with concise summary like `[Table: name, N rows]` after processing
  - `"drop"` - Remove from conversation history after processing

Reference in acts: `"tables.recent_posts"`

**Security Notes**:
- Table and column names are validated (alphanumeric + underscore only)
- WHERE clauses are sanitized to prevent SQL injection
- Row limits are enforced to prevent excessive data transfer

**History Retention for Token Optimization**:

In multi-act narratives, large table inputs can cause token explosion as they're re-sent with every subsequent act. Use `history_retention` to control this behavior:

- **Use `"full"` when**:
  - Single-act narratives (no subsequent acts)
  - Small result sets (< 5KB)
  - Subsequent acts need to re-examine the data

- **Use `"summary"` when**:
  - Multi-act narratives with large data
  - Subsequent acts only need the decision/result, not raw data
  - Token optimization is important (can reduce usage by 80%+)

- **Use `"drop"` when**:
  - Input is truly one-time (never referenced again)
  - Maximum token savings needed
  - Data was only used for initial decision

**Auto-summarization**: Inputs exceeding 10KB are automatically summarized even with `history_retention = "full"` to prevent token overflow.

#### `[media.name]` - Media File Definitions

Define media files (images, audio, video, documents) with automatic MIME type inference:

```toml
[media.logo]
file = "./images/logo.png"
# MIME type automatically inferred as "image/png"

[media.screenshot]
url = "https://example.com/dashboard.jpg"
# MIME type automatically inferred as "image/jpeg"

[media.interview]
file = "./audio/interview.mp3"
# MIME type automatically inferred as "audio/mp3"

[media.report]
file = "./docs/report.pdf"
# MIME type automatically inferred as "application/pdf"

[media.custom]
file = "./image.webp"
mime = "image/webp"  # Can override inferred type
```

Fields:
- Source (one required): `file`, `url`, or `base64`
- `mime` (string, optional): MIME type (auto-inferred from extension if not provided)
- `filename` (string, optional): Filename for documents

Supported extensions with auto-inference:
- **Images**: .png, .jpg, .jpeg, .gif, .webp
- **Audio**: .mp3, .wav, .ogg
- **Video**: .mp4, .webm, .avi, .mov
- **Documents**: .pdf, .txt, .md, .json

Reference in acts: `"media.logo"`

### Referencing Resources in Acts

#### Single Resource Reference

```toml
[acts]
# Reference a bot command
fetch_data = "bots.get_stats"

# Reference a table
load_data = "tables.recent_posts"

# Reference media
analyze_image = "media.logo"

# Reference another narrative (runs it and uses its output)
run_preprocessing = "narrative:data_preparation"

# Plain text (unchanged)
discuss = "What do you think about our stats?"
```

#### Multiple Inputs (Array Syntax)

Combine resources and text in a single act:

```toml
[acts]
comprehensive_analysis = [
    "bots.get_stats",        # Bot command
    "media.screenshot",       # Image
    "tables.recent_posts",    # Table data
    "Compare all this data"   # Text prompt
]
```

#### Mixing Friendly and Verbose Syntax

You can use friendly syntax for simple cases and drop down to verbose syntax when you need fine control:

```toml
[media.logo]
file = "./logo.png"

[acts]
# Friendly syntax for simple act
simple = "media.logo"

# Verbose syntax for complex act with overrides
[acts.complex]
model = "gemini-2.0-flash-exp"
temperature = 0.3
max_tokens = 2000

[[acts.complex.input]]
ref = "media.logo"  # Reference defined media

[[acts.complex.input]]
type = "text"
content = "Detailed analysis prompt"
```

Note: Use `ref = "resource.name"` in verbose syntax to reference friendly resources.

### Benefits of Friendly Syntax

- **DRY (Don't Repeat Yourself)**: Define once, use many times
- **Less boilerplate**: ~30% fewer lines of code
- **Easier to read**: Clear separation of resources and logic
- **Easier to refactor**: Change definition in one place
- **Sensible defaults**: MIME types inferred automatically
- **100% backward compatible**: Existing narratives work unchanged

## Section Reference

### `[narrative]` - Metadata

Required fields:
- `name` (string): Unique identifier for this narrative
- `description` (string): Human-readable description

Optional fields:
- `template` (string): Name of database table to use as schema source for content generation (see [Content Generation](#content-generation))
- `skip_content_generation` (boolean): Skip automatic content generation to custom tables (default: `false`)
- `model` (string): Default model for all acts in this narrative (can be overridden per-act)
- `temperature` (float): Default temperature for all acts (range: 0.0-1.0, can be overridden per-act)
- `max_tokens` (integer): Default max_tokens for all acts (can be overridden per-act)

**Configuration hierarchy:** Act-level overrides take precedence over narrative-level defaults, which take precedence over executor defaults.

### `[toc]` - Table of Contents

Required fields:
- `order` (array of strings): Act names in execution order

Optional fields:
- `carousel` (integer): Number of times to repeat the entire narrative (default: 1)

Acts execute sequentially in this order, with each act seeing previous outputs as conversation context.

#### Carousel Mode

When `carousel` is specified, the entire narrative loops the specified number of times:

```toml
[toc]
order = ["generate", "critique", "refine"]
carousel = 3  # Run the full narrative 3 times
```

**Use cases:**
- Batch content generation (e.g., generate 10 posts per loop, run 3 loops = 30 posts)
- Iterative refinement with fresh context each loop
- Building up a corpus of related content

**How it works:**
- Each loop sees the conversation context from the current loop only (not previous loops)
- Content generation creates rows for all loops in the same output table
- Budget-aware execution ensures rate limits are respected across all loops

### `[acts]` - Act Definitions

Acts can be defined in several ways, from simple to complex:

#### 1. Simple Text Acts

```toml
[acts]
act_name = "Text prompt goes here"
```

This creates an act with:
- Single text input
- No model override (uses executor default)
- No temperature/max_tokens overrides

#### 2. Resource References (Friendly Syntax)

Reference pre-defined resources:

```toml
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "123"

[media.logo]
file = "./logo.png"

[acts]
# Single resource
fetch_data = "bots.get_stats"
analyze_logo = "media.logo"
```

#### 3. Multiple Inputs (Array Syntax)

Combine multiple resources and/or text:

```toml
[acts]
analyze = [
    "bots.get_stats",
    "media.screenshot",
    "Compare the screenshot with the stats"
]

multi_input = [
    "tables.recent_posts",
    "Based on these posts, recommend new content"
]
```

#### 4. Narrative Composition

Run other narratives as steps in your narrative using the `narrative:` prefix:

```toml
[toc]
order = ["prepare_data", "analyze_data", "publish_results"]

[acts]
# Run the data_preparation narrative first
prepare_data = "narrative:data_preparation"

# Use the output from data_preparation in subsequent steps
analyze_data = ["narrative:data_preparation", "Analyze this prepared data"]

# Run multiple narratives in sequence
process_all = [
    "narrative:collect_data",
    "narrative:clean_data", 
    "narrative:analyze_data"
]
```

**How it works:**
- The referenced narrative is loaded and executed completely
- Its final output becomes the input for the current act
- Narratives are resolved relative to the calling narrative's directory
- Use the base filename without `.toml` extension (e.g., `data_preparation` for `data_preparation.toml`)

**Use cases:**
- Breaking complex workflows into reusable components
- Creating pipelines where each narrative handles one responsibility
- Composing modular narratives for maintainability

#### 5. Structured Acts (Full Configuration)

Use TOML's array-of-tables syntax (`[[...]]`) for full control:

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

### Example 5: Friendly Syntax - Bot Commands and Media

```toml
[narrative]
name = "discord_content_analysis"
description = "Analyze Discord server with friendly syntax"

[toc]
order = ["fetch_stats", "fetch_channels", "analyze_screenshot", "recommend"]

# Define bot commands once
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

[bots.get_channels]
platform = "discord"
command = "channels.list"
guild_id = "1234567890"

# Define media once
[media.dashboard]
file = "./screenshots/dashboard.png"
# MIME type auto-inferred as image/png

# Simple references in acts
[acts]
fetch_stats = "bots.get_stats"
fetch_channels = "bots.get_channels"

# Multi-input act with array syntax
analyze_screenshot = [
    "media.dashboard",
    "bots.get_stats",
    "Compare the dashboard screenshot with the actual stats"
]

# Mix bot commands with text
recommend = [
    "bots.get_stats",
    "bots.get_channels",
    "Based on our server stats and channel activity, recommend content strategy"
]
```

### Example 6: Friendly Syntax - Table References

```toml
[narrative]
name = "content_analysis"
description = "Analyze previously generated content"

[toc]
order = ["load_posts", "load_metrics", "analyze_trends", "recommend"]

# Define table queries once
[tables.recent_posts]
table_name = "social_posts_20241120_153045"
where_clause = "status = 'approved'"
limit = 50
format = "markdown"
alias = "approved_content"

[tables.engagement_metrics]
table_name = "post_metrics"
columns = ["post_id", "views", "reactions", "shares"]
order_by = "views DESC"
limit = 20
format = "json"
alias = "top_posts"

# Reference tables in acts
[acts]
load_posts = "tables.recent_posts"
load_metrics = "tables.engagement_metrics"

# Combine table data with text
analyze_trends = [
    "tables.recent_posts",
    "tables.engagement_metrics",
    "Identify trends in our top-performing content"
]

recommend = """
Based on the analysis from {{analyze_trends}}, recommend 5 new content ideas
that build on successful themes.
"""
```

### Example 7: Comprehensive - All Friendly Syntax Features

```toml
[narrative]
name = "comprehensive_friendly"
description = "Demonstrates all friendly syntax features"

[toc]
order = ["fetch_data", "load_historical", "analyze_visual", "recommend"]

# Bot commands
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "1234567890"

# Table queries
[tables.previous_posts]
table_name = "approved_posts_2024"
limit = 30
format = "markdown"

# Media files with auto-inferred MIME types
[media.logo]
file = "./images/logo.png"

[media.chart]
url = "https://example.com/engagement_chart.jpg"

[media.report]
file = "./docs/q3_report.pdf"

# Acts using friendly syntax
[acts]
# Single resource references
fetch_data = "bots.get_stats"
load_historical = "tables.previous_posts"

# Multi-input with array syntax
analyze_visual = [
    "media.chart",
    "media.logo",
    "bots.get_stats",
    "Analyze our branding and engagement data"
]

# Mix everything together
recommend = [
    "tables.previous_posts",
    "media.report",
    "bots.get_stats",
    "Based on historical performance and current stats, create a content strategy"
]
```

### Example 8: Mixing Friendly and Verbose Syntax

You can start with friendly syntax and drop down to verbose when you need fine control:

```toml
[narrative]
name = "mixed_syntax"
description = "Mix friendly and verbose syntax"

[toc]
order = ["simple", "complex"]

# Define resources
[bots.get_stats]
platform = "discord"
command = "server.get_stats"
guild_id = "123"

[media.logo]
file = "./logo.png"

[acts]
# Friendly syntax for simple act
simple = ["bots.get_stats", "media.logo", "Quick analysis"]

# Verbose syntax for complex act with overrides
[acts.complex]
model = "gemini-2.0-flash-exp"
temperature = 0.3
max_tokens = 2000

# Can reference friendly resources with 'ref'
[[acts.complex.input]]
ref = "bots.get_stats"

[[acts.complex.input]]
ref = "media.logo"

# And mix with inline definitions
[[acts.complex.input]]
type = "text"
content = "Provide detailed technical analysis"
```

## Security Policies

Bot write operations (create, edit, delete) require explicit security policies to prevent unauthorized actions. Define policies in your narrative:

```toml
[narrative]
name = "setup_discord_server"
description = "Automated server setup"

[security.policy]
# Define which write operations are allowed
allowed_commands = [
    "channels.create",
    "channels.create_message",
    "roles.create"
]

# Optional: Require human confirmation for sensitive operations
require_confirmation = ["channels.delete", "roles.delete"]

# Optional: Restrict operations to specific resources
[security.constraints]
guild_id = "1234567890"  # Only allow operations on this guild
channel_prefix = "bot-"  # Only create channels starting with "bot-"
```

**Policy Enforcement:**
- If a narrative attempts a write operation without proper policy, execution fails with a security error
- Read operations never require security policies
- Policies are validated before any bot commands execute

## Best Practices

1. **Use Friendly Syntax**: Start with friendly syntax (`[bots]`, `[tables]`, `[media]`) for cleaner, more maintainable narratives. Drop down to verbose syntax only when you need fine control.

2. **Context Passing**: Each act sees all previous outputs. Design prompts accordingly.

3. **Temperature Guidelines**:
   - 0.0-0.3: Analytical, factual, deterministic tasks
   - 0.4-0.7: Balanced tasks
   - 0.8-1.0: Creative, exploratory tasks

4. **Define Resources Once**: Use `[bots]`, `[tables]`, and `[media]` sections to define resources once and reference them multiple times. This follows the DRY principle and makes refactoring easier.

5. **Let MIME Types Be Inferred**: For media files, let Botticelli infer MIME types from file extensions (.png, .jpg, .mp3, .pdf, etc.). Only specify `mime` explicitly when you need to override the default.

6. **Model Selection**:
   - Vision tasks: `gemini-pro-vision`, `gpt-4-vision-preview`
   - Audio transcription: `whisper-large-v3`
   - Document analysis: `claude-3-opus-20240229`
   - Creative writing: `gpt-4`, `claude-3-opus-20240229`
   - Fast tasks: `gpt-3.5-turbo`, `claude-3-haiku-20240307`
   - Cost-effective batch generation: `gemini-2.0-flash-lite`, `gemini-1.5-flash`

7. **Mixing Formats**: You can mix friendly and verbose syntax in the same narrative. Start simple and add complexity only where needed.

8. **Act Naming**: Use descriptive act names that indicate their purpose (e.g., `fetch_stats`, `analyze_visual`, `recommend_strategy`).

9. **Array Syntax for Multi-Input**: Use array syntax `act = ["resource1", "resource2", "text"]` for acts with multiple inputs. It's much cleaner than verbose table syntax.

10. **Carousel for Batch Generation**: Use `carousel` in `[toc]` to generate large batches of content efficiently while respecting rate limits.

11. **Security First**: Always define security policies for narratives that perform write operations. Test on development servers first.

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
