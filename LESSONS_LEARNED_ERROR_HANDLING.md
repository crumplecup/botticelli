# Lessons Learned: Error Handling with derive_more

**Date:** 2025-11-19  
**Context:** Refactoring botticelli_error and updating CLAUDE.md

## The Problem

Despite having guidance in CLAUDE.md to "use derive_more when convenient," the codebase accumulated significant technical debt:

- **15 error types** with manual `Display` implementations
- **10 error types** with manual `Error` implementations  
- **193 lines** of unnecessary boilerplate code
- **Inconsistent patterns** across error types
- **Audit failures** - violations went undetected

### Root Causes

1. **Vague guidance**: "when convenient" is subjective and easily ignored
2. **Contradictory examples**: CLAUDE.md examples showed manual implementations
3. **No enforcement**: No audit checklist to catch violations
4. **No prohibition**: Manual implementations weren't explicitly forbidden
5. **Convenience trap**: Manual implementations are "easy" in the moment

## The Solution

### 1. Mandatory Requirements (Not Suggestions)

**Before:**
> "Use the derive_more crate to implement Display and Error when convenient."

**After:**
> "**MANDATORY:** All error types MUST use `derive_more::Display` and `derive_more::Error`. Manual implementations are NOT allowed."

**Why this matters:**
- "Must" vs "should" - no ambiguity
- "Not allowed" vs "when convenient" - explicit prohibition
- Bold and prominent - impossible to miss

### 2. Explicit Audit Checklist

Added 8-point checklist for audits:

