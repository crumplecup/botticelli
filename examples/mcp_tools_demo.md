# Botticelli MCP Tools Demo

This demo showcases how Botticelli's chat interface exercises different MCP tools in response to natural language prompts.

## Prerequisites

```bash
# Start the chat interface
just chat-local
```

## Demo Flow

### 1. Create Narrative - Schema Inference

**User Prompt:**
```
Create a narrative to generate social media posts for the MINT navigation center in Grants Pass, Oregon.
```

**What Happens:**
- Chat interface sends prompt to MCP server
- MCP server calls `create_narrative` tool
- LLM analyzes the prompt and infers:
  - Need for a `discord_posts` table
  - Schema: `content TEXT, scheduled_time TIMESTAMP, target_audience TEXT`
  - Narrative steps to generate posts
- Returns TOML narrative
- Chat interface saves to database
- Displays: "Narrative created and saved!"

**Expected Output:**
```toml
[metadata]
title = "MINT Navigation Center Social Media Campaign"
description = "Generate engaging social media posts..."

[[tables]]
name = "discord_posts"
description = "Storage for generated social media content"

[[tables.columns]]
name = "content"
type = "TEXT"
description = "The post content"

[[steps]]
step_number = 1
description = "Generate initial post ideas"
llm_provider = "gemini"
# ... more steps
```

---

### 2. Validate Narrative

**User Prompt:**
```
Validate the narrative I just created.
```

**What Happens:**
- Retrieves latest narrative from database
- Calls `validate_narrative` tool with TOML content
- Checks:
  - TOML syntax validity
  - Required fields present
  - Schema definitions valid
  - Step sequences logical
- Returns validation result

**Expected Output:**
```
✓ Narrative is valid
- All required fields present
- Schema definitions correct
- 5 steps defined
```

---

### 3. Execute Narrative - Generate Content

**User Prompt:**
```
Execute the MINT narrative to generate 10 social media posts.
```

**What Happens:**
- Loads narrative from database
- Calls `execute_narrative` tool
- For each step:
  - Connects to specified LLM provider (Gemini)
  - Generates content based on prompt templates
  - Inserts results into `discord_posts` table
- Tracks execution state
- Returns summary

**Expected Output:**
```
Executing narrative: MINT Navigation Center Social Media Campaign
Step 1/5: Generate initial post ideas... ✓
Step 2/5: Create engaging hooks... ✓
Step 3/5: Add call-to-action... ✓
Step 4/5: Schedule posts... ✓
Step 5/5: Review and finalize... ✓

Generated 10 posts in discord_posts table
Execution time: 45 seconds
```

---

### 4. List Content - Query Results

**User Prompt:**
```
Show me the social media posts that were generated.
```

**What Happens:**
- Calls `list_content` tool
- Queries `discord_posts` table
- Returns rows with content

**Expected Output:**
```
Found 10 posts in discord_posts:

1. "🏠 Need support? MINT Navigation Center is here for you! 
   We provide housing assistance, case management, and community 
   resources. Visit us in Grants Pass! #CommunitySupport"
   Scheduled: 2025-12-10 09:00:00

2. "✨ New services available at MINT! Connect with our team 
   to learn about employment programs and life skills workshops..."
   Scheduled: 2025-12-10 14:00:00

... (8 more posts)
```

---

### 5. Describe Table - Introspection

**User Prompt:**
```
What columns does the discord_posts table have?
```

**What Happens:**
- Calls `describe_table` tool
- Queries PostgreSQL schema information
- Returns column definitions

**Expected Output:**
```
Table: discord_posts

Columns:
- id: SERIAL PRIMARY KEY
- content: TEXT
- scheduled_time: TIMESTAMP
- target_audience: TEXT
- created_at: TIMESTAMP DEFAULT NOW()
```

---

### 6. Complex Multi-Tool Workflow

**User Prompt:**
```
Create a narrative to analyze user engagement, execute it with 100 sample records, then show me the top 10 results sorted by engagement score.
```

**What Happens:**
1. `create_narrative` - Generates engagement analysis narrative
2. `validate_narrative` - Ensures it's valid
3. `execute_narrative` - Runs analysis, creates `user_engagement` table
4. `list_content` - Queries with LIMIT and ORDER BY

**Expected Output:**
```
Creating narrative... ✓
Validating narrative... ✓
Executing narrative... ✓
  - Created user_engagement table
  - Processed 100 records
  - Calculated engagement scores

Top 10 users by engagement:
1. user_42 - Score: 95.3
2. user_17 - Score: 89.7
3. user_103 - Score: 87.2
...
```

---

## Demo Script for Presentations

Run these commands in sequence:

```bash
# 1. Start chat interface
just chat-local

# 2. Create narrative
> Create a narrative to generate social media posts for the MINT navigation center

# 3. Validate it
> Validate the narrative

# 4. Execute it
> Execute the narrative to generate 5 posts

# 5. View results
> Show me the generated posts

# 6. Introspect schema
> What columns does the discord_posts table have?

# 7. Create another narrative
> Create a narrative to analyze which posts get the most engagement

# 8. Chain operations
> Execute the engagement analysis and show me the top 3 posts
```

---

## Key Takeaways

1. **Natural Language Interface**: Users describe what they want, not how to do it
2. **Schema Inference**: LLM automatically creates appropriate database schemas
3. **Multi-Provider Support**: Works with Gemini, Claude, GPT, etc.
4. **Persistent Storage**: All narratives and generated data stored in PostgreSQL
5. **Tool Chaining**: Complex workflows combine multiple MCP tools seamlessly
6. **Validation Built-in**: Automatic checks ensure data quality

---

## Testing the Demo

Run the integration tests to verify all functionality:

```bash
# Test individual tools
cargo test --test mcp_integration_test test_create_narrative
cargo test --test mcp_integration_test test_validate_narrative
cargo test --test mcp_integration_test test_execute_narrative

# Test complete workflow
cargo test --test mcp_integration_test test_full_narrative_workflow
```

---

## Architecture Diagram

```
┌─────────────┐
│   User      │
│  (Natural   │
│  Language)  │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  Chat TUI       │
│  (Ratatui)      │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ CommandExecutor │
│  (Routes to     │
│   MCP tools)    │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  MCP Client     │
│  (JSON-RPC)     │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  MCP Server     │
│  (Stdio/HTTP)   │
└──────┬──────────┘
       │
       ├──────────────┬──────────────┐
       ▼              ▼              ▼
┌──────────┐   ┌──────────┐   ┌──────────┐
│ Gemini   │   │PostgreSQL│   │  Claude  │
│   API    │   │ Database │   │   API    │
└──────────┘   └──────────┘   └──────────┘
```

---

## Next Steps

- Add more example prompts
- Create video walkthrough
- Add error recovery demos
- Show concurrent narrative execution
- Demonstrate rollback capabilities
