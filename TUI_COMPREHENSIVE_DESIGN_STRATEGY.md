# TUI Comprehensive Design Strategy

**Date**: 2025-12-15
**Status**: 📋 Planning Document
**Purpose**: Expose the full range of Botticelli features through a powerful, intuitive TUI

---

## Executive Summary

This document outlines a comprehensive TUI design strategy that showcases Botticelli's unique position as a **multi-LLM orchestration platform** with:

1. **Internal Tool Ecosystem** - Native narrative generation, validation, and execution
2. **External MCP Integration** - Access to ecosystem servers (filesystem, git, search, cloud)
3. **Self-Driving Intelligence** - LLMs that discover and use tools autonomously
4. **Multi-Backend Flexibility** - Gemini, Claude, Groq, HuggingFace, Ollama, Perplexity
5. **Professional Workflows** - Database management, scheduling, bot deployment

**Design Philosophy**: Make complexity accessible through progressive disclosure. Beginners see a clean chat interface. Power users access full orchestration capabilities.

---

## Current Capabilities Inventory

### ✅ Implemented Features

#### 1. MCP Client Architecture
- **UnifiedMcpClient**: Routes tool calls between internal and external systems
- **Internal Tool Registry**: Narrative, database, elicitation tools
- **External Server Connections**: Filesystem, git, search via pmcp
- **Agentic Loops**: Self-driving execution with max iteration limits
- **Metrics Tracking**: Tool call counts, success rates, timing

#### 2. Internal MCP Tools

**Narrative Tools:**
- `create_narrative` - Generate narrative TOML from prompts
- `validate_narrative` - Comprehensive validation with actionable errors
- `list_narratives` - Browse available narrative files
- `load_narrative` - Read narrative content
- `execute_narrative` - Run multi-act workflows with any LLM backend

**Elicitation Tools:**
- `create_elicitation_session` - Interactive narrative building
- `elicit_metadata` - Guided metadata creation
- `elicit_act` - Step-by-step act definition
- `finalize_elicitation` - Complete and save narrative
- `create_carousel` - Multi-level narrative composition

**Database Tools** (feature-gated):
- `create_table` - Schema definition and creation
- `query_table` - SQL query execution with results
- `inspect_table` - Schema reflection and metadata
- `table_exists` - Check table availability

**Registry Tools:**
- `list_registry_keys` - Browse available items
- `get_registry_item` - Retrieve by key
- `upsert_registry_item` - Create or update entries

#### 3. External MCP Ecosystem

**Available Servers** (via pmcp):
- **@modelcontextprotocol/server-filesystem** - File operations, directory traversal
- **@modelcontextprotocol/server-git** - Git status, commits, branches, history
- **@modelcontextprotocol/server-brave-search** - Web search integration
- **@modelcontextprotocol/server-github** - GitHub API access
- **@modelcontextprotocol/server-postgres** - Direct database access
- **@modelcontextprotocol/server-google-maps** - Location services
- **@modelcontextprotocol/server-fetch** - HTTP requests
- **@modelcontextprotocol/server-memory** - Persistent KV storage

#### 4. LLM Backend Support

**Fully Integrated:**
- ✅ **Gemini** (Google) - Flash, Pro, Thinking modes
- ✅ **Anthropic** (Claude) - Sonnet, Opus, Haiku
- ✅ **Groq** - Fast inference on open models
- ✅ **HuggingFace** - Access to model hub
- ✅ **Ollama** - Local model execution
- ✅ **Perplexity** - Web-grounded responses

**Features:**
- Token counting per provider
- Rate limit detection and handling
- Model family classification
- Automatic model selection (ModelSelector)
- Tiered fallback strategies

#### 5. Narrative Execution Engine

**Capabilities:**
- Multi-act sequential workflows
- Template variable substitution
- Cross-act context passing
- Per-act model selection
- Stop sequence handling
- Thinking mode support
- Tool calling within narratives
- Database integration
- Media attachment handling

#### 6. Current TUI State

**Implemented:**
- Chat view with message history
- Narrative browser (list navigation)
- Narrative editor (placeholder)
- Settings view (placeholder)
- Tab switching (Tab/Shift+Tab)
- MCP integration with real-time updates
- Input handling and command dispatch
- Event loop with async support

**Architecture:**
- TuiApp coordinator (245 lines)
- Composable View trait
- Centralized Command dispatch
- AppState with ViewMode enum
- EventHandler with mpsc channels

### 🚧 Partially Implemented

- Narrative validation UI (backend complete, no visualization)
- Tool call tracking (logged but not displayed)
- External server management (can connect, no UI)
- Database browser (tools available, no interface)
- Metrics collection (tracked but not shown)

### 📋 Planned Features

- Bot scheduling and management
- Social media integration (Discord ready)
- Conversation history persistence
- Saved queries and filters
- Multi-pane layouts
- Search across all entities
- Export and import workflows
- Configuration presets
- Collaborative editing

---

## Strategic Design Goals

### 1. **Progressive Disclosure**

**Principle**: Surface complexity only when needed.

**Application:**
- **Level 1 (Beginner)**: Chat interface - "just talk to Claude"
- **Level 2 (User)**: Narrative browser - "run pre-made workflows"
- **Level 3 (Creator)**: Narrative editor - "build custom workflows"
- **Level 4 (Power User)**: Tool explorer - "access raw MCP capabilities"
- **Level 5 (Developer)**: Database browser - "inspect internal state"

### 2. **Self-Driving by Default**

**Principle**: The LLM should discover tools, not the user.

**Application:**
- Don't show tool lists upfront - show what was used after execution
- Emphasize natural language: "Create a narrative about space exploration"
- LLM figures out it needs: `create_elicitation_session` → `elicit_metadata` → `elicit_act` → `finalize_elicitation`
- User sees thinking process in real-time

### 3. **Unified Mental Model**

**Principle**: Everything is a resource that can be queried, modified, or executed.

**Resources:**
- **Narratives**: `.toml` files defining workflows
- **Bots**: Assigned narrative + schedule + platform
- **Conversations**: Message history + context
- **Tables**: Database schemas + data
- **Tools**: Internal registry + external servers
- **Schedules**: Cron expressions + bot assignments

**Actions:**
- **List**: Browse available resources
- **Inspect**: View details and metadata
- **Execute**: Run workflows or queries
- **Edit**: Modify configuration
- **Create**: Generate new resources

