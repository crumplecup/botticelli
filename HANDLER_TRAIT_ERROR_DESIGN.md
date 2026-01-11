# Handler Trait Error Design Analysis

## Context

Currently implementing Serenity's `EventHandler` trait in `BotticelliHandler`. The trait methods (e.g., `guild_create`, `ready`, `channel_create`) don't return `Result` - they return `()`.

Our internal helper methods (`store_guild`, `store_channel`, etc.) are fallible and should return `Result`, but we're constrained by the external trait interface.

## The Question

**Should we modify our architecture to better handle errors in event handlers, given that we cannot change the Serenity trait signature?**

Current pattern:
```rust
// Serenity's trait (we cannot change this)
async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
    // Must return ()
}
```

Our helper methods (we control these):
```rust
async fn store_guild(&self, guild: &Guild) -> DiscordResult<()> {
    // Can fail
}
```

## Option 1: Keep Current Design (Error Handling at Call Site)

**What we're doing now:**

```rust
async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
    if let Err(e) = self.store_guild(&guild).await {
        error!(guild_id = %guild.id, error = %e, "Failed to store guild");
        return;
    }
    
    for channel in guild.channels.values() {
        if let Err(e) = self.store_channel(Some(guild.id), &Channel::Guild(channel.clone())).await {
            error!(channel_id = %channel.id, error = %e, "Failed to store channel");
        }
    }
    // ... continue with best effort
}
```

### Benefits
1. **Granular error handling**: Can decide per-entity whether to abort or continue
2. **Best-effort processing**: One channel failing doesn't stop the rest
3. **Clear error context**: Each error log includes specific entity IDs
4. **No trait changes needed**: Works within Serenity's constraints
5. **Partial success**: Some data stored is better than none

### Problems
1. **Verbose call sites**: Every call needs `if let Err` wrapper
2. **Inconsistent error handling**: Each event handler decides independently
3. **Hidden failures**: Errors logged but not bubbled up to caller
4. **No error aggregation**: Can't know "5 of 100 channels failed"
5. **Testing complexity**: Hard to verify error paths without checking logs

### Current State Issues
- `guild_create` has different error handling than `channel_create`
- Some errors abort (guild), others continue (channels/roles/members)
- No way for external code to know if event handling partially failed

## Option 2: Internal Result-Returning Trait

**Create our own trait abstraction:**

```rust
// Our internal trait
trait DiscordEventProcessor {
    async fn process_guild_create(&self, guild: &Guild) -> DiscordResult<()>;
    async fn process_channel_create(&self, channel: &GuildChannel) -> DiscordResult<()>;
    // ... etc
}

// Implement our trait
impl DiscordEventProcessor for BotticelliHandler {
    async fn process_guild_create(&self, guild: &Guild) -> DiscordResult<()> {
        self.store_guild(guild).await?;
        
        for channel in guild.channels.values() {
            self.store_channel(Some(guild.id), &Channel::Guild(channel.clone())).await?;
        }
        // ... propagate errors naturally
        Ok(())
    }
}

// Serenity trait calls our trait
#[async_trait]
impl EventHandler for BotticelliHandler {
    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
        if let Err(e) = self.process_guild_create(&guild).await {
            error!(error = %e, "Failed to process guild_create event");
        }
    }
}
```

### Benefits
1. **Clean separation**: Event handling logic vs error handling policy
2. **Testable**: Can test `process_guild_create` with Result assertions
3. **Natural error flow**: Use `?` operator throughout
4. **Consistent policy**: Single place decides abort vs continue
5. **Composable**: Easy to add retry logic, error aggregation, etc.
6. **Clear boundaries**: Serenity interface → Our interface → Repository

### Problems
1. **Additional abstraction layer**: More code, more indirection
2. **Two-trait system**: Must maintain both EventHandler and DiscordEventProcessor
3. **Duplication risk**: Each event needs two implementations
4. **Lost Serenity features**: Context, metadata might not pass through cleanly
5. **Over-engineering?**: Might be solving a problem we don't have yet

## Option 3: Error Aggregation Pattern

**Collect errors and report once:**

```rust
async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
    let mut errors = Vec::new();
    
    if let Err(e) = self.store_guild(&guild).await {
        errors.push(format!("Guild: {}", e));
        return; // Can't continue without guild
    }
    
    for channel in guild.channels.values() {
        if let Err(e) = self.store_channel(Some(guild.id), &Channel::Guild(channel.clone())).await {
            errors.push(format!("Channel {}: {}", channel.id, e));
        }
    }
    
    if !errors.is_empty() {
        error!(
            guild_id = %guild.id,
            error_count = errors.len(),
            errors = ?errors,
            "Partial failure storing guild data"
        );
    }
}
```

