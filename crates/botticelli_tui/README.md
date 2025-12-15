# Botticelli TUI

Terminal User Interface for Botticelli with interactive chat and MCP (Model Context Protocol) integration.

## Features

### Chat Mode (New!)

- 💬 **Interactive Chat** - Converse with Claude through a clean terminal interface
- 🔧 **MCP Tool Integration** - LLM can autonomously use narrative tools during conversation
- 📝 **Narrative Tools** - Create, validate, list, and load narratives
- 🎨 **Rich Formatting** - Color-coded messages and real-time tool execution visualization
- ⚡ **Async Execution** - Non-blocking tool calls with live UI updates

### Content Review Mode (Original)

- **Content review** - Browse pending, approved, and rejected content
- **Multi-table support** - Switch between different content tables
- **Keyboard navigation** - Vim-style keybindings
- **Status updates** - Approve, reject, or delete content
- **Real-time** - Auto-refreshes on changes

## Quick Start

### Prerequisites

- Rust toolchain installed
- Anthropic API key (for chat mode)
- PostgreSQL database (for content review mode)

### Setup

1. **Set your API key:**
   ```bash
   export ANTHROPIC_API_KEY=sk-ant-your-key-here
   ```

2. **Create narratives directory** (will be created automatically if missing):
   ```bash
   mkdir -p narratives
   ```

3. **Run the TUI:**
   ```bash
   cargo run --bin tui
   ```

## Chat Mode Usage

### Controls

- **Type** in the input box at the bottom
- **Press Enter** to send your message
- **Press Ctrl+C** or **q** to quit

### Available Tools

The LLM has access to the following MCP tools:

| Tool | Description | Parameters |
|------|-------------|------------|
| `echo` | Simple echo for testing | `message`: string |
| `create_narrative` | Create narrative from TOML | `toml_content`: string, `name?`: string |
| `validate_narrative` | Validate narrative structure | `toml_content`: string |
| `list_narratives` | List available narratives | `pattern?`: string (default: "*.toml") |
| `load_narrative` | Load narrative from file | `filename`: string |

### Example Prompts

Try these prompts to test the tool integration:

**Test echo:**
```
> Echo hello world
```

**List narratives:**
```
> List available narratives in the narratives directory
```

**Create narrative:**
```
> Create a simple narrative with 3 acts about space exploration
```

**Load and validate:**
```
> Load the space_exploration narrative and validate its structure
```

### Chat UI Layout

```
┌─────────────────────────────────────────┐
│             Chat Messages               │
│                                         │
│ You: Hello!                             │
│                                         │
│ 🔧 echo({"message": "Hello!"})          │
│ ✅ echo: Echo: Hello!                   │
│                                         │
│ Bot: I echoed your message!             │
│                                         │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│ Input: Type your message here...        │
└─────────────────────────────────────────┘
```

### Message Types

- **User messages** - Green "You:" prefix
- **Assistant messages** - Blue "Bot:" prefix
- **Tool calls** - Cyan 🔧 icon with tool name and arguments
- **Tool results** - ✅ (success) or ❌ (failure) with result text
- **Thinking** - Yellow 💭 icon for extended thinking mode

## Content Review Mode Usage

### Database Setup

```rust
use botticelli_tui::run_tui;
use diesel::pg::PgConnection;

let mut conn = PgConnection::establish(&database_url)?;
run_tui(&mut conn).await?;
```

### From CLI

```bash
# Run with database
DATABASE_URL=postgres://user:pass@localhost/db botticelli tui

# Review mode
botticelli tui --mode review
```

### Keyboard Controls

#### Navigation

- `↑/k` - Move up
- `↓/j` - Move down
- `PgUp` - Page up
- `PgDn` - Page down
- `Home/g` - Go to top
- `End/G` - Go to bottom

#### Actions

- `a` - Approve content
- `r` - Reject content
- `d` - Delete content
- `t` - Switch table
- `q/Esc` - Quit

#### Filters

- `1` - Show pending only
- `2` - Show approved only
- `3` - Show rejected only
- `0` - Show all

### Review Interface

```
┌─ Content Review (Table: social_posts) ─────────────────────────┐
│ Status: Pending  │  Count: 15                                   │
├──────────────────────────────────────────────────────────────────┤
│ ID   │ Narrative      │ Act      │ Status  │ Created          │
├──────┼────────────────┼──────────┼─────────┼──────────────────┤
│ 1    │ generate-post  │ draft    │ Pending │ 2024-11-18 10:30│
│ 2    │ generate-post  │ refine   │ Pending │ 2024-11-18 10:31│
│ ...  │                │          │         │                  │
└──────────────────────────────────────────────────────────────────┘

[a]pprove  [r]eject  [d]elete  [t]able  [q]uit
```

