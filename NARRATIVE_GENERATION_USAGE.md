# Narrative Generation Usage Guide

**Tools**: `create_narrative`, `modify_narrative`, `save_narrative`  
**Status**: Phase 1 Complete  
**Last Updated**: 2025-12-08

---

## Quick Start

### Basic Workflow

```
User → LLM → create_narrative → TOML_v1
User → LLM → modify_narrative → TOML_v2  
User → LLM → save_narrative → file saved
```

The LLM maintains context by passing the TOML forward through iterations.

---

## Tool 1: `create_narrative`

### Purpose
Generate a complete narrative from a natural language description.

### Input Schema
```json
{
  "description": "string (required) - What the narrative should do",
  "name": "string (required) - Narrative identifier",
  "default_model": "string (optional) - LLM model to use",
  "default_temperature": "number (optional) - 0.0-1.0"
}
```

### Examples

#### Simple Narrative
```json
{
  "description": "Analyze user feedback and generate a summary report",
  "name": "feedback_analysis"
}
```

**Output:**
```toml
[narrative]
name = "feedback_analysis"
description = "Analyze user feedback and generate a summary report"

[toc]
order = ["analyze", "generate"]

[acts]
analyze = "Analyze user feedback"
generate = "generate a summary report"
```

#### With Model Configuration
```json
{
  "description": "Fetch Discord stats then analyze trends",
  "name": "discord_trends",
  "default_model": "gemini-2.0-flash-exp",
  "default_temperature": 0.7
}
```

**Output:**
```toml
[narrative]
name = "discord_trends"
description = "Fetch Discord stats then analyze trends"
model = "gemini-2.0-flash-exp"
temperature = 0.7

[toc]
order = ["fetch", "analyze"]

[acts]
fetch = "Fetch Discord stats"
analyze = "analyze trends"
```

#### Multi-Act Workflow
```json
{
  "description": "Fetch data, then analyze it, then generate report, then send email",
  "name": "reporting_pipeline"
}
```

**Output:** Creates 4 acts based on "then" pattern.

### Act Extraction Logic

The tool intelligently extracts acts from descriptions:

**Pattern: "then" splitting**
- "Do X then do Y" → 2 acts

**Pattern: "and" splitting**
- "Do X and do Y" → 2 acts (only if no "then")

**Pattern: comma splitting**
- "Fetch data, analyze results, post findings" → 3 acts

**Verb Detection**
Common verbs are automatically recognized for act naming:
- fetch, get, retrieve, load, read
- analyze, process, transform
- generate, create, build
- send, post, publish
- compare, evaluate, filter

---

## Tool 2: `modify_narrative`

### Purpose
Transform an existing narrative based on instructions.

### Input Schema
```json
{
  "narrative_toml": "string (required) - Existing TOML to modify",
  "modification": "string (required) - What to change",
  "save_to": "string (optional) - Path to auto-save result"
}
```

### Supported Modifications

#### Change Model
```json
{
  "narrative_toml": "<existing TOML>",
  "modification": "Change model to Claude"
}
```

**Recognized patterns:**
- "Change model to X"
- "Use X model"
- "Set model to X"
- "Use Claude" / "Use Gemini" / "Use GPT"

**Models detected:**
- `claude` → `claude-3-5-sonnet-20241022`
- `gemini` → `gemini-2.0-flash-exp`
- `gpt-4` → `gpt-4`
- `gpt` → `gpt-4-turbo`

#### Set Temperature
```json
{
  "narrative_toml": "<existing TOML>",
  "modification": "Set temperature to 0.3"
}
```

**Extracts numeric value** from modification text.

#### Add Act
```json
{
  "narrative_toml": "<existing TOML>",
  "modification": "Add act that summarizes the results"
}
```

**Recognized patterns:**
- "Add act that X"
- "Add act which X"
- "Add an act that X"

#### Remove Act
```json
{
  "narrative_toml": "<existing TOML>",
  "modification": "Remove act named analyze"
}
```

**Extracts act name** from modification text.

#### Add Bot Command
```json
{
  "narrative_toml": "<existing TOML>",
  "modification": "Add bot command to fetch Discord stats"
}
```

**Recognized patterns:**
- "Add bot command"
- "Add bot"
- "Bot command"

*Note: Phase 1 adds a basic bot command template. Phase 3 will add full parsing.*