### 4. **Observability First**

**Principle**: Always show what's happening under the hood.

**Application:**
- Real-time tool call visualization
- Thinking mode output streaming
- Metrics dashboard (calls/sec, tokens, latency)
- Error traces with actionable suggestions
- Audit log of all operations

### 5. **Multi-Modal Interaction**

**Principle**: Support different work styles and workflows.

**Modes:**
- **Conversational**: Natural language chat
- **Imperative**: Keyboard shortcuts and commands
- **Visual**: Browse and click (when appropriate)
- **Scripted**: Saved workflows and macros

---

## Comprehensive Tab Architecture

### Tab 1: Chat 💬 - **The Gateway**

**Purpose**: Natural language interface to all Botticelli capabilities

**Layout:**
```
┌─ Chat ─────────────────────────────────────────────────────────────┐
│ ┌─ Conversation ─────────────────────────────────────────────────┐ │
│ │ System: I have access to 15 internal tools and 3 external MCP  │ │
│ │ servers. I can help with narratives, databases, files, and git.│ │
│ │                                                                 │ │
│ │ You: Create a narrative about space exploration with 3 acts    │ │
│ │                                                                 │ │
│ │ Assistant: I'll create a space exploration narrative. Let me   │ │
│ │ start an elicitation session...                                │ │
│ │                                                                 │ │
│ │ [Thinking] Breaking this down:                                 │ │
│ │ 1. Create session for new narrative                            │ │
│ │ 2. Elicit metadata (name, description, model)                  │ │
│ │ 3. Create 3 acts about space exploration                       │ │
│ │ 4. Finalize and save                                            │ │
│ │                                                                 │ │
│ │ [Tool: create_elicitation_session]                             │ │
│ │ ✓ Created session: a1b2c3d4                                    │ │
│ │                                                                 │ │
│ │ [Tool: elicit_metadata]                                        │ │
│ │ {                                                               │ │
│ │   "name": "space_exploration",                                 │ │
│ │   "description": "Multi-act narrative...",                     │ │
│ │   "model": "claude-3-5-sonnet-20241022"                        │ │
│ │ }                                                               │ │
│ │ ✓ Metadata set                                                 │ │
│ │                                                                 │ │
│ │ [Tool: elicit_act] (1/3)                                       │ │
│ │ Act: "introduction" - Set the scene for space exploration      │ │
│ │ ✓ Act created                                                  │ │
│ │ ... (continuing)                                                │ │
│ └─────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ ┌─ Input ───────────────────────────────────────────────────────┐ │
│ │ > _                                                            │ │
│ └─────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│ ⚡ 4 tools called │ 🎯 Session: a1b2c3d4 │ 🤖 Claude Sonnet        │
├─────────────────────────────────────────────────────────────────────┤
│ Ctrl+L Clear │ Ctrl+T Tools │ / Search │ : Command                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **Thinking Mode Visualization**
   - Show LLM's reasoning process in real-time
   - Collapsible thinking blocks
   - Highlight key decisions

2. **Tool Call Rendering**
   - Show tool name, arguments, result
   - Color-coded by success/failure
   - Expandable JSON views
   - Link to tool documentation

3. **Smart Context**
   - Display current session IDs
   - Show loaded narratives
   - Indicate active external servers
   - Preview recent history

4. **Quick Actions**
   - `Ctrl+T` - Open tool explorer
   - `Ctrl+N` - View last narrative
   - `Ctrl+D` - Database quick query
   - `Ctrl+E` - Export conversation

**Interactions:**
- Type naturally - LLM figures out tools
- Reference entities: "Use the narrative we just created"
- Chain operations: "Now execute it with Gemini"
- Inspect results: "Show me the validation errors"

---

### Tab 2: Orchestrator 🎭 - **The Brain**

**Purpose**: Visualize and control the agentic execution loop

**Layout:**
```
┌─ Orchestrator ─────────────────────────────────────────────────────┐
│ ┌─ Execution Flow ──────────────────────────────────────────────┐ │
│ │                                                                │ │
│ │  User Message                                                  │ │
│ │      │                                                         │ │
│ │      ▼                                                         │ │
│ │  [Iteration 1] ──────────────────────────────────────         │ │
│ │      │ Model: Claude Sonnet                                   │ │
│ │      │ Tokens: 1,234 in / 456 out                             │ │
│ │      │ Latency: 1.2s                                          │ │
│ │      │                                                         │ │
│ │      ├─> Tool: create_narrative                               │ │
│ │      │   Source: Internal Registry                            │ │
│ │      │   Args: {name: "test", ...}                            │ │
│ │      │   Result: ✓ Success (234ms)                            │ │
│ │      │                                                         │ │
│ │      ├─> Tool: validate_narrative                             │ │
│ │      │   Source: Internal Registry                            │ │
│ │      │   Args: {content: "...", strict: true}                 │ │
│ │      │   Result: ✗ Failed - 2 errors                          │ │
│ │      │                                                         │ │
│ │      ▼                                                         │ │
│ │  [Iteration 2] ──────────────────────────────────────         │ │
│ │      │ Model: Claude Sonnet                                   │ │
│ │      │ Tokens: 2,100 in / 890 out                             │ │
│ │      │                                                         │ │
│ │      ├─> Tool: create_narrative (retry with fixes)            │ │
│ │      │   Result: ✓ Success (189ms)                            │ │
│ │      │                                                         │ │
│ │      ├─> Tool: validate_narrative                             │ │
│ │      │   Result: ✓ Success - No errors                        │ │
│ │      │                                                         │ │
│ │      ▼                                                         │ │
│ │  Final Response: "Created and validated narrative 'test'"     │ │
│ │                                                                │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ ┌─ Metrics ─────────────────────────────────────────────────────┐ │
│ │ Iterations: 2 / 10 max                                        │ │
│ │ Total Tokens: 4,680                                            │ │
│ │ Internal Tools: 4 calls                                        │ │
│ │ External Tools: 0 calls                                        │ │
│ │ Success Rate: 75% (3/4)                                        │ │
│ │ Total Latency: 2.8s                                            │ │
│ └────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│ [P] Pause │ [S] Step │ [M] Max Iterations │ [E] Export Trace      │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **Execution Visualization**
   - Tree view of iteration flow
   - Tool call dependencies
   - Success/failure indicators
   - Timing waterfall

