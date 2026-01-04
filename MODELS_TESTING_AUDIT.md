# Models Testing Audit

## Executive Summary

**Current State:** ~2900 lines of tests across 24 test files
**Status:** Many tests outdated after refactoring, some busywork, significant gaps in coverage

## Test Inventory

### ✅ Keep & Fix (High Value)

#### 1. **gemini_integration_test.rs** (~200 lines)
- **Status:** Good structure, needs updates for new error types
- **Value:** Tests real Gemini API with model selection
- **Action:** Update error handling to use new error types, add tracing
- **Priority:** HIGH

#### 2. **ollama_client_test.rs** (~100 lines)
- **Status:** Basic coverage, needs expansion
- **Value:** Tests local Ollama integration
- **Action:** Update error types, add streaming tests, add trait coverage
- **Priority:** HIGH

#### 3. **token_counting_test.rs**
- **Status:** Needs update for new error types
- **Value:** Critical for cost tracking and rate limiting
- **Action:** Update to use ModelsError, add edge cases
- **Priority:** HIGH

#### 4. **capabilities_test.rs**
- **Status:** New capability system needs testing
- **Value:** Tests trait system integration
- **Action:** Expand to cover all providers, add ModelCapabilities
- **Priority:** MEDIUM

### 🔄 Fix or Replace

#### 5. **anthropic_api_test.rs** (~150 lines)
- **Issue:** Uses `Box<dyn std::error::Error>` anti-pattern
- **Action:** Rewrite to use BotticelliResult, add tracing
- **Priority:** MEDIUM

#### 6. **gemini_streaming_test.rs**
- **Issue:** Outdated after streaming refactor
- **Action:** Update for new Streaming trait with Error type
- **Priority:** HIGH

#### 7. **gemini_live_*.rs** (3 files)
- **Issue:** WebSocket streaming, unclear if working after refactor
- **Action:** Audit carefully, may need significant rework
- **Priority:** MEDIUM (if feature is used) / LOW (if experimental)

#### 8. **openai_compat_provider_test.rs**
- **Issue:** Name outdated (should be openai_provider_test.rs)
- **Action:** Rename, update to OpenAI* types, add error handling tests
- **Priority:** MEDIUM

### ❌ Delete (Busywork / Duplicates)

#### 9. **anthropic_api_test.rs.wip**
- **Reason:** WIP file, incomplete
- **Action:** DELETE

#### 10. **gemini_mock_test.rs**
- **Issue:** Unclear value if integration tests cover same ground
- **Action:** Review if mocking provides unique value, otherwise DELETE
- **Priority:** Evaluate first

#### 11. **rate_limit_detection_test.rs**
- **Issue:** Rate limiting now in _rate_limit crate with own tests
- **Action:** DELETE if duplicate, or move to _rate_limit tests
- **Priority:** Check for overlap

#### 12. **model_family_test.rs** & **model_selector_test.rs**
- **Issue:** Unclear if model family/selector features still exist
- **Action:** DELETE if features removed
- **Priority:** Verify features exist

#### 13. **selection_test.rs**
- **Issue:** Possibly duplicate of model_selector tests
- **Action:** DELETE if redundant

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

- **Current:** 24 test files, ~2900 lines
- **After cleanup:** ~15 test files, ~2000 lines (estimate)
- **After gap filling:** ~20 test files, ~3500 lines (estimate)
- **Coverage target:** 80%+ for public APIs, 60%+ overall

## Dependencies

- Error refactoring (DONE)
- Trait definitions stable (DONE)
- Provider implementations stable (IN PROGRESS)

## Notes

- Many tests use API keys (rate-limited, cost money)
- Prefer mocking over live API calls where possible
- Keep API tests minimal and focused
- Consider test doubles for external dependencies
