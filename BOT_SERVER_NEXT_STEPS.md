# Bot Server - Next Steps

**Status**: ✅ Implementation Complete - Ready for Testing  
**Date**: 2025-11-28  
**Commit**: 8d2feae

## What We Built

A complete three-phase content pipeline bot server that:

1. **Generation Bot** - Creates diverse Discord posts using carousel narratives
2. **Curation Bot** - Reviews and approves quality content (simulating human selection)
3. **Posting Bot** - Publishes approved content with human-like timing jitter

## Architecture Highlights

### Driver-Agnostic Design
- All bots generic over `BotticelliDriver` trait
- Works with any LLM backend (Gemini, Claude, etc.)
- Clean separation between bot logic and LLM implementation

### Narrative-Driven
- Each bot executes pre-configured narratives
- No hardcoded prompts in bot code
- Easy to modify behavior by editing TOML files

### JSON Compliance Workflow
- Integrated JSON extraction with schema validation
- Type coercion for common mismatches
- Fuzzy field name matching
- JSONB support for complex types

### Schema Inference
- Tables created automatically from JSON output
- No manual schema definition required
- Target table name support for shared tables

## Configuration

All bots configured via `bot_server.toml`:

```toml
[generation]
narrative_path = "crates/botticelli_narrative/narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"
interval_hours = 24  # Once per day

[curation]
narrative_path = "crates/botticelli_narrative/narratives/discord/curate_and_approve.toml"
narrative_name = "curate"
check_interval_hours = 12  # Check twice per day
batch_size = 10  # Process until queue empty

[posting]
narrative_path = "crates/botticelli_narrative/narratives/discord/discord_poster.toml"
narrative_name = "post"
base_interval_hours = 2  # Post every ~2 hours
jitter_minutes = 30  # ±30 min for natural rhythm
```

## Testing Plan

### Phase 1: Individual Bot Testing

**Test Generation Bot:**
```bash
just narrate generation_carousel.batch_generate
```
Expected: 15 posts (5 narratives × 3 iterations) in `potential_discord_posts`

**Test Curation Bot:**
```bash
just narrate curate_and_approve.curate
```
Expected: Process pending posts, move best to `approved_discord_posts`

**Test Posting Bot:**
```bash
just narrate discord_poster.post
```
Expected: Post one approved post to Discord, mark as posted

### Phase 2: Server Integration

**Start bot server:**
```bash
just bot-server
```

**Monitor:**
- Check logs for schedule execution
- Verify database table updates
- Confirm Discord posts appear
- Validate timing jitter working

### Phase 3: Production Readiness

**Observability:**
- [ ] All actors properly instrumented with `#[instrument]`
- [ ] Structured logging at key decision points
- [ ] Error tracking and recovery
- [ ] Metrics collection (posts generated, approved, posted)

**Reliability:**
- [ ] Graceful shutdown handling
- [ ] Database connection pool management
- [ ] Rate limit budget enforcement
- [ ] Retry logic for transient failures

**Deployment:**
- [ ] Systemd service file
- [ ] Environment variable configuration
- [ ] Secret management for API keys
- [ ] Log rotation and monitoring alerts

## Key Achievements

1. **Multi-narrative TOML support** - Multiple narratives per file
2. **Carousel mode** - Iterate through narrative sequences
3. **JSON compliance workflow** - Robust extraction and validation
4. **Schema inference** - Automatic table creation
5. **Budget multipliers** - Voluntary rate limiting
6. **Driver-agnostic bots** - Works with any LLM backend

## Known Issues

### Narratives Still TODO:
- `curate_and_approve.toml` - Needs implementation
- `discord_poster.toml` - Needs implementation

Both should follow the pattern established in `generation_carousel.toml`:
- Use JSON compliance workflow for final act
- Target appropriate tables (approved → posting)
- Include proper instrumentation

### Testing Gaps:
- No unit tests for bot actors yet
- Need integration tests for full pipeline
- Manual testing only so far

## Next Actions

### Immediate (Before Running Server):
1. Implement `curate_and_approve.toml` narrative
2. Implement `discord_poster.toml` narrative
3. Test each narrative individually
4. Verify table schemas match expectations

### Short Term (This Week):
1. Run bot server in development mode
2. Monitor for errors and edge cases
3. Add missing instrumentation
4. Write integration tests

### Medium Term (Next Sprint):
1. Deploy to production environment
2. Set up monitoring and alerting
3. Tune scheduling parameters
4. Collect metrics on success rates

## Questions to Answer

1. **Curation selection criteria** - How should the LLM decide which posts are "best"?
2. **Posting strategy** - Should we post to specific channels? Time of day optimization?
3. **Content diversity** - How to ensure variety in generated content?
4. **Failure handling** - What happens if generation produces no valid JSON?
5. **Queue management** - Should we limit pending post accumulation?

## Resources

- **Planning**: `BOT_SERVER_DEPLOYMENT_PLAN.md`
- **JSON Strategy**: `JSON_SCHEMA_MISMATCH_STRATEGY.md`
- **Extraction**: `JSON_EXTRACTION_STRATEGY.md`
- **Config**: `bot_server.toml`
- **Code**: `crates/botticelli_bot/`