2. **Real-Time Metrics**
   - Token usage per iteration
   - Tool call distribution
   - Latency breakdown
   - Cost estimation

3. **Interactive Controls**
   - Pause execution mid-loop
   - Step through iterations
   - Adjust max iterations
   - Modify approval rules

4. **Debugging Support**
   - Export execution trace (JSON)
   - Replay from checkpoint
   - Inspect tool arguments
   - View full responses

**Use Cases:**
- **Development**: Debug why a workflow is looping
- **Optimization**: Identify slow tools
- **Learning**: Understand how LLM chains tools
- **Auditing**: Export full execution trace

---

### Tab 3: Narratives 📖 - **The Workflows**

**Purpose**: Browse, create, edit, validate, and execute narrative workflows

**Layout:**
```
┌─ Narratives ───────────────────────────────────────────────────────┐
│ ┌─ Browser ─────────────┐ ┌─ Details ─────────────────────────────┐│
│ │ 📁 Recent             │ │ 📄 daily_showcase.toml                ││
│ │   ├─ showcase.toml    │ │                                       ││
│ │   └─ space_explora... │ │ Name: Daily Showcase                  ││
│ │                       │ │ Description: Daily content showcase   ││
│ │ 📁 discord/ (40)      │ │ Model: claude-3-5-sonnet-20241022    ││
│ │   ├─ daily_showcase   │ │                                       ││
│ │   ├─ welcome          │ │ Acts: 3                               ││
│ │   └─ community_qa     │ │ ├─ 1. introduction                    ││
│ │                       │ │ │    Prompt: "Set the scene..."       ││
│ │ 📁 examples/ (10)     │ │ │    Temp: 0.7                        ││
│ │   ├─ simple_greeting  │ │ │                                     ││
│ │   └─ multi_act_demo   │ │ ├─ 2. showcase_features               ││
│ │                       │ │ │    Prompt: "Highlight key..."       ││
│ │ 📁 tests/ (5)         │ │ │    Temp: 0.8                        ││
│ │                       │ │ │    Thinking: extended               ││
│ │ ────────────────────  │ │ │                                     ││
│ │ [N] New               │ │ └─ 3. conclusion                      ││
│ │ [I] Import            │ │      Prompt: "Wrap up with..."        ││
│ │ [S] Search            │ │      Temp: 0.6                        ││
│ └───────────────────────┘ │                                       ││
│                           │ Variables:                            ││
│                           │ • topic: AI, blockchain, crypto       ││
│                           │ • audience: developers                ││
│                           │                                       ││
│                           │ Validation: ✓ No errors               ││
│                           │ Last executed: 2024-12-14 08:30      ││
│                           │                                       ││
│                           │ [X] Execute │ [E] Edit │ [V] Validate││
│                           │ [D] Duplicate │ [Del] Delete          ││
│                           └───────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│ 40 narratives │ Last: daily_showcase │ Status: Valid               │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **Smart Browser**
   - Hierarchical folder view
   - Recent files at top
   - Quick search and filter
   - Category grouping

2. **Rich Details Panel**
   - Metadata preview
   - Act structure visualization
   - Variable inspection
   - Validation status
   - Execution history

3. **Quick Actions**
   - Execute with default backend
   - Validate before execution
   - Duplicate for variants
   - Edit in place

4. **Create Wizard** (Press `N`)
   ```
   ┌─ Create Narrative Wizard ──────────────────────────────┐
   │                                                         │
   │ Step 1/3: Basic Info                                   │
   │                                                         │
   │ Name: space_exploration                                │
   │ Description: Multi-act narrative about space           │
   │ Model: [claude-3-5-sonnet-20241022 ▼]                 │
   │                                                         │
   │ [Next] [Cancel]                                        │
   │                                                         │
   │ ─────────────────────────────────────────────────────  │
   │                                                         │
   │ Step 2/3: Acts                                         │
   │                                                         │
   │ How many acts? [3]                                     │
   │                                                         │
   │ Act 1: introduction                                    │
   │   Prompt: [Enter prompt...]                            │
   │   Temperature: [0.7]                                   │
   │                                                         │
   │ [+ Add Act] [Next] [Back]                              │
   │                                                         │
   │ ─────────────────────────────────────────────────────  │
   │                                                         │
   │ Step 3/3: Review & Create                              │
   │                                                         │
   │ [Preview TOML] [Create] [Back] [Cancel]                │
   └─────────────────────────────────────────────────────────┘
   ```

5. **Validation View** (Press `V`)
   ```
   ┌─ Validation Results ────────────────────────────────────┐
   │                                                         │
   │ ✗ 2 errors, 1 warning found in test.toml               │
   │                                                         │
   │ Error 1: Invalid acts syntax                           │
   │   Line 10: Found [[acts]] but acts should be a table   │
   │                                                         │
   │   Suggestion: Use one of these formats:                │
   │   • [acts.introduction]                                │
   │   • [acts."Act 1"]                                     │
   │                                                         │
   │   [Jump to Line] [Auto-Fix]                            │
   │                                                         │
   │ Error 2: Undefined model                               │
   │   Line 3: Model 'gpt-4' not recognized                 │
   │                                                         │
   │   Did you mean:                                        │
   │   • gemini-2.0-flash-exp                               │
   │   • claude-3-5-sonnet-20241022                         │
   │                                                         │
   │   [Select Suggestion] [Ignore]                         │
   │                                                         │
   │ Warning 1: Unused resource                             │
   │   table 'analytics' defined but never referenced       │
   │                                                         │
   │   [Show References] [Ignore]                           │
   │                                                         │
   │ [Re-validate] [Export Report] [Close]                  │
   └─────────────────────────────────────────────────────────┘
   ```

**Interactions:**
- Arrow keys navigate files
- Enter to view details
- `X` to execute immediately
- `E` to open in editor
- `V` to validate
- `N` to create new (launches wizard OR chat)
  - Wizard: Traditional form-based
  - Chat: "Describe the narrative you want to create..."

---

### Tab 4: Tools 🔧 - **The Capabilities**

**Purpose**: Explore and test internal/external tools directly

**Layout:**
```
┌─ Tools ────────────────────────────────────────────────────────────┐
│ ┌─ Registry ────────────┐ ┌─ Tool Details ────────────────────────┐│
│ │ Internal (15)         │ │ 🔧 create_narrative                   ││
│ │ ├─ Narratives (5)     │ │                                       ││
│ │ │  ├─ create_...      │ │ Source: Internal Registry             ││
│ │ │  ├─ validate_...    │ │ Category: Narrative Generation        ││
│ │ │  ├─ execute_...     │ │                                       ││
│ │ │  ├─ list_...        │ │ Description:                          ││
│ │ │  └─ load_...        │ │ Creates a new narrative TOML file     ││
│ │ ├─ Elicitation (5)    │ │ from a natural language prompt using  ││
│ │ │  ├─ create_sessi... │ │ LLM-guided elicitation.               ││
│ │ │  ├─ elicit_meta...  │ │                                       ││
│ │ │  └─ ...             │ │ Input Schema:                         ││
│ │ ├─ Database (4)       │ │ {                                     ││
│ │ │  ├─ create_table    │ │   "prompt": {                         ││
│ │ │  ├─ query_table     │ │     "type": "string",                 ││
│ │ │  └─ ...             │ │     "description": "What to create"   ││
│ │ └─ Registry (1)       │ │   },                                  ││
│ │                       │ │   "model": {                          ││
│ │ External Servers (3)  │ │     "type": "string",                 ││
│ │ ├─ filesystem         │ │     "default": "gemini-2.0-flash..."  ││
│ │ │  └─ 12 tools        │ │   }                                   ││
│ │ ├─ git                │ │ }                                     ││
│ │ │  └─ 8 tools         │ │                                       ││
│ │ └─ brave-search       │ │ Output Schema:                        ││
│ │    └─ 2 tools         │ │ {                                     ││
│ │                       │ │   "file_path": "string",              ││
│ │ ────────────────────  │ │   "content": "string",                ││
│ │ [C] Connect Server    │ │   "validation": {...}                 ││
│ │ [R] Refresh           │ │ }                                     ││
│ └───────────────────────┘ │                                       ││
│                           │ Recent Calls: 234                     ││
│                           │ Success Rate: 97.4%                   ││
│                           │ Avg Latency: 1.2s                     ││
│                           │                                       ││
│                           │ [T] Test Tool │ [D] Documentation     ││
│                           │ [M] Metrics │ [E] Export Schema       ││
│                           └───────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│ 15 internal │ 22 external (3 servers) │ 234 total calls           │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **Unified Tool Browser**
   - Internal tools from ToolRegistry
   - External tools from connected MCP servers
   - Categorized by function
   - Searchable by name/description