## Examples

### Running with Debug Logging

See detailed execution logs:

```bash
RUST_LOG=botticelli_tui=debug,botticelli_mcp_client=debug cargo run --bin tui
```

### Running the Chat Example

There's a standalone chat example:

```bash
cargo run --example chat_with_mcp
```

## Architecture

### Chat Mode Architecture

Channel-based async tool execution:

```
┌──────────┐   Enter    ┌───────────┐   Async    ┌─────────┐
│  Input   │──────────>│  AppState │──────────>│   MCP   │
│  Buffer  │            │           │            │  Client │
└──────────┘            └───────────┘            └─────────┘
                              │                       │
                              │   Channel             │
                              │<──────────────────────│
                              ▼
                        ┌───────────┐
                        │   Chat    │
                        │   View    │
                        └───────────┘
```

Flow:

1. **User Input** → Chat view collects input
2. **Message Sent** → Added to conversation history
3. **MCP Execution** → Spawned as async task (non-blocking)
4. **Tool Calls** → LLM orchestrates tool usage
5. **Results** → Sent through channel to UI thread
6. **UI Update** → Messages and tool calls visualized in real-time

## Library Usage

Use the TUI as a library in your own applications:

```rust
use botticelli_models::AnthropicClient;
use botticelli_tui::Tui;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with MCP integration
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let driver = Arc::new(AnthropicClient::new(
        api_key,
        "claude-3-5-sonnet-20241022",
    ));

    let mut tui = Tui::with_mcp(driver)?;
    tui.run().await?;

    Ok(())
}
```

## Configuration

### Narratives Directory

Default: `./narratives/`

- Created automatically if it doesn't exist
- Contains TOML narrative definition files
- Scanned by the `list_narratives` tool

### LLM Model

Current: `claude-3-5-sonnet-20241022`

To use a different model, modify `src/bin/tui.rs`:

```rust
let driver = Arc::new(AnthropicClient::new(
    api_key,
    "claude-opus-4-5-20250929",  // Change here
));
```

### Database Tables (Review Mode)

Required tables:
- `content_generation` - Content records
- `content_generation_tables` - Table metadata

## Development

### Adding Custom Tools

Extend `AppState::with_mcp_integration()` in `src/state.rs`:

```rust
// Register custom tool
registry.register(
    "my_tool".to_string(),
    Arc::new(MyCustomTool::new())
)?;
```

Your tool must implement the `ToolHandler` trait from `botticelli_mcp_client`:

```rust
use async_trait::async_trait;
use botticelli_mcp_client::{ToolHandler, McpClientResult};
use pmcp::{Content, ToolInfo};

struct MyCustomTool;

#[async_trait]
impl ToolHandler for MyCustomTool {
    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "my_tool",
            Some("Description".to_string()),
            serde_json::json!({ /* schema */ }),
        )
    }

    async fn execute(&self, args: serde_json::Value)
        -> McpClientResult<Vec<Content>> {
        // Implementation
    }
}
```

## Troubleshooting

### API Key Not Set

**Error:**
```
Error: ANTHROPIC_API_KEY environment variable not set
```

**Solution:**
```bash
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

### No Tool Execution

If tools aren't being called:

1. The LLM may not think tools are needed for your prompt
2. Try explicit prompts: "Use the echo tool to say hello"
3. Enable debug logging to see execution details:
   ```bash
   RUST_LOG=debug cargo run --bin tui
   ```

### Compilation Errors

Ensure all features are enabled:

```bash
cargo check --all-features
```

## Known Limitations

1. **Internal Tools Not Connected** - Tool registry is populated but `UnifiedMcpClient` currently only supports external MCP servers. Tools will work when internal tool support is added.

2. **No Database Integration in Chat** - Currently uses file-based narratives only.

3. **Single Conversation** - Only one active conversation at a time.

4. **No Conversation Persistence** - Conversations are lost on exit.

## Future Enhancements

- [ ] Multiple conversation support
- [ ] Conversation history persistence
- [ ] Database-backed narrative storage
- [ ] External MCP server connections
- [ ] Tool discovery UI
- [ ] Settings view for configuration
- [ ] Narrative browser integration
- [ ] View mode switching (Chat ↔ Review ↔ Browser)

## Dependencies

### Core
- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `tokio` - Async runtime

### Botticelli
- `botticelli_mcp_client` - MCP orchestration
- `botticelli_models` - LLM drivers
- `botticelli_narrative` - Narrative system
- `botticelli_database` - Database operations (optional)

### Optional
- `diesel` - Database queries (feature: `database`)

## Version

Current version: 0.2.0

## License

See workspace root for license information.
