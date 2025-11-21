# Act Output Capture and Template Substitution

## Problem

The `publish_welcome.toml` narrative needs to:
1. Send a message via `publish_message` act (returns message with `id` field)
2. Pin that message via `pin_message` act (requires the `message_id` from step 1)

Currently, there's no way to capture output from one act and use it as input to another act.

## Current State

### What Works
- Acts execute sequentially
- Bot commands can be defined with static parameters
- Template syntax `{{act_name}}` is used but not implemented

### What Doesn't Work
- Capturing return values from bot commands
- Passing captured values to subsequent acts
- Template substitution in bot command arguments

## Proposed Solution

### 1. Act Output Storage

Add output storage to the executor:

```rust
pub struct NarrativeExecutor {
    // ... existing fields ...
    act_outputs: HashMap<String, serde_json::Value>, // Store output from each act
}
```

When an act completes:
- If it has bot commands, capture the JSON response
- Store it in `act_outputs` keyed by act name
- For LLM acts, store the generated text

### 2. Template Substitution

When processing bot command args:
- Scan for `{{act_name}}` patterns
- Look up `act_name` in `act_outputs`
- Replace template with captured value
- Support JSON path: `{{act_name.field.subfield}}`

Example:
```toml
[bots.pin_message]
platform = "discord"
command = "messages.pin"
message_id = "{{publish_message.id}}"  # Get id field from publish_message output
```

### 3. Implementation Steps

#### Step 1: Add Output Capture
- Modify `process_act()` to return act output
- Store output in `act_outputs` HashMap
- Handle both bot command JSON and LLM text outputs

#### Step 2: Template Resolution
- Create `resolve_templates()` function
- Parse `{{name.path}}` syntax
- Look up values in `act_outputs`
- Support JSON path traversal

#### Step 3: Apply to Bot Commands
- Before executing bot command, resolve all template args
- Replace `{{...}}` with actual values
- Validate required values are present

#### Step 4: Error Handling
- Error if template references undefined act
- Error if referenced act hasn't executed yet (ordering)
- Error if JSON path doesn't exist in output

### 4. Example Flow

```toml
# Act 1: Send message
publish_message = ["bots.publish_message"]

# Act 2: Pin the message (uses output from act 1)
pin_message = ["bots.pin_message"]

[bots.publish_message]
platform = "discord"
command = "messages.send"
content = "Welcome!"

[bots.pin_message]
platform = "discord"
command = "messages.pin"
message_id = "{{publish_message.id}}"  # Template substitution
```

Execution:
1. `publish_message` executes → returns `{"id": "123456", "content": "Welcome!", ...}`
2. Store in `act_outputs["publish_message"]`
3. `pin_message` executes → resolve `{{publish_message.id}}` → `"123456"`
4. Call `messages.pin` with `message_id = "123456"`

## Benefits

- Enables act chaining with data flow
- Makes narratives more powerful
- Follows declarative TOML style
- Simple template syntax

## Testing Strategy

1. Unit test template parsing and resolution
2. Test with mock bot commands
3. Integration test with Discord (send + pin)
4. Test error cases (missing act, wrong path)

## Documentation Updates

- Add template syntax to NARRATIVE_TOML_SPEC
- Document JSON path syntax
- Add examples of act chaining
- Show error messages for common mistakes