### Benefits
1. **Visibility**: See all failures in one log entry
2. **Metrics-friendly**: Easy to count failure rates
3. **Best effort**: Continues processing despite errors
4. **Single decision point**: One place to decide criticality
5. **Structured errors**: Can return structured data for dashboards

### Problems
1. **Still verbose**: Still need if-let at each call
2. **Memory overhead**: Collecting error strings
3. **Delayed feedback**: Errors reported at end, not immediately
4. **Loss of detail**: String concatenation loses error types
5. **Inconsistent handling**: When to abort vs continue still ad-hoc

## Option 4: Result-Returning Helpers + Error Policy Enum

**Encode error handling strategy in types:**

```rust
enum ErrorPolicy {
    Abort,      // Stop on first error
    BestEffort, // Log and continue
    Aggregate,  // Collect all errors
}

async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
    let result = self
        .process_with_policy(ErrorPolicy::BestEffort, || async {
            self.store_guild(&guild).await?;
            
            for channel in guild.channels.values() {
                self.store_channel(Some(guild.id), &Channel::Guild(channel.clone())).await?;
            }
            
            Ok(())
        })
        .await;
        
    if let Err(e) = result {
        error!(guild_id = %guild.id, error = %e, "Guild processing failed");
    }
}
```

### Benefits
1. **Explicit policy**: Strategy is clear in code
2. **Reusable**: Same pattern across all handlers
3. **Type-safe**: Compiler enforces error handling
4. **Flexible**: Easy to change strategy per event type
5. **Clean helpers**: Store methods just return Result naturally

### Problems
1. **Complex implementation**: Requires sophisticated error handling infrastructure
2. **Async closure issues**: Rust's async closures are still rough
3. **Type inference problems**: May need turbofish syntax
4. **Over-abstraction**: Might be too clever
5. **Debugging difficulty**: Hidden control flow

## Recommendation

**Go with Option 1 (Current Design) with refinements:**

### Why Option 1
1. **Serenity constraint is real**: We can't change the trait
2. **Best effort is correct**: Discord events should be processed independently
3. **Simple is better**: Explicit error handling at each call site is clear
4. **No new abstractions**: Doesn't add complexity to solve a non-problem
5. **Event-driven nature**: Events are fire-and-forget; errors are expected

### Refinements to Current Approach

1. **Consistent error handling pattern:**
   ```rust
   // Critical failure - abort event processing
   if let Err(e) = self.store_guild(&guild).await {
       error!(guild_id = %guild.id, error = %e, "Critical: Failed to store guild");
       return;
   }
   
   // Non-critical - log and continue
   if let Err(e) = self.store_channel(...).await {
       error!(channel_id = %id, error = %e, "Failed to store channel");
       // Continue processing other channels
   }
   ```

2. **Document criticality:**
   ```rust
   /// Store a Discord guild in the database.
   /// 
   /// # Errors
   /// 
   /// Returns error if guild cannot be stored. This is a critical failure -
   /// dependent entities (channels, roles) cannot be stored without the guild.
   async fn store_guild(&self, guild: &Guild) -> DiscordResult<()>
   ```

3. **Add error metrics:**
   ```rust
   if let Err(e) = self.store_channel(...).await {
       error!(channel_id = %id, error = %e, "Failed to store channel");
       // Could add: metrics::increment("discord.store_channel.error");
   }
   ```

4. **Structured error context:**
   ```rust
   error!(
       event = "guild_create",
       guild_id = %guild.id,
       entity = "channel",
       entity_id = %channel.id,
       error = %e,
       "Entity storage failed"
   );
   ```

## What NOT to Do

1. **Don't panic on storage failures** - Discord events will keep coming
2. **Don't silently ignore errors** - Must log with context
3. **Don't create internal Result-returning trait** - Over-engineering
4. **Don't try to batch/transaction** - Events are independent
5. **Don't block event processing** - Fast failure is acceptable

## Alternative: If We Owned the Trait

If we controlled EventHandler, we'd want:

```rust
trait EventHandler {
    async fn guild_create(&self, ctx: Context, guild: Guild) -> Result<(), EventError>;
    //                                                            ^^^^^^^^^^^^^^^^^^^
}
```

But we don't, and that's OK. Discord's event model is inherently best-effort.

## Conclusion

**Keep Option 1 (current design) with documented conventions:**
- Helper methods return `Result` ✅
- EventHandler methods handle errors explicitly ✅  
- Critical failures abort event processing ✅
- Non-critical failures log and continue ✅
- All errors include structured context ✅

The current design is correct for the problem domain. The verbosity is the cost of explicitness.