### Output
```json
{
  "toml": "string - Updated TOML",
  "validation": {
    "valid": "boolean",
    "errors": ["array of error messages"],
    "warnings": ["array of warnings"]
  },
  "changes": ["array of change descriptions"],
  "saved_to": "string | null - Path if save_to provided"
}
```

---

## Tool 3: `save_narrative`

### Purpose
Save narrative TOML to a file with safety checks.

### Input Schema
```json
{
  "narrative_toml": "string (required) - TOML content",
  "file_path": "string (required) - Where to save (.toml extension required)",
  "overwrite": "boolean (optional, default: false) - Allow overwriting"
}
```

### Examples

#### Basic Save
```json
{
  "narrative_toml": "<TOML content>",
  "file_path": "./narratives/my_narrative.toml"
}
```

#### Overwrite Existing
```json
{
  "narrative_toml": "<TOML content>",
  "file_path": "./narratives/existing.toml",
  "overwrite": true
}
```

### Safety Features

✅ **Extension validation** - Must end in `.toml`  
✅ **Overwrite protection** - Prevents accidental overwrites  
✅ **Directory creation** - Auto-creates parent directories  
✅ **Path sanitization** - Validates file paths

### Output
```json
{
  "status": "saved",
  "file_path": "/absolute/path/to/file.toml",
  "size_bytes": 1234,
  "overwritten": false
}
```

---

## Complete Workflow Examples

### Example 1: Simple Iteration

**User:** "Create a narrative that analyzes Discord server stats"

**LLM calls:** `create_narrative`
```json
{
  "description": "Analyze Discord server stats",
  "name": "discord_analysis"
}
```

**Returns:**
```json
{
  "toml": "[narrative]\nname = \"discord_analysis\"\n...",
  "validation": {"valid": true},
  "summary": "Created narrative with 1 act"
}
```

---

**User:** "Add a fetch step before the analysis"

**LLM calls:** `modify_narrative`
```json
{
  "narrative_toml": "<previous TOML>",
  "modification": "Add act that fetches the server stats"
}
```

**Returns:**
```json
{
  "toml": "<updated TOML>",
  "changes": ["Added act 'fetch'"]
}
```

---

**User:** "Use Claude instead"

**LLM calls:** `modify_narrative`
```json
{
  "narrative_toml": "<previous TOML>",
  "modification": "Use Claude"
}
```

**Returns:**
```json
{
  "toml": "<updated TOML>",
  "changes": ["Changed model to 'claude-3-5-sonnet-20241022'"]
}
```

---

**User:** "Save it"

**LLM calls:** `save_narrative`
```json
{
  "narrative_toml": "<final TOML>",
  "file_path": "./narratives/discord_analysis.toml"
}
```

**Returns:**
```json
{
  "status": "saved",
  "file_path": "/full/path/to/discord_analysis.toml"
}
```

### Example 2: Complex Multi-Act Workflow

**User:** "Create a pipeline that fetches data, processes it, generates a report, and emails the results"

**LLM calls:** `create_narrative`
```json
{
  "description": "Fetch data, then process it, then generate report, then email results",
  "name": "reporting_pipeline",
  "default_model": "gemini-2.0-flash-exp"
}
```

**Returns:** Narrative with 4 acts (fetch, process, generate, email)

---

**User:** "Make the processing step use a lower temperature for more consistent results"

**LLM calls:** `modify_narrative`
```json
{
  "narrative_toml": "<previous TOML>",
  "modification": "Set temperature to 0.2"
}
```

*Note: Phase 1 sets global temperature. Phase 3 will support per-act temperatures.*

---

**User:** "Actually, remove the email step - we'll handle that manually"

**LLM calls:** `modify_narrative`
```json
{
  "narrative_toml": "<previous TOML>",
  "modification": "Remove act email"
}
```

**Returns:**
```json
{
  "toml": "<updated TOML>",
  "changes": ["Removed act 'email'"]
}
```

---

**User:** "Save this as data_pipeline.toml"

**LLM calls:** `save_narrative`
```json
{
  "narrative_toml": "<final TOML>",
  "file_path": "./narratives/data_pipeline.toml"
}
```

### Example 3: Refinement Through Validation

**User:** "Create a narrative for content curation"

**LLM calls:** `create_narrative`
```json
{
  "description": "Curate content from multiple sources",
  "name": "content_curation"
}
```

