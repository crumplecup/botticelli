# Phase 3.5: Security Framework Integration

**Status**: Planning  
**Started**: 2025-11-20  
**Dependencies**: Phase 3 (botticelli_security crate - ✅ COMPLETE)

## Overview

Phase 3.5 integrates the security framework into the existing bot command infrastructure and implements the first safe write command (`channels.send_message`).

## What We Built in Phase 3

✅ **botticelli_security crate** (1,906 lines, 37 tests passing):
- Permission model with allow/deny lists
- Input validation (Discord-specific)
- Content filtering (regex, mentions, URLs)
- Rate limiting (token bucket algorithm)
- Approval workflows (24hr expiry)
- Secure executor wrapper (5-layer pipeline)
- Comprehensive error types

## Phase 3.5 Goals

1. **Integrate SecureExecutor with BotCommandRegistry**
2. **Implement audit logging with database**
3. **Add first write command: channels.send_message**
4. **TOML configuration for security policies**
5. **Approval management commands/UI**

---

## Implementation Summary

This document outlines the integration plan for Phase 3.5. Key components:

1. **Security Integration** - Wrap existing executors with SecureExecutor
2. **Audit Logging** - PostgreSQL-based audit trail
3. **First Write Command** - `channels.send_message` with full security
4. **Configuration** - TOML-based security policies
5. **Approval Management** - Commands for managing pending actions

See PHASE_3_SECURITY_FRAMEWORK.md for detailed implementation specifications.

---

## Timeline

**Week 1-2**: Integration layer + audit logging  
**Week 3**: First write command implementation  
**Week 4**: TOML configuration system  
**Week 5**: Approval management UI  
**Week 6**: Testing, staging, production rollout  

**Total Effort**: 6 weeks

---

## Success Criteria

- ✅ All write commands protected by 5-layer security pipeline
- ✅ 100% audit log capture rate
- ✅ <500ms p99 latency for security checks
- ✅ 0 unauthorized command executions
- ✅ Configuration validation prevents security bypasses
- ✅ Approval workflow completes in <2 minutes

---

## Next Steps

Start with Week 1: Integration Layer
- Add security dependency to botticelli_social
- Implement SecureDiscordExecutor wrapper
- Update BotCommandRegistry
- Load security config from TOML

**Ready to proceed when approved!**
