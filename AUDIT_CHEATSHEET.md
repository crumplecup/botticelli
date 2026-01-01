# Audit Cheatsheet

Lessons learned from botticelli_core audit (2026-01-01). Use this checklist to catch patterns that slip through initial review.

## Quick Scan Checklist

Run these greps first, fix issues before detailed review:

```bash
# 1. Box<dyn Error> anti-pattern
rg "Box<dyn.*Error>" crates/*/src crates/*/tests

# 2. Manual builders (should use derive_builder)
rg "pub fn builder\(\)" crates/*/src

# 3. Manual impl Display/Error (should use derive_more)
rg "impl.*Display.*for" crates/*/src
rg "impl.*Error.*for" crates/*/src

# 4. println! in tests (should use tracing)
rg "println!" crates/*/tests

# 5. Tests without tracing initialization
rg "#\[test\]" crates/*/tests -A 5 | grep -v "init_test_tracing\|use tracing"

# 6. Public struct fields (should be private with getters)
rg "pub struct.*\{" crates/*/src -A 3 | grep "pub [a-z]"
```

## Error Handling Patterns

### ❌ Common Mistakes

1. **Box<dyn Error> anywhere**
   ```rust
   // ❌ In library code
   fn foo() -> Result<T, Box<dyn Error>>
   
   // ❌ In tests
   fn test_foo() -> Result<(), Box<dyn std::error::Error>>
   ```

2. **Manual error implementations**
   ```rust
   // ❌ Manual Display
   impl Display for MyError {
       fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { ... }
   }
   
   // ❌ Manual Error
   impl std::error::Error for MyError { ... }
   ```

3. **String as error type**
   ```rust
   // ❌ String errors
   fn get_tokenizer(model: &str) -> Result<Tokenizer, String>
   ```

### ✅ Correct Patterns

1. **Use library error types**
   ```rust
   // ✅ In library code
   fn foo() -> Result<T, MyError>
   
   // ✅ In tests
   fn test_foo() -> Result<(), MyError>
   ```

2. **Use derive_more**
   ```rust
   // ✅ Derived Display + Error
   #[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
   #[display("Error: {} at {}:{}", message, file, line)]
   pub struct MyError {
       pub message: String,
       pub file: &'static str,
       pub line: u32,
   }
   ```

3. **Create specific error types**
   ```rust
   // ✅ Proper error type
   #[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
   pub enum MyErrorKind {
       #[display("Not found: {}", _0)]
       NotFound(String),
   }
   ```

## Builder Patterns

### ❌ Common Mistakes

1. **Manual builder implementation**
   ```rust
   // ❌ Handrolled builder
   pub struct MyBuilder { ... }
   impl MyBuilder {
       pub fn new() -> Self { ... }
       pub fn field(mut self, val: T) -> Self { ... }
       pub fn build(self) -> Result<My, Error> { ... }
   }
   ```

2. **derive_new for complex types**
   ```rust
   // ❌ derive_new with many optional fields
   #[derive(derive_new::new)]
   pub struct Config {
       field1: String,
       field2: Option<u32>,
       field3: Option<bool>,
       field4: Option<String>,
   }
   ```

### ✅ Correct Patterns

1. **Use derive_builder for complex types**
   ```rust
   // ✅ For types with optional fields or validation
   #[derive(Debug, Clone, derive_builder::Builder)]
   pub struct Config {
       field1: String,
       #[builder(default)]
       field2: Option<u32>,
   }
   ```

2. **Use derive_new only for simple constructors**
   ```rust
   // ✅ derive_new for simple, non-optional construction
   #[derive(Debug, Clone, derive_new::new)]
   pub struct Point {
       x: f64,
       y: f64,
   }
   ```

## Struct Design

### ❌ Common Mistakes

1. **Public fields**
   ```rust
   // ❌ Direct field access
   pub struct Config {
       pub host: String,
       pub port: u16,
   }
   ```

2. **Manual getter/setter implementations**
   ```rust
   // ❌ Handwritten accessors
   impl Config {
       pub fn host(&self) -> &str { &self.host }
       pub fn set_host(&mut self, host: String) { self.host = host; }
   }
   ```

### ✅ Correct Patterns

