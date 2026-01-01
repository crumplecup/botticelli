# Error Macro Pattern

Inspired by the arcgis error handling approach, we've added two macros to reduce boilerplate when integrating external errors.

## Macros

### `bridge_error!`

Creates the conversion chain from external errors through wrapper errors to ErrorKind:

```rust
// ExternalError → WrapperError → ErrorKind
bridge_error!(reqwest::Error => HttpError => BotticelliErrorKind);
```

**What it generates:**
```rust
impl From<reqwest::Error> for BotticelliErrorKind {
    #[track_caller]
    fn from(err: reqwest::Error) -> Self {
        HttpError::from(err).into()
    }
}
```

### `error_from!`

Creates the final conversion to the top-level Error with tracing:

```rust
// SourceError → Error (with logging)
error_from!(HttpError => BotticelliError);
```

**What it generates:**
```rust
impl From<HttpError> for BotticelliError {
    #[track_caller]
    fn from(err: HttpError) -> Self {
        let kind = err.into();
        tracing::error!(error_kind = %kind, "Error created");
        Self(Box::new(kind))
    }
}
```

## Usage Example

### 1. Create a Wrapper Error Type

```rust
// In crates/botticelli_error/src/url.rs
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("URL Error: {} at {}:{}", message, file, line)]
pub struct UrlError {
    message: String,
    line: u32,
    file: &'static str,
}

impl UrlError {
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}

// Add From for the external error
impl From<url::ParseError> for UrlError {
    #[track_caller]
    fn from(err: url::ParseError) -> Self {
        Self::new(err.to_string())
    }
}
```

### 2. Add to ErrorKind

```rust
// In crates/botticelli_error/src/error.rs
#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum BotticelliErrorKind {
    // ... existing variants ...
    
    #[from(UrlError)]
    Url(UrlError),
}
```

### 3. Use the Macros

```rust
// At the bottom of error.rs

// Bridge: url::ParseError → UrlError → BotticelliErrorKind
bridge_error!(url::ParseError => UrlError => BotticelliErrorKind);

// Final: UrlError → BotticelliError (with tracing)
error_from!(UrlError => BotticelliError);
```

### 4. Now `?` Just Works

```rust
use botticelli_error::BotticelliResult;
use url::Url;

fn parse_endpoint(s: &str) -> BotticelliResult<Url> {
    let url = Url::parse(s)?;  // url::ParseError → BotticelliError automatically!
    Ok(url)
}
```

## Benefits

1. **Automatic location tracking** - `#[track_caller]` propagates caller location
2. **Automatic tracing** - Every error creation is logged at ERROR level
3. **Type safety** - Strong typing all the way through
4. **Minimal boilerplate** - Two macro calls instead of multiple From impls
5. **Observability** - `error_kind` field in traces for filtering/analysis
6. **? operator support** - Seamless error propagation

## Pattern

The full conversion chain:

```
External Error (url::ParseError)
    ↓ From impl in wrapper
Wrapper Error (UrlError)              [has location]
    ↓ bridge_error! macro
ErrorKind (BotticelliErrorKind::Url)  [#[from] on variant]
    ↓ error_from! macro
Top-Level Error (BotticelliError)     [has tracing]
```

Every step:
- ✅ Tracks caller location with `#[track_caller]`
- ✅ Logs at error level in final conversion
- ✅ Works with `?` operator
- ✅ Provides structured error_kind field for queries

## Existing Implementations

Already set up for:
- ✅ `HttpError` - Wraps HTTP/reqwest errors (when `reqwest` feature enabled)
- ✅ `JsonError` - Wraps serde_json errors (when `serde_json` feature enabled)

Both use:
- Private fields with `derive_getters::Getters`
- `#[track_caller]` on constructors and From impls
- Consistent `"message at file:line"` display format

## Adding New External Errors

1. Create wrapper type with location tracking
2. Add variant to BotticelliErrorKind with `#[from]`
3. Call `bridge_error!` macro
4. Call `error_from!` macro
5. Done! The `?` operator now handles it automatically.
