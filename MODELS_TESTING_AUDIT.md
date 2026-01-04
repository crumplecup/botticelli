# Models Testing Audit

## Executive Summary

**Current State:** Integration tests at workspace level (groq_backend_integration_test.rs works)
**Unit Tests:** None in _models crate currently
**Status:** After refactoring, need to build comprehensive unit test suite

## Test Inventory

### ✅ Existing Tests (Working)

#### 1. **groq_backend_integration_test.rs** (workspace level)
- **Status:** Working after refactoring
- **Value:** Tests real Groq API through MCP backend
- **Action:** Keep as-is, already uses proper error handling and tracing
- **Priority:** DONE

### 📝 Unit Tests Needed (No tests currently exist)

### 📝 Missing Tests (Gaps)

#### Critical Gaps

1. **Error Conversion Chain Tests**
   - Test OllamaErrorKind → OllamaError → ModelsError → BotticelliError
   - Test GeminiErrorKind → GeminiError → ModelsError → BotticelliError
   - Test OpenAIErrorKind → OpenAIError → ModelsError → BotticelliError
   - Validate chained conversion macros work correctly
   - **Priority:** CRITICAL

2. **External Error Capture Tests**
   - Verify tiktoken_rs::CoreBPE captured in Arc
   - Verify serde_json::Error captured correctly
   - Verify reqwest::Error captured correctly
   - Verify tungstenite::Error captured correctly
   - Verify ollama_rs::error::OllamaError captured correctly
   - Verify gemini_rust::GeminiError captured correctly
   - **Priority:** CRITICAL

3. **Trait Implementation Tests**
   - BotticelliDriver for all providers (Gemini, Ollama, OpenAI, Groq, HuggingFace, Anthropic)
   - Streaming for all providers that support it
   - TokenCounting for all providers
   - ToolCalling for providers that support it
   - Vision for providers that support it
   - Metadata for all providers
   - **Priority:** HIGH

4. **Builder Error Tests**
   - Test GenerateResponseBuilder errors convert to string properly
   - Test MessageBuilder errors convert to string properly
   - Test TierConfigBuilder errors in rate_limit context
   - **Priority:** HIGH

5. **OpenAI-Compatible Provider Tests**
   - Groq client end-to-end
   - HuggingFace client end-to-end
   - Verify both use OpenAIError properly
   - Verify provider_name() returns correct values
   - **Priority:** MEDIUM

6. **Streaming Error Handling**
   - Test stream interruption handling
   - Test partial response handling
   - Test channel errors in async streams
   - **Priority:** MEDIUM

7. **ModelCapabilities Tests**
   - Test capabilities() returns correct values for each provider
   - Test max_tokens, supports_streaming, supports_tools, etc.
   - **Priority:** MEDIUM

8. **DTO Serialization/Deserialization**
   - Test all OpenAI DTOs with derive_getters
   - Test ChatMessage, ChatChoice, ChatUsage, ChatResponse
   - Test error cases (missing fields, invalid data)
   - **Priority:** LOW

#### Nice-to-Have Gaps

9. **Gemini Live Protocol**
   - Test all protocol message types
   - Test handshake flow
   - Test error handling
   - **Priority:** LOW (if experimental)

10. **Rate Limit Detection Integration**
   - Test rate limit detection with real provider responses
   - **Priority:** LOW (covered in _rate_limit crate)

11. **Token Counting Edge Cases**
   - Test unicode handling
   - Test very long inputs
   - Test malformed inputs
   - **Priority:** LOW

## Testing Strategy

### Immediate Actions (Week 1)

1. **Delete busywork:**
   - Remove .wip file
   - Remove duplicate/obsolete tests
   - Clean up test directory

2. **Fix critical tests:**
   - Update gemini_integration_test.rs error handling
   - Update ollama_client_test.rs error handling
   - Fix gemini_streaming_test.rs for new Streaming trait

3. **Add critical gap tests:**
   - Error conversion chain tests
   - External error capture tests

### Short-term (Week 2-3)

4. **Expand trait coverage:**
   - Test all trait implementations for all providers
   - Add ModelCapabilities tests

5. **Fix medium-priority tests:**
   - Update anthropic_api_test.rs
   - Rename and fix openai_compat tests
   - Update token_counting tests

### Medium-term (Month 1-2)

6. **Add provider-specific tests:**
   - Groq end-to-end
   - HuggingFace end-to-end
   - Anthropic edge cases

7. **Add streaming tests:**
   - Stream interruption handling
   - Partial responses
   - Error propagation

### Long-term (As needed)

8. **Polish:**
   - DTO serialization edge cases
   - Gemini Live protocol (if feature is used)
   - Token counting edge cases

## Test Quality Standards

All new/updated tests must:

1. **Use proper error types**
   - Return `BotticelliResult<()>` or specific error types
   - NEVER `Box<dyn std::error::Error>`

2. **Include tracing**
   - Call `init_tracing()` helper
   - Use structured logging for debugging

3. **Be deterministic**
   - Use mocks where possible
   - Mark API tests with `#[cfg_attr(not(feature = "api"), ignore)]`

4. **Test errors**
   - Test happy path AND error paths
   - Verify error types and messages

5. **Document purpose**
   - Clear docstring explaining what and why
   - Document any setup requirements

## Metrics

- **Current:** 1 integration test file at workspace level (groq), 0 unit tests in _models
- **After gap filling:** ~8-10 unit test files, ~1500-2000 lines (estimate)
- **Coverage target:** 70%+ for public APIs, 50%+ overall (realistic given external API dependencies)

## Current Status

✅ **Completed:**
- Error refactoring (DONE)
- Trait definitions stable (DONE)  
- Provider implementations stable (DONE)
- One working integration test (groq)

🔄 **In Progress:**
- Building unit test suite from scratch

## Next Steps

1. Create unit tests for error conversion chains (CRITICAL)
2. Create unit tests for external error capture (CRITICAL)
3. Add trait implementation tests (HIGH)
4. Add provider-specific unit tests (MEDIUM)

## Notes

- Many tests use API keys (rate-limited, cost money)
- Prefer mocking over live API calls where possible
- Keep API tests minimal and focused
- Consider test doubles for external dependencies
