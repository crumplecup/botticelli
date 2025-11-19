# Error Type Derives Analysis

**Date:** 2025-11-19  
**Question:** Are we deriving all possible traits for error types?

## Current State

### ErrorKind Enums (5 types)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
```
✅ Has: Debug, Clone, PartialEq, Eq, Hash
❌ Missing: PartialOrd, Ord

### Wrapper Error Structs (10 types)
```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
```
✅ Has: Debug, Clone
❌ Missing: PartialEq, Eq, Hash, PartialOrd, Ord

## CLAUDE.md Policy

> "Data structures should derive Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, and Hash if possible."

**Target:** Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash

## Analysis: Can Error Types Derive These?

### Copy - ❌ Not Possible

Error structs contain `String` (heap-allocated, not Copy).

**Conclusion:** Cannot derive Copy. Correct as-is.

### Clone - ✅ Already Have

All error types derive Clone.

**Conclusion:** Correct.

### Debug - ✅ Already Have

All error types derive Debug.

**Conclusion:** Correct.

### PartialEq - ❓ Should We?

**Capability:**
- ErrorKind enums: ✅ Can derive (already do)
- Wrapper structs: ✅ Can derive (String is PartialEq)

**Use cases:**
- Test assertions: `assert_eq!(err.kind(), &ExpectedKind)`
- Error matching: `if err.kind() == &SpecificError`
- Deduplication: Removing duplicate errors

**Current status:**
- ErrorKind enums: ✅ Have it
- Wrapper structs: ❌ Missing it

**Issue:** Wrapper structs have `line` and `file` fields. Two errors with same kind but different locations would be unequal, which may not be desired for error matching.

**Options:**
1. Derive PartialEq - errors from different lines are different
2. Don't derive - force manual comparison of kind only
3. Implement PartialEq manually to compare only kind

**Recommendation:** Derive PartialEq for wrapper structs. Location tracking is part of the error's identity.

### Eq - ❓ Should We?

**Capability:**
- ErrorKind enums: ✅ Can derive (already do)
- Wrapper structs: ✅ Can derive (String is Eq)

**Depends on:** PartialEq must be implemented first

**Recommendation:** Derive Eq if we derive PartialEq.

### Hash - ❓ Should We?

**Capability:**
- ErrorKind enums: ✅ Can derive (already do)
- Wrapper structs: ✅ Can derive (String is Hash)

**Use cases:**
- HashMap/HashSet keys
- Error deduplication in collections
- Error frequency counting

**Depends on:** Eq must be implemented first

**Recommendation:** Derive Hash for wrapper structs. Enables collection-based error handling.

### PartialOrd - ❓ Should We?

**Capability:**
- ErrorKind enums: ✅ Can derive (String is PartialOrd)
- Wrapper structs: ✅ Can derive (String is PartialOrd)

**Use cases:**
- Sorting errors (by severity? by kind name?)
- Priority queues
- Ordered error reports

**Current derive order:**
```rust
pub enum StorageErrorKind {
    NotFound(String),
    PermissionDenied(String),
    Io(String),
    // ...
}
```

With derive PartialOrd, NotFound < PermissionDenied < Io (enum variant order).

**Question:** Does variant order represent meaningful ordering?
- ❌ Probably not - NotFound isn't "less than" PermissionDenied
- ✅ But consistent ordering is useful for sorting/display

**Recommendation:** Derive PartialOrd. Variant order provides consistent (if arbitrary) ordering for sorting.

### Ord - ❓ Should We?

**Capability:**
- ErrorKind enums: ✅ Can derive
- Wrapper structs: ✅ Can derive

**Depends on:** PartialOrd and Eq must be implemented first

**Recommendation:** Derive Ord if we derive PartialOrd.

## Comparison: std::io::Error

Let's check what std library does:

```rust
pub struct Error { /* private fields */ }
```

**std::io::Error derives:**
- Debug ✅
- Clone ❌ (not Clone)
- PartialEq ❌ (not PartialEq)
- Eq ❌ (not Eq)
- Hash ❌ (not Hash)
- PartialOrd ❌ (not PartialOrd)
- Ord ❌ (not Ord)

**std only provides:**
- Display, Debug, Error (core traits)
- From implementations (conversions)

**Reasoning:**
- std::io::Error is not comparable or hashable
- Errors are typically propagated, not collected/compared
- Error comparison is rarely meaningful

## Comparison: anyhow::Error

```rust
pub struct Error { /* private fields */ }
```

**anyhow::Error derives:**
- Debug ✅
- Display ✅
- Clone ❌ (not Clone)
- PartialEq ❌ (not PartialEq)

**Reasoning:**
- Focus on ergonomic error propagation
- Not collection/comparison use cases

## Use Case Analysis for Our Project

### Current Usage Patterns

Do we:
1. **Compare errors?** Check tests for `assert_eq!` on errors
2. **Store in collections?** Check for `HashSet<Error>` or `HashMap<Error, _>`
3. **Sort errors?** Check for `.sort()` on error collections
4. **Deduplicate errors?** Check for unique error tracking

Let me check:

```bash
# Check for error comparisons in tests
grep -r "assert_eq.*Error" tests/

