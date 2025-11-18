# botticelli_tui

Terminal User Interface for the Botticelli ecosystem.

## Overview

Interactive terminal UI for reviewing and managing generated content. Built with Ratatui for a rich terminal experience.

## Features

- **Content review**: Browse pending, approved, and rejected content
- **Multi-table support**: Switch between different content tables
- **Keyboard navigation**: Vim-style keybindings
- **Status updates**: Approve, reject, or delete content
- **Real-time**: Auto-refreshes on changes

## Usage

### Basic

```rust
use botticelli_tui::run_tui;
use diesel::pg::PgConnection;

let mut conn = PgConnection::establish(&database_url)?;
run_tui(&mut conn).await?;
```

### From CLI

```bash
# Run TUI
botticelli tui

# Or with database URL
DATABASE_URL=postgres://user:pass@localhost/db botticelli tui
```

## Keyboard Controls

### Navigation

- `↑/k` - Move up
- `↓/j` - Move down
- `PgUp` - Page up
- `PgDn` - Page down
- `Home/g` - Go to top
- `End/G` - Go to bottom

### Actions

- `a` - Approve content
- `r` - Reject content  
- `d` - Delete content
- `t` - Switch table
- `q/Esc` - Quit

### Filters

- `1` - Show pending only
- `2` - Show approved only
- `3` - Show rejected only
- `0` - Show all

## Interface

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

## Application Modes

### Browse Mode

Default mode for navigating content:
- Arrow keys/vim keys for movement
- Page up/down for faster scrolling
- Filters to show specific status

### Table Selection Mode

Press `t` to select different content tables:
- Shows all available content generation tables
- Arrow keys to select
- Enter to switch

## Content Details

Selected row shows full details:
- Complete JSON data
- Narrative and act information
- Generation timestamp
- Model used
- Review status and timestamp

## Configuration

The TUI automatically detects content generation tables from the database metadata.

Required database tables:
- `content_generation` - Content records
- `content_generation_tables` - Table metadata

## Error Handling

The TUI displays errors in the UI:
- Database connection issues
- Query failures
- Invalid operations

Press any key to dismiss error messages.

## Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `botticelli_database` - Database operations
- `tokio` - Async runtime
- `diesel` - Database queries

## Version

Current version: 0.2.0