2. **Tool Inspector**
   - Full JSON schema display
   - Parameter documentation
   - Example inputs/outputs
   - Historical metrics

3. **Interactive Testing** (Press `T`)
   ```
   ┌─ Test Tool: create_narrative ──────────────────────────┐
   │                                                         │
   │ Input:                                                  │
   │ {                                                       │
   │   "prompt": "Create a narrative about AI safety",      │
   │   "model": "claude-3-5-sonnet-20241022"                │
   │ }                                                       │
   │                                                         │
   │ [Execute] [Load Example] [Clear]                       │
   │                                                         │
   │ ───────────────────────────────────────────────────────│
   │                                                         │
   │ Result:                                                 │
   │ ✓ Success (1.2s)                                       │
   │                                                         │
   │ {                                                       │
   │   "file_path": "./narratives/ai_safety.toml",          │
   │   "content": "[narrative]\\nname = \\"ai_safety\\"...",  │
   │   "validation": {                                       │
   │     "valid": true,                                     │
   │     "errors": []                                        │
   │   }                                                     │
   │ }                                                       │
   │                                                         │
   │ [Save Result] [Retry] [Close]                          │
   └─────────────────────────────────────────────────────────┘
   ```

4. **Server Management** (Press `C`)
   ```
   ┌─ Connect External MCP Server ──────────────────────────┐
   │                                                         │
   │ Select server to connect:                              │
   │                                                         │
   │ ○ Filesystem (@modelcontextprotocol/server-filesystem) │
   │ ○ Git (@modelcontextprotocol/server-git)               │
   │ ○ Brave Search (@modelcontextprotocol/server-brave...) │
   │ ○ GitHub (@modelcontextprotocol/server-github)         │
   │ ○ PostgreSQL (@modelcontextprotocol/server-postgres)   │
   │ ○ Custom (enter command...)                            │
   │                                                         │
   │ [Connect] [Configure] [Cancel]                         │
   │                                                         │
   │ Active Servers:                                        │
   │ ✓ filesystem (12 tools)                                │
   │ ✓ git (8 tools)                                        │
   │ ✓ brave-search (2 tools)                               │
   │                                                         │
   │ [Disconnect All] [View Logs]                           │
   └─────────────────────────────────────────────────────────┘
   ```

**Use Cases:**
- **Discovery**: "What tools can I use?"
- **Learning**: "How does create_narrative work?"
- **Testing**: "Let me try this tool with sample data"
- **Debugging**: "Why is this tool failing?"
- **Integration**: "Connect to filesystem MCP server"

---

### Tab 5: Database 💾 - **The Persistence**

**Purpose**: Browse tables, run queries, inspect data