# Check for error collections
grep -r "HashSet.*Error\|HashMap.*Error" crates/

# Check for error sorting
grep -r "\.sort.*error\|error.*\.sort" crates/
```

### Use Case: Testing

In tests, we often want:
```rust
let err = some_function().unwrap_err();
assert_eq!(err.kind(), &ExpectedErrorKind);
```

This requires PartialEq on ErrorKind ✅ (already have).
But comparing whole errors (including location) is rarely useful.

### Use Case: Error Aggregation

If we collect errors:
```rust
let mut errors: HashSet<ErrorKind> = HashSet::new();
errors.insert(error.kind().clone());
```

This requires Hash on ErrorKind ✅ (already have).
Wrapper struct doesn't need Hash for this pattern.

## Recommendations

### ErrorKind Enums - Add PartialOrd, Ord

**Current:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
```

**Recommended:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
```

**Rationale:**
- Enables sorting for consistent error reporting
- Low cost (just variant order)
- Follows CLAUDE.md policy
- Consistent with other enum types

### Wrapper Error Structs - Keep Minimal

**Current:**
```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
```

**Recommended:** Keep as-is

**Rationale:**
- Errors are typically propagated, not compared
- Location tracking makes comparison less useful
- Following std library precedent
- PartialEq would compare locations (confusing)
- Hash would hash locations (rarely useful)

**Exception:** If use cases emerge, add derives then.

## Updated CLAUDE.md Policy

Current policy is too broad:

> "Data structures should derive Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, and Hash if possible."

Should be refined to:

> "Data structures should derive Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, and Hash if possible."
>
> **Exception for error types:**
> - ErrorKind enums: Derive all comparison and ordering traits
> - Wrapper error structs: Derive only Debug, Clone (+ derive_more traits)
> - Rationale: Wrapper structs have location tracking; comparing locations is rarely meaningful

## Action Items

### Immediate

1. Add PartialOrd, Ord to all ErrorKind enums
2. Keep wrapper structs minimal (Debug, Clone only)
3. Update CLAUDE.md with error type exception

### Verification

After changes:
```bash
# ErrorKind enums should have 8 standard derives
grep "^#\[derive.*ErrorKind" -A 1 crates/botticelli_error/src/*.rs

# Wrapper structs should have 2 standard derives + derive_more
grep "^#\[derive.*Error\]" -B 1 crates/botticelli_error/src/*.rs | grep -v Kind
```

## Conclusion

**Question:** Are we deriving all possible traits?

**Answer:** 
- ✅ Wrapper structs: Correct as-is (Debug, Clone only)
- ❌ ErrorKind enums: Missing PartialOrd, Ord (should add)

**Rationale:**
- ErrorKind enums benefit from full derives (comparison, sorting, hashing)
- Wrapper structs should be minimal (location tracking makes comparison confusing)
- Follows std library precedent (std::io::Error isn't PartialEq)

**Action:** Add PartialOrd, Ord to ErrorKind enums only.