**Returns:**
```json
{
  "toml": "<basic TOML>",
  "validation": {
    "valid": true,
    "warnings": ["Consider adding more specific acts"]
  }
}
```

---

**User:** "Make it more specific - fetch from Reddit and Twitter"

**LLM calls:** `modify_narrative`
```json
{
  "narrative_toml": "<previous TOML>",
  "modification": "Add act that fetches from Reddit and another that fetches from Twitter"
}
```

**Returns:** Updated narrative with specific fetch acts

---

## Best Practices

### For Descriptions

✅ **Be specific about workflow steps**
```json
{"description": "Fetch data, analyze trends, generate report"}
```

❌ **Avoid vague descriptions**
```json
{"description": "Do some analysis"}
```

---

✅ **Use connecting words** ("then", "and")
```json
{"description": "Fetch data then analyze it"}
```

❌ **Don't use run-on sentences**
```json
{"description": "Fetch data analyze it make report send email"}
```

---

✅ **Include action verbs**
```json
{"description": "Retrieve stats, analyze patterns, post insights"}
```

---

### For Modifications

✅ **Be explicit about what to change**
```json
{"modification": "Change model to Claude"}
```

❌ **Avoid ambiguous requests**
```json
{"modification": "Make it better"}
```

---

✅ **One change per modification**
```json
{"modification": "Set temperature to 0.3"}
```

❌ **Don't combine multiple changes**
```json
{"modification": "Change model to Claude and set temperature to 0.3 and add an act"}
```

*Note: Make separate modify_narrative calls for multiple changes.*

---

✅ **Use recognized patterns**
```json
{"modification": "Add act that summarizes results"}
{"modification": "Use Gemini model"}
{"modification": "Set temperature to 0.5"}
```

---

### For File Paths

✅ **Use relative paths from project root**
```json
{"file_path": "./narratives/my_narrative.toml"}
```

❌ **Avoid absolute paths unless necessary**
```json
{"file_path": "/home/user/project/narratives/my_narrative.toml"}
```

---

✅ **Always use .toml extension**
```json
{"file_path": "./my_narrative.toml"}
```

❌ **Other extensions are rejected**
```json
{"file_path": "./my_narrative.txt"}  // ❌ Error
```

---

## Limitations (Phase 1)

### Current
- ⚠️ Simple act extraction (heuristic-based)
- ⚠️ Global model/temperature only (not per-act)
- ⚠️ Basic bot command template (not full parsing)
- ⚠️ No table queries or media in Phase 1
- ⚠️ No nested narrative references

### Coming in Phase 2-4
- ✨ LLM-assisted act extraction
- ✨ Per-act configuration
- ✨ Full resource parsing (bots, tables, media)
- ✨ Smart suggestions
- ✨ Token cost estimation

---

## Error Handling

### Common Errors

**Invalid Input**
```json
{
  "error": "InvalidInput: Missing 'description'"
}
```
**Fix:** Provide required field.

---

**Unknown Modification**
```json
{
  "error": "InvalidInput: Could not understand modification: 'foo bar'"
}
```
**Fix:** Use recognized patterns (see "Supported Modifications").

---

**Validation Failed**
```json
{
  "toml": "<invalid TOML>",
  "validation": {
    "valid": false,
    "errors": ["Missing [narrative] section"]
  }
}
```
**Fix:** The TOML is returned but invalid. Report the issue for debugging.

---

**File Exists**
```json
{
  "error": "ToolExecutionFailed: File already exists. Set overwrite=true"
}
```
**Fix:** Set `overwrite: true` in save_narrative.

---

**Invalid Extension**
```json
{
  "error": "InvalidInput: File path must end with .toml extension"
}
```
**Fix:** Use `.toml` extension.

---

## Testing

See `crates/botticelli_mcp/tests/narrative_generation_test.rs` for:
- 10 integration tests covering all tools
- Example usage patterns
- Edge case handling

Run tests:
```bash
cargo test -p botticelli_mcp --test narrative_generation_test
```

---

## Next Steps

1. **Test with real MCP clients**
2. **Gather feedback on act extraction quality**
3. **Implement Phase 2 enhancements**
4. **Add LLM-assisted modification parsing**

---

**Status**: Phase 1 Complete ✅  
**Tests**: 10/10 passing ✅  
**Ready for**: Production use with MCP clients