```
1. ✅ NO manual `impl std::fmt::Display` for error types
2. ✅ NO manual `impl std::error::Error` for error types
3. ✅ ALL error structs use `derive_more::Display` with `#[display(...)]`
4. ✅ ALL error structs use `derive_more::Error`
5. ✅ ALL ErrorKind enum variants have `#[display(...)]` attributes
6. ✅ ALL error constructors use `#[track_caller]`
7. ✅ ALL crate-level error enums use `derive_more::From`
8. ✅ ALL from attributes are explicit: `#[from(ErrorType)]`
```

**Critical addition:**
> "If you find manual Display/Error implementations during audit: Flag as critical issue requiring immediate refactoring."

### 3. Clear Patterns with Examples

Replaced contradictory examples with 4 clear patterns:

1. **Pattern 1:** Simple error structs (message + location)
2. **Pattern 2:** ErrorKind enums (specific conditions)
3. **Pattern 3:** Wrapper error structs (kind + location)
4. **Pattern 4:** Crate-level aggregation

Each pattern has:
- Clear use case
- Complete example using derive_more
- Key requirements list
- What NOT to do

### 4. Reference Implementation

Instead of inline examples that could become outdated:

> "See `crates/botticelli_error` for a complete, production-ready reference implementation of all error patterns using derive_more."

Benefits:
- Living documentation (always up-to-date)
- Full working examples
- Can run tests to verify
- Shows real-world complexity

## Implementation Results

### Metrics

```
Files updated:     15 error modules
Lines removed:     193 boilerplate lines
Code reduction:    20% (967 → 774 lines)
Time to refactor:  ~2 hours
Test failures:     0
```

### Benefits Realized

1. **Less boilerplate** - 193 lines removed
2. **More declarative** - Error messages in attributes
3. **Easier maintenance** - Change attribute vs entire impl
4. **Consistent patterns** - All errors follow same approach
5. **Better audits** - Clear criteria to check

### Pattern Breakdown

| Pattern | Files | Display | Error | Lines Saved |
|---------|-------|---------|-------|-------------|
| Simple structs | 5 | 5 | 5 | ~40 |
| Wrapper structs | 5 | 5 | 5 | ~40 |
| ErrorKind enums | 5 | 5 | 0 | ~113 |
| **Total** | **15** | **15** | **10** | **~193** |

## Why This Keeps Happening

### Psychological Factors

1. **Immediate convenience** - Manual impl is "quick" right now
2. **Future burden invisible** - Maintenance cost not apparent
3. **Pattern blindness** - Copy existing (bad) patterns
4. **Guidance fatigue** - Skip reading long documents

### Structural Factors

1. **Soft requirements** - "Should" gets ignored under pressure
2. **Missing enforcement** - No automated checks
3. **Example quality** - Outdated examples propagate bad patterns
4. **Audit gaps** - No clear criteria means violations slip through

## Prevention Strategy

### Making Requirements Stick

1. **Use imperative language**
   - ❌ "Consider using derive_more"
   - ✅ "MUST use derive_more"

2. **Explicit prohibitions**
   - ❌ "Prefer derive_more over manual implementations"
   - ✅ "Manual implementations are NOT allowed"

3. **Make it prominent**
   - ❌ Buried in paragraph
   - ✅ Bold header: "MANDATORY: ALWAYS Use derive_more"

4. **Provide audit checklist**
   - ❌ "Check for good practices"
   - ✅ "8-point checklist with specific things to verify"

### Documentation Best Practices

1. **Show correct way, not wrong way**
   - Examples should only show the required pattern
   - Don't show manual implementations for comparison
   - Reference existing code as examples

2. **Make guidelines searchable**
   - Use consistent terminology
   - Add section headers for ctrl-F
   - Create audit checklists

3. **Regular updates**
   - Update examples when patterns evolve
   - Remove outdated guidance
   - Add lessons learned

4. **Reference real code**
   - Point to production implementations
   - Link to working examples
   - Living documentation stays current

## Wider Applicability

This lesson applies beyond error handling:

### Other Areas to Audit

1. **Import patterns** - Are we using crate-level imports consistently?
2. **Derive policies** - Are we maximizing trait derives?
3. **Module organization** - Is lib.rs only mod/pub use?
4. **Documentation** - Do all public items have docs?
5. **Testing** - Are tests in tests/ directory?

### General Principles

1. **Make requirements mandatory, not optional**
2. **Use imperative language ("must", not "should")**
3. **Provide audit checklists**
4. **Show correct patterns only**
5. **Reference production code**
6. **Update based on violations**

## Action Items

### Immediate

- [x] Update CLAUDE.md with mandatory derive_more requirements
- [x] Add explicit audit checklist
- [x] Remove contradictory examples
- [x] Point to botticelli_error as reference

### Future Audits

When auditing any crate, check for:
- Manual Display implementations on errors
- Manual Error implementations on errors
- Missing derive_more usage
- Flag as **CRITICAL** if found

### Code Reviews

Require derive_more for all new error types:
- Block PRs with manual implementations
- Point reviewer to CLAUDE.md section
- Reference botticelli_error patterns

### Continuous Improvement

- Monitor for similar patterns elsewhere
- Update CLAUDE.md when violations found
- Create automation where possible
- Share lessons learned

## Key Takeaways

### For Documentation Writers

1. **Be explicit**: "Must" not "should"
2. **Be prominent**: Bold, headers, cant-miss
3. **Be specific**: Checklists, not vague guidance
4. **Be current**: Reference real code

### For Code Authors

1. **Read the docs**: Especially mandatory sections
2. **Follow patterns**: Use existing code as reference
3. **Question convenience**: "Quick" now = debt later
4. **Ask when unsure**: Better than guessing

### For Auditors

1. **Use checklists**: Don't rely on memory
2. **Be thorough**: Check all requirements
3. **Flag violations**: Even "small" ones
4. **Update docs**: When you find gaps

## Conclusion

The derive_more refactoring was not just about removing 193 lines of code. It was about:

1. **Discovering a pattern of violations** (15 instances)
2. **Understanding why they occurred** (soft guidance)
3. **Fixing the root cause** (mandatory requirements)
4. **Preventing future occurrences** (audit checklist)
5. **Sharing the lessons** (this document)

The updated CLAUDE.md now has:
- ✅ Mandatory requirements (not suggestions)
- ✅ Explicit prohibitions (not preferences)
- ✅ Audit checklist (not vague guidance)
- ✅ Reference implementation (not outdated examples)

**Result:** Future error types will follow best practices from day one, preventing accumulation of technical debt.

---

**Document Created:** 2025-11-19  
**Related Changes:**
- CLAUDE.md: Error Handling section rewritten
- botticelli_error: Complete refactoring (15 types, 193 lines)
- Documentation: DERIVE_MORE_IMPLEMENTATION.md, DERIVE_MORE_OPPORTUNITIES.md