**Layout:**
```
┌─ Database ─────────────────────────────────────────────────────────┐
│ ┌─ Tables ──────────────┐ ┌─ Query Editor ────────────────────────┐│
│ │ 📊 Tables             │ │ SELECT * FROM content                 ││
│ │ ├─ content (1,234)    │ │ WHERE created_at > NOW() - INTERVAL   ││
│ │ ├─ narrativ... (456)  │ │   '7 days'                            ││
│ │ ├─ act_execu... (1.8K)│ │ ORDER BY created_at DESC              ││
│ │ ├─ model_res... (5.4K)│ │ LIMIT 20;                             ││
│ │ └─ content_... (789)  │ │                                       ││
│ │                       │ │ [Execute: Ctrl+Enter] [Format] [Save] ││
│ │ 📑 Saved Queries      │ └───────────────────────────────────────┘│
│ │ ├─ Recent content     │                                          │
│ │ ├─ Failed executions  │ ┌─ Results ─────────────────────────────┐│
│ │ └─ Top performers     │ │ Showing 20 of 142 rows (0.34s)        ││
│ │                       │ │                                       ││
│ │ ────────────────────  │ │ id │ created_at │ type │ platform     ││
│ │ [Q] New Query         │ │ ───┼────────────┼──────┼──────────    ││
│ │ [S] Schema Browser    │ │ 123│ 2024-12-14 │ post │ discord      ││
│ └───────────────────────┘ │ 124│ 2024-12-14 │ comment│discord    ││
│                           │ 125│ 2024-12-13 │ post │ discord      ││
│                           │ ... (17 more rows)                    ││
│                           │                                       ││
│                           │ [→] Details │ [↓] Export │ [F] Filter ││
│                           └───────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│ postgres://localhost:5432/botticelli │ 5 tables │ 142 rows        │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **Table Browser**
   - List all tables with row counts
   - Schema inspector
   - Saved queries library

2. **Query Editor**
   - SQL syntax highlighting
   - Auto-completion
   - Format/prettify
   - Save queries for reuse

3. **Results Viewer**
   - Paginated results
   - Export to CSV/JSON
   - Filter and sort
   - Drill-down to row details

4. **Schema Inspector** (Press `S`)
   ```
   ┌─ Table Schema: content ─────────────────────────────────┐
   │                                                         │
   │ Table: content                                          │
   │ Rows: 1,234                                             │
   │                                                         │
   │ Columns:                                                │
   │ ┌──────────────┬──────────┬──────────┬────────────────┐│
   │ │ Name         │ Type     │ Nullable │ Default        ││
   │ ├──────────────┼──────────┼──────────┼────────────────┤│
   │ │ id           │ SERIAL   │ NO       │ nextval(...)   ││
   │ │ created_at   │ TIMESTAMP│ NO       │ NOW()          ││
   │ │ content_type │ TEXT     │ NO       │ -              ││
   │ │ platform     │ TEXT     │ NO       │ -              ││
   │ │ body         │ TEXT     │ YES      │ NULL           ││
   │ └──────────────┴──────────┴──────────┴────────────────┘│
   │                                                         │
   │ Indexes:                                                │
   │ • PRIMARY KEY (id)                                      │
   │ • INDEX idx_created_at (created_at DESC)                │
   │                                                         │
   │ [Generate Query] [Export DDL] [Close]                  │
   └─────────────────────────────────────────────────────────┘
   ```

**Quick Actions:**
- `/` - Search tables
- `Q` - New query
- `S` - Schema browser
- `Ctrl+Enter` - Execute query

**MCP Integration:**
- Use `query_table` tool from chat
- "Show me recent content from the database"
- LLM generates and executes SQL

---

### Tab 6: Bots 🤖 - **The Automation**

**Purpose**: Manage bot deployments, schedules, and status

**Layout:**
```
┌─ Bots ─────────────────────────────────────────────────────────────┐
│ ┌─ Bot List ────────────┐ ┌─ Bot Details ─────────────────────────┐│
│ │ Active (2)            │ │ 🤖 showcase_bot                       ││
│ │ ├─ ● showcase_bot     │ │                                       ││
│ │ └─ ○ welcome_bot      │ │ Status: ● Running                     ││
│ │                       │ │ Platform: Discord                     ││
│ │ Inactive (1)          │ │ Channel: #showcase                    ││
│ │ └─ test_bot           │ │ Narrative: daily_showcase.toml        ││
│ │                       │ │                                       ││
│ │ ────────────────────  │ │ Schedule:                             ││
│ │ [N] New Bot           │ │ • 10:00 AM daily (Mon-Fri)            ││
│ │ [I] Import Config     │ │ • Next run: 2024-12-15 10:00         ││
│ └───────────────────────┘ │                                       ││
│                           │ Configuration:                        ││
│                           │ • Model: claude-3-5-sonnet-20241022   ││
│                           │ • Temperature: 0.7                    ││
│                           │ • Max tokens: 2048                    ││
│                           │                                       ││
│                           │ Execution History (5 recent):         ││
│                           │ ✓ 2024-12-14 10:00 - Success (3 acts) ││
│                           │ ✓ 2024-12-13 10:00 - Success (3 acts) ││
│                           │ ⚠ 2024-12-12 10:00 - Failed (act 2)   ││
│                           │ ✓ 2024-12-11 10:00 - Success (3 acts) ││
│                           │ ✓ 2024-12-10 10:00 - Success (3 acts) ││
│                           │                                       ││
│                           │ [S] Start │ [P] Pause │ [L] View Logs ││
│                           │ [A] Assign Narrative │ [E] Edit       ││
│                           └───────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│ 2 active │ 1 inactive │ Next run: showcase_bot in 12h 34m         │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **Bot Management**
   - Start/stop bots
   - View real-time status
   - Edit configuration
   - Assign narratives

2. **Scheduling**
   - Cron expressions
   - Next run preview
   - Execution history
   - Failure tracking

3. **Execution Logs** (Press `L`)
   ```
   ┌─ Bot Logs: showcase_bot ────────────────────────────────┐
   │                                                         │
   │ 2024-12-14 10:00:00 [INFO] Starting execution          │
   │ 2024-12-14 10:00:01 [INFO] Loaded narrative: daily...  │
   │ 2024-12-14 10:00:02 [INFO] Act 1: introduction         │
   │ 2024-12-14 10:00:05 [INFO] Response received (456 tok) │
   │ 2024-12-14 10:00:06 [INFO] Act 2: showcase_features    │
   │ 2024-12-14 10:00:09 [INFO] Response received (892 tok) │
   │ 2024-12-14 10:00:10 [INFO] Act 3: conclusion           │
   │ 2024-12-14 10:00:12 [INFO] Response received (234 tok) │
   │ 2024-12-14 10:00:13 [INFO] Posted to Discord #showcase │
   │ 2024-12-14 10:00:14 [INFO] Execution complete          │
   │                                                         │
   │ [Export] [Filter] [Search] [Close]                     │
   └─────────────────────────────────────────────────────────┘
   ```