1. **Private fields with derive_getters**
   ```rust
   // ✅ Encapsulated with derived getters
   #[derive(Debug, Clone, derive_getters::Getters)]
   pub struct Config {
       /// Host address
       host: String,
       /// Port number
       port: u16,
   }
   ```

2. **Add derive_setters for mutable config**
   ```rust
   // ✅ With setters when needed
   #[derive(Debug, Clone, derive_getters::Getters, derive_setters::Setters)]
   #[setters(prefix = "with_")]
   pub struct Config {
       host: String,
       #[setters(skip)]
       created_at: DateTime<Utc>,
   }
   ```

## Test Patterns

### ❌ Common Mistakes

1. **Tests without tracing**
   ```rust
   // ❌ Silent tests
   #[test]
   fn test_foo() {
       let result = foo();
       assert!(result.is_ok());
   }
   ```

2. **println! in tests**
   ```rust
   // ❌ Unstructured output
   #[test]
   fn test_foo() {
       println!("Testing foo");
       assert!(foo().is_ok());
   }
   ```

3. **Busywork tests**
   ```rust
   // ❌ Testing derive macros
   #[test]
   fn builder_works() {
       let _ = MyStruct::builder().field(1).build();
   }
   
   #[test]
   fn default_works() {
       let _ = MyStruct::default();
   }
   ```

4. **Global state without orchestration**
   ```rust
   // ❌ Multiple tests setting global state
   #[test]
   fn test_init_1() {
       init_observability(...); // Sets global
   }
   
   #[test]
   fn test_init_2() {
       init_observability(...); // Fails: already set
   }
   ```

### ✅ Correct Patterns

1. **Initialize tracing in all test files**
   ```rust
   // ✅ At top of every test file
   fn init_test_tracing() {
       let _ = ObservabilityConfig::builder()
           .service_name("my-tests")
           .exporter(ExporterBackend::Stdout)
           .enable_metrics(false)
           .log_level("debug")
           .build()
           .ok()
           .and_then(|config| init_observability_with_config(config).ok());
   }
   ```

2. **Use structured logging in tests**
   ```rust
   // ✅ Visible, parseable test output
   #[test]
   fn test_foo() -> Result<(), MyError> {
       init_test_tracing();
       use tracing::{debug, info};
       
       info!("Testing foo functionality");
       debug!(input = "test", "Calling foo");
       
       let result = foo("test")?;
       
       debug!(output = ?result, "Result received");
       info!("Test completed successfully");
       Ok(())
   }
   ```

3. **Test real functionality, not macros**
   ```rust
   // ✅ Test business logic
   #[test]
   fn validate_rejects_invalid_input() -> Result<(), MyError> {
       let config = Config::builder().value(-1).build()?;
       let result = config.validate();
       assert!(result.is_err());
       Ok(())
   }
   
   // ❌ Delete these
   // fn builder_works() { ... }
   // fn default_works() { ... }
   ```

4. **Orchestrator pattern for global state**
   ```rust
   // ✅ Single test orchestrator
   fn scenario_1() -> Result<(), MyError> { ... }
   fn scenario_2() -> Result<(), MyError> { ... }
   
   #[test]
   fn test_orchestrator() -> Result<(), MyError> {
       // Set up global state once
       init_global_state()?;
       
       // Run scenarios serially
       scenario_1()?;
       scenario_2()?;
       
       // Clean up
       cleanup_global_state();
       Ok(())
   }
   ```

## Type Organization

### ❌ Common Mistakes

1. **Traits in _core crate**
   ```rust
   // ❌ Wrong location
   // crates/botticelli_core/src/provider.rs
   pub trait Provider {
       fn generate(&self, request: Request) -> Result<Response>;
   }
   ```

2. **Types in _interface crate**
   ```rust
   // ❌ Wrong location
   // crates/botticelli_interface/src/config.rs
   pub struct Config { ... }
   ```

### ✅ Correct Patterns

1. **Traits go in _interface**
   ```rust
   // ✅ Correct location
   // crates/botticelli_interface/src/provider.rs
   pub trait Provider {
       fn generate(&self, request: Request) -> Result<Response>;
   }
   ```