4. **Create Bot Wizard** (Press `N`)
   ```
   ┌─ Create Bot ────────────────────────────────────────────┐
   │                                                         │
   │ Step 1/3: Basic Info                                   │
   │                                                         │
   │ Name: community_bot                                    │
   │ Platform: [Discord ▼]                                  │
   │ Channel: #community                                    │
   │                                                         │
   │ ─────────────────────────────────────────────────────  │
   │                                                         │
   │ Step 2/3: Assign Narrative                             │
   │                                                         │
   │ Narrative: [Select from library ▼]                     │
   │ • community_qa.toml                                    │
   │ • welcome.toml                                         │
   │ • daily_showcase.toml                                  │
   │                                                         │
   │ Or: [Create New Narrative]                             │
   │                                                         │
   │ ─────────────────────────────────────────────────────  │
   │                                                         │
   │ Step 3/3: Schedule                                     │
   │                                                         │
   │ Frequency: [Daily ▼]                                   │
   │ Time: [14:00]                                          │
   │ Days: [Mon Tue Wed Thu Fri Sat Sun]                    │
   │                                                         │
   │ Cron: 0 14 * * *                                       │
   │ Next run: 2024-12-15 14:00                             │
   │                                                         │
   │ [Create] [Back] [Cancel]                               │
   └─────────────────────────────────────────────────────────┘
   ```

**Integration:**
- Bots reference narratives by path
- Execute on schedule using narrative engine
- Post results to configured platform
- Track all executions in database

---

### Tab 7: Settings ⚙️ - **The Configuration**

**Purpose**: Global configuration, API keys, preferences

**Layout:**
```
┌─ Settings ─────────────────────────────────────────────────────────┐
│                                                                     │
│ ┌─ General ─────────────────────────────────────────────────────┐ │
│ │ UI Theme: [Dark ▼]                                            │ │
│ │ Editor Mode: [Vim ▼]                                          │ │
│ │ Auto-save: [✓] Enabled                                        │ │
│ │ Confirmation prompts: [✓] Enabled                             │ │
│ └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ ┌─ LLM Providers ───────────────────────────────────────────────┐ │
│ │ Default Provider: [Claude ▼]                                  │ │
│ │                                                               │ │
│ │ ● Anthropic (Claude)                                          │ │
│ │   API Key: ••••••••••••••••••••••••••                         │ │
│ │   Models: Sonnet, Opus, Haiku                                │ │
│ │   Status: ✓ Connected                                        │ │
│ │   [Test Connection] [Update Key]                             │ │
│ │                                                               │ │
│ │ ● Google (Gemini)                                             │ │
│ │   API Key: ••••••••••••••••••••••••••                         │ │
│ │   Models: 2.0 Flash, 1.5 Pro, Thinking                       │ │
│ │   Status: ✓ Connected                                        │ │
│ │   [Test Connection] [Update Key]                             │ │
│ │                                                               │ │
│ │ ○ Groq (Not configured)                                       │ │
│ │   [Set API Key]                                               │ │
│ │                                                               │ │
│ │ ○ HuggingFace (Not configured)                                │ │
│ │   [Set API Key]                                               │ │
│ │                                                               │ │
│ │ ○ Ollama (Local)                                              │ │
│ │   Host: [localhost:11434]                                     │ │
│ │   Status: ○ Not running                                      │ │
│ │   [Start Ollama] [Configure]                                 │ │
│ └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ ┌─ Database ────────────────────────────────────────────────────┐ │
│ │ Connection String: postgres://localhost:5432/botticelli       │ │
│ │ Pool Size: [10]                                               │ │
│ │ Status: ✓ Connected                                           │ │
│ │ [Test Connection] [Edit]                                      │ │
│ └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ ┌─ MCP Servers ─────────────────────────────────────────────────┐ │
│ │ Internal Registry: ✓ Loaded (15 tools)                       │ │
│ │                                                               │ │
│ │ External Servers:                                             │ │
│ │ • filesystem: ✓ Connected (12 tools)                         │ │
│ │ • git: ✓ Connected (8 tools)                                 │ │
│ │ • brave-search: ✓ Connected (2 tools)                        │ │
│ │                                                               │ │
│ │ [Add Server] [Remove] [View All]                              │ │
│ └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ ┌─ Paths ───────────────────────────────────────────────────────┐ │
│ │ Narratives Directory: [./narratives]                          │ │
│ │ Database URL: [postgres://localhost:5432/botticelli]         │ │
│ │ Logs Directory: [./logs]                                      │ │
│ │ [Edit Paths]                                                  │ │
│ └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│ [Save All] [Reset to Defaults] [Export Config]                    │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

1. **LLM Provider Management**
   - Configure all supported providers
   - Test connections
   - Set default provider
   - View available models

2. **MCP Configuration**
   - Manage external servers
   - View tool availability
   - Connection status

3. **Paths and Directories**
   - Configure file locations
   - Set working directories

4. **Import/Export**
   - Export full configuration
   - Import from file
   - Share configurations

---

## Advanced Features

### 1. **Multi-Pane Layouts**

Allow power users to see multiple tabs simultaneously:

```
┌─────────────────────────────────────────────────────────────────┐
│ Chat (50%)                │ Orchestrator (50%)                  │
│                           │                                     │
│ You: Create a narrative   │ [Iteration 1]                       │
│                           │   ├─> create_narrative              │
│ Assistant: Creating...    │   └─> validate_narrative            │
│   [Tool: create_...]      │                                     │
│   [Tool: validate_...]    │ [Iteration 2]                       │
│                           │   └─> (final response)              │
├───────────────────────────┴─────────────────────────────────────┤
│ Status: 2 iterations │ 2 tools │ 1.2s                           │
└─────────────────────────────────────────────────────────────────┘
```

**Layouts:**
- Horizontal split (top/bottom)
- Vertical split (left/right)
- Quad view (2x2)
- Custom arrangements

### 2. **Global Search** (Press `/`)

```
┌─ Search Everything ─────────────────────────────────────────────┐
│                                                                 │
│ Query: space exploration                                       │
│                                                                 │
│ Results (12):                                                   │
│                                                                 │
│ 📖 Narratives (3)                                               │
│ ├─ space_exploration.toml - Multi-act narrative...            │
│ ├─ mars_mission.toml - Mars exploration workflow              │
│ └─ nasa_prompt.toml - NASA-style content generation           │
│                                                                 │
│ 💬 Conversations (2)                                            │
│ ├─ 2024-12-14 10:30 - "Create narrative about space..."       │
│ └─ 2024-12-10 08:15 - "Mars exploration ideas"                │
│                                                                 │
│ 💾 Database (4)                                                 │
│ ├─ content #123 - "Space exploration advancements..."         │
│ ├─ content #456 - "Mars mission updates"                      │
│ └─ ...                                                          │
│                                                                 │
│ 🔧 Tools (3)                                                    │
│ ├─ create_narrative - "...space..."                           │
│ └─ ...                                                          │
│                                                                 │
│ [Enter] Open │ [Ctrl+N/P] Navigate │ [Esc] Close              │
└─────────────────────────────────────────────────────────────────┘
```

### 3. **Command Palette** (Press `:`)

Vim-style command execution:

```
┌─ Command Palette ───────────────────────────────────────────────┐
│ :execute narrative daily_showcase                              │
│                                                                 │
│ Suggestions:                                                    │
│ • :execute narrative <name> [backend]                          │
│ • :create narrative <name>                                     │
│ • :validate narrative <name>                                   │
│ • :connect server <name> <command>                             │
│ • :query <sql>                                                 │
│ • :bot start <name>                                            │
│ • :export config                                               │
│                                                                 │
│ Recent:                                                         │
│ • :execute narrative showcase                                  │
│ • :validate narrative test                                     │
│ • :connect server filesystem npx -y @model...                  │
└─────────────────────────────────────────────────────────────────┘
```

**Commands:**
- `:exec <narrative>` - Execute narrative
- `:val <narrative>` - Validate narrative
- `:new narrative` - Create narrative
- `:connect <server>` - Connect MCP server
- `:q <sql>` - Run database query
- `:bot start <name>` - Start bot
- `:export <type>` - Export data

### 4. **Metrics Dashboard**

Real-time performance monitoring:

```
┌─ Metrics Dashboard ─────────────────────────────────────────────┐
│                                                                 │
│ ┌─ Tool Calls (Last 24h) ────────────────────────────────────┐ │
│ │ Total: 1,234                                               │ │
│ │ Success: 1,156 (93.7%)                                     │ │
│ │ Failed: 78 (6.3%)                                          │ │
│ │                                                            │ │
│ │ By Category:                                               │ │
│ │ ████████████████ Narratives (640) 51.9%                   │ │
│ │ ██████████ Database (312) 25.3%                           │ │
│ │ ██████ Elicitation (187) 15.2%                            │ │
│ │ ███ External (95) 7.7%                                    │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ ┌─ LLM Usage ────────────────────────────────────────────────┐ │
│ │ Total Requests: 456                                        │ │
│ │ Total Tokens: 1.2M (890K in / 310K out)                   │ │
│ │ Avg Latency: 1.8s                                          │ │
│ │                                                            │ │
│ │ By Provider:                                               │ │
│ │ • Claude: 234 requests (51.3%)                             │ │
│ │ • Gemini: 189 requests (41.4%)                             │ │
│ │ • Groq: 33 requests (7.2%)                                 │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ ┌─ Bot Executions ───────────────────────────────────────────┐ │
│ │ Total: 142                                                 │ │
│ │ Successful: 138 (97.2%)                                    │ │
│ │ Failed: 4 (2.8%)                                           │ │
│ │                                                            │ │
│ │ Next Scheduled:                                            │ │
│ │ • showcase_bot in 2h 15m                                   │ │
│ │ • welcome_bot in 5h 30m                                    │ │
│ └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 5. **Export Workflows**