2. **Types go in _core**
   ```rust
   // ✅ Correct location
   // crates/botticelli_core/src/config.rs
   pub struct Config { ... }
   ```

## Consolidation Opportunities

### 🔍 Look For Duplicates

1. **Similar error types**
   - Check for ErrorKind enums that overlap
   - Consolidate into umbrella error type in _error crate

2. **Similar data types**
   - Look for structs with same fields
   - Check usage patterns - one might be unused
   - Example: `TokenUsage` vs `TokenUsageData`

3. **Builder vs derive_new**
   - If both exist, pick one pattern
   - Use derive_new for simple, use derive_builder for complex

## Feature-Specific Checks

### Optional Dependencies

1. **Error types must be unconditional**
   ```rust
   // ❌ Error types behind features
   #[cfg(feature = "database")]
   pub enum DatabaseError { ... }
   
   // ✅ Always available
   pub enum DatabaseError {
       #[cfg(feature = "postgres")]
       Postgres(PostgresError),
       #[cfg(feature = "sqlite")]
       Sqlite(SqliteError),
   }
   ```

2. **botticelli_error must be required dependency**
   ```toml
   # ❌ Optional
   [dependencies]
   botticelli_error = { version = "0.1", optional = true }
   
   # ✅ Required
   [dependencies]
   botticelli_error = "0.1"
   ```

## Derives Audit

### Check Every Struct

```rust
// ✅ Standard derives for data types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MyData { ... }

// ✅ Builder for complex construction
#[derive(Debug, Clone, derive_builder::Builder)]
pub struct MyConfig { ... }

// ✅ Getters for encapsulation
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct MyState { ... }

// ✅ Setters when mutation needed
#[derive(Debug, Clone, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct MyMutableConfig { ... }

// ✅ Display + Error for errors
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Error: {}", message)]
pub struct MyError { ... }
```

## Usage Analysis

When reviewing a crate, check **where types are actually used**:

```bash
# Find all usages of a type
rg "TypeName" crates/ --type rust

# Check if type is in public API
rg "pub.*TypeName" crates/*/src/lib.rs

# Find test usage
rg "TypeName" crates/*/tests

# Check imports
rg "use.*TypeName" crates/
```

If a type has:
- ❌ No external usage → candidate for deletion
- ❌ Only internal usage → make private or merge
- ❌ Similar type with overlapping purpose → consolidate
- ✅ Clear public API usage → keep and ensure proper design

## Audit Order

1. **Quick scan** - Run grep commands above
2. **Error handling** - Check all Result types and error implementations
3. **Builders** - Audit construction patterns
4. **Struct design** - Check field visibility and derives
5. **Tests** - Verify tracing, error types, and value
6. **Type organization** - Traits vs types in correct crates
7. **Consolidation** - Look for duplicates and unused code
8. **Usage analysis** - Verify each public type is actually used
9. **Feature gates** - Check optional dependencies
10. **Final review** - Read through with fresh eyes

## Red Flags

Stop and investigate when you see:

- ❌ `Box<dyn Error>` anywhere
- ❌ `impl Display` or `impl Error` manually written
- ❌ `pub struct` with `pub` fields
- ❌ `println!` in tests
- ❌ Builder struct + impl manually written
- ❌ `#[allow(dead_code)]` or any `#[allow(...)]`
- ❌ Tests named `test_default` or `test_builder`
- ❌ Multiple types with similar names (e.g., `Data` vs `DataType`)
- ❌ Traits in `_core` or types in `_interface`
- ❌ String as error type
- ❌ `botticelli_error` as optional dependency

## Summary: What Got Missed

In the botticelli_core audit, initial pass missed:

1. ✅ **Box<dyn Error>** in tests (caught later)
2. ✅ **Manual builders** instead of derive_builder
3. ✅ **Public fields** instead of private + derives
4. ✅ **String errors** instead of proper error types
5. ✅ **Missing tracing** in tests
6. ✅ **println!** instead of structured logging
7. ✅ **Duplicate types** (TokenUsage vs TokenUsageData)
8. ✅ **Traits in wrong crate** (Provider in _core)
9. ✅ **Busywork tests** testing macros
10. ✅ **botticelli_error optional** instead of required

This cheatsheet captures those lessons. Use it for the next audit!