Export any data for external processing:

```
┌─ Export ────────────────────────────────────────────────────────┐
│                                                                 │
│ What to export:                                                 │
│ ○ Conversation history                                         │
│ ○ Narrative (TOML)                                             │
│ ○ Database query results                                       │
│ ● Orchestrator execution trace                                 │
│ ○ Bot execution logs                                           │
│ ○ Tool call metrics                                            │
│ ○ Full configuration                                           │
│                                                                 │
│ Format:                                                         │
│ ○ JSON                                                         │
│ ● YAML                                                          │
│ ○ CSV (if applicable)                                          │
│ ○ Markdown                                                      │
│                                                                 │
│ Include:                                                        │
│ [✓] Timestamps                                                  │
│ [✓] Metadata                                                    │
│ [✓] Full tool arguments                                        │
│ [ ] Sensitive data (API keys, etc.)                            │
│                                                                 │
│ Output: [./exports/trace_2024-12-15.yaml]                      │
│                                                                 │
│ [Export] [Cancel]                                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Enhanced Chat (Week 1)
**Goal**: Make chat the primary interface with full tool visibility

1. **Tool Call Visualization**
   - Render tool name, args, result in chat
   - Color-code success/failure
   - Expandable JSON views
   - Link to tool docs

2. **Thinking Mode Display**
   - Stream thinking blocks in real-time
   - Collapsible sections
   - Highlight key decisions

3. **Smart Context Indicators**
   - Show active sessions
   - Display loaded narratives
   - Indicate connected servers

**Deliverables:**
- Enhanced ChatView with tool rendering
- ThinkingBlock component
- ToolCallCard component
- ContextIndicator widget

### Phase 2: Orchestrator View (Week 2)
**Goal**: Full transparency into agentic execution

1. **Execution Flow Visualization**
   - Tree view of iterations
   - Tool call dependencies
   - Success/failure indicators
   - Timing waterfall

2. **Metrics Display**
   - Token usage tracking
   - Tool call distribution
   - Latency breakdown
   - Cost estimation

3. **Interactive Controls**
   - Pause/resume execution
   - Step through iterations
   - Adjust max iterations
   - Export traces

**Deliverables:**
- New OrchestratorView
- ExecutionTree widget
- MetricsPanel component
- Interactive controls

### Phase 3: Enhanced Narratives (Week 3)
**Goal**: Professional narrative management

1. **Rich Browser**
   - Hierarchical folder view
   - Quick search and filter
   - Category grouping
   - Recent files

2. **Validation UI**
   - Display errors with suggestions
   - Auto-fix capabilities
   - Jump to line
   - Actionable feedback

3. **Create Wizard**
   - Step-by-step wizard
   - Or conversational creation
   - Preview before save
   - Validation on create

**Deliverables:**
- Enhanced NarrativeBrowserView
- ValidationPanel component
- CreateWizard component
- Chat-based creation flow

### Phase 4: Tools Explorer (Week 4)
**Goal**: Discover and test all capabilities

1. **Tool Browser**
   - Internal + external tools
   - Categorization
   - Search and filter
   - Metrics per tool

2. **Tool Inspector**
   - Full schema display
   - Parameter docs
   - Example I/O
   - Historical metrics

3. **Interactive Testing**
   - JSON editor for inputs
   - Execute and view results
   - Save test cases
   - Export schemas

4. **Server Management**
   - Connect to MCP servers
   - View available tools
   - Monitor connection status
   - Disconnect/reconnect

**Deliverables:**
- New ToolsView
- ToolBrowser widget
- ToolInspector panel
- TestRunner component
- ServerManager UI

### Phase 5: Database Browser (Week 5)
**Goal**: Professional database management

1. **Table Browser**
   - List tables with counts
   - Schema inspector
   - Saved queries

2. **Query Editor**
   - SQL syntax highlighting
   - Auto-completion
   - Format/prettify
   - Execute and view results

3. **Results Viewer**
   - Paginated display
   - Export capabilities
   - Filter and sort
   - Drill-down details

**Deliverables:**
- New DatabaseView
- TableBrowser widget
- QueryEditor component
- ResultsViewer component
- SchemaInspector panel

### Phase 6: Bot Management (Week 6)
**Goal**: Deploy and monitor bots

1. **Bot Browser**
   - Active/inactive lists
   - Status indicators
   - Quick actions

2. **Bot Details**
   - Configuration display
   - Execution history
   - Log viewer
   - Edit interface

3. **Create Wizard**
   - Bot configuration
   - Narrative assignment
   - Schedule setup
   - Platform integration

**Deliverables:**
- New BotsView
- BotBrowser widget
- BotDetails panel
- CreateBotWizard component
- LogViewer component

### Phase 7: Advanced Features (Week 7-8)
**Goal**: Power user capabilities

1. **Multi-Pane Layouts**
   - Split views
   - Custom arrangements
   - Save layouts

2. **Global Search**
   - Search all entities
   - Fuzzy matching
   - Quick navigation

3. **Command Palette**
   - Vim-style commands
   - Auto-completion
   - Command history

4. **Metrics Dashboard**
   - Real-time monitoring
   - Historical trends
   - Performance insights

5. **Export Workflows**
   - Multiple formats
   - Configurable includes
   - Batch exports

**Deliverables:**
- Layout manager
- Global search component
- Command palette
- Metrics dashboard
- Export system

---

## Technical Architecture

### State Management

```rust
pub struct GlobalState {
    /// Current tab
    current_tab: Tab,

    /// Chat state
    chat: ChatState,

    /// Orchestrator state
    orchestrator: OrchestratorState,

    /// Narrative browser state
    narratives: NarrativeState,

    /// Tool explorer state
    tools: ToolsState,

    /// Database browser state
    database: DatabaseState,

    /// Bot manager state
    bots: BotsState,

    /// Settings
    settings: SettingsState,

    /// Global search state
    search: SearchState,

    /// Layout configuration
    layout: LayoutConfig,
}
```

### View Trait Extension

```rust
pub trait View {
    fn render(&self, frame: &mut Frame, state: &AppState) -> TuiResult<()>;
    fn handle_input(&self, key: KeyEvent, state: &AppState) -> TuiResult<Option<Command>>;

    // New methods for advanced features
    fn supports_search(&self) -> bool { false }
    fn search(&self, query: &str, state: &AppState) -> Vec<SearchResult> { vec![] }

    fn supports_export(&self) -> bool { false }
    fn export(&self, format: ExportFormat, state: &AppState) -> TuiResult<String> {
        Err(TuiError::not_implemented())
    }

    fn commands(&self) -> Vec<CommandDefinition> { vec![] }
}
```

### Component Library

Build reusable widgets:

```rust
// Widget for rendering tool calls
pub struct ToolCallWidget<'a> {
    tool_name: &'a str,
    arguments: &'a Value,
    result: Option<&'a str>,
    success: bool,
    expanded: bool,
}

// Widget for thinking mode
pub struct ThinkingWidget<'a> {
    content: &'a str,
    expanded: bool,
}

// Widget for metrics
pub struct MetricsWidget {
    iterations: usize,
    tokens: usize,
    latency: Duration,
    success_rate: f64,
}

// Widget for tree views
pub struct TreeWidget<T> {
    items: Vec<TreeNode<T>>,
    selected: Option<usize>,
    expanded: HashSet<usize>,
}
```

---

## Success Metrics

### User Experience
- **Discoverability**: Users can find features without documentation (85%+ success rate)
- **Speed**: Common workflows complete in <5 interactions
- **Clarity**: Tool usage is transparent (users understand what happened)
- **Flexibility**: Multiple paths to same goal (chat, keyboard, visual)

### Technical
- **Performance**: UI stays responsive with 100+ tool calls
- **Reliability**: Zero crashes during normal operation
- **Completeness**: All MCP capabilities accessible
- **Extensibility**: New views add without refactoring core

### Adoption
- **Chat Usage**: 60%+ of workflows start with chat
- **Tool Discovery**: Users discover >50% of tools naturally
- **Advanced Features**: 30%+ use orchestrator/tools tabs
- **Self-Service**: 80%+ of tasks completed without external docs

---

## Open Questions

1. **Keyboard Shortcuts**: Vim-style or Emacs-style or hybrid?
2. **Mobile Support**: Should we design for terminal on mobile?
3. **Collaboration**: Multi-user access to same TUI instance?
4. **Plugins**: Allow custom views/commands via plugin system?
5. **Themes**: Just dark/light or full customization?
6. **Notifications**: Desktop notifications for bot executions?
7. **Clipboard**: Integration for easy copy/paste of results?
8. **Mouse**: Full mouse support or keyboard-only?

---

## Next Steps

1. **Review**: Get feedback on this comprehensive design
2. **Prioritize**: Which phases to tackle first?
3. **Prototype**: Build Phase 1 (Enhanced Chat) to validate approach
4. **Iterate**: Test with users, refine based on feedback
5. **Document**: Create user guide and keyboard reference
6. **Ship**: Release incrementally, phase by phase

---

**Status**: 📋 Planning Complete - Ready for Review
**Timeline**: 8 weeks for full implementation
**Risk Level**: Medium (significant scope, proven patterns)
**Dependencies**: None blocking (all features already implemented)

---

🤖 Generated with Claude Code - Botticelli TUI Comprehensive Design Strategy
