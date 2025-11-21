# TUI Architecture Refactoring Plan

## Problem Statement

The botticelli_tui crate currently has database functionality tightly coupled throughout the codebase, making it impossible to compile without the database feature. This violates the principle of feature independence and creates unnecessary dependencies.

### Current Issues

1. **Database coupling in core App struct** - The `App` struct in `app.rs` has database-specific fields (`content_rows`, `selected_content`, etc.) mixed with UI state
2. **View functions require database** - Functions like `draw_detail_view`, `draw_edit_view`, etc. in `ui.rs` are unused without database but still reference database types
3. **Feature gate proliferation** - Individual functions are feature-gated rather than organizing code by feature domain
4. **Compilation warnings** - Without database feature, we get numerous unused import/function warnings
5. **Architectural confusion** - No clear separation between TUI framework code and database-specific views

### Root Cause

The TUI was initially designed only for database management, so database functionality was baked into the core architecture. As we expand the TUI to support other features (narrative execution, bot management, etc.), this tight coupling becomes a major impediment.

## Strategic Solution

### Principle: Module-Level Feature Gates

Instead of feature-gating individual functions, we organize code into feature-specific modules that can be cleanly enabled/disabled:

```
botticelli_tui/
├── src/
│   ├── lib.rs              # Core exports, minimal dependencies
│   ├── app.rs              # Core App trait/interface (no features)
│   ├── events.rs           # Event handling (no features)
│   ├── ui.rs               # Basic UI utilities (no features)
│   ├── database/           # #[cfg(feature = "database")]
│   │   ├── mod.rs
│   │   ├── app.rs          # DatabaseApp struct
│   │   ├── views.rs        # Database-specific views
│   │   └── handlers.rs     # Database event handlers
│   ├── narrative/          # #[cfg(feature = "narrative")]
│   │   ├── mod.rs
│   │   ├── app.rs          # NarrativeApp struct
│   │   └── views.rs
│   └── bot/                # #[cfg(feature = "discord")]
│       ├── mod.rs
│       └── ...
```

### Benefits

1. **Clean feature boundaries** - Each feature lives in its own module
2. **No scattered gates** - One `#[cfg(feature = "...")]` at module level instead of hundreds
3. **Compile efficiency** - Unused features don't even parse their code
4. **Clear ownership** - Easy to see what code belongs to which feature
5. **Testing simplicity** - Test each feature module independently

## Implementation Strategy

### Phase 1: Extract Database-Specific App State

Create `database/app.rs` with:

```rust
#[cfg(feature = "database")]
pub struct DatabaseApp {
    // Database-specific state
    content_rows: Vec<ContentRow>,
    selected_content: Option<usize>,
    current_table: String,
    // ...
}
```

### Phase 2: Create Feature-Specific View Modules

Move database views from `ui.rs` to `database/views.rs`:

```rust
#[cfg(feature = "database")]
pub fn draw_database_list_view(f: &mut Frame, app: &DatabaseApp, area: Rect) {
    // Implementation
}
```

### Phase 3: Define Core App Trait

Create a trait that all feature-specific apps implement:

```rust
pub trait TuiApp {
    fn handle_event(&mut self, event: Event) -> Result<(), TuiError>;
    fn render(&self, f: &mut Frame, area: Rect);
    fn should_quit(&self) -> bool;
}
```

### Phase 4: Feature Selection at Runtime

In `lib.rs` or main binary:

```rust
pub enum AppVariant {
    #[cfg(feature = "database")]
    Database(DatabaseApp),
    
    #[cfg(feature = "narrative")]
    Narrative(NarrativeApp),
    
    #[cfg(feature = "discord")]
    Bot(BotApp),
}

impl TuiApp for AppVariant {
    fn handle_event(&mut self, event: Event) -> Result<(), TuiError> {
        match self {
            #[cfg(feature = "database")]
            Self::Database(app) => app.handle_event(event),
            // ...
        }
    }
    // ...
}
```

## Testing Strategy

Each feature module should have its own tests in `tests/`:

```
tests/
├── database_app_test.rs       # #[cfg(feature = "database")]
├── narrative_app_test.rs      # #[cfg(feature = "narrative")]
└── core_test.rs               # No feature requirements
```

Use cargo-hack to verify feature independence:

```bash
just check-features  # Tests all feature combinations
```

## Migration Path

1. ✅ Create `database/` module structure
2. ✅ Move database-specific App fields to `DatabaseApp`
3. ✅ Move database views to `database/views.rs`
4. ✅ Move database event handlers to `database/handlers.rs`
5. ⏳ Define core `TuiApp` trait
6. ⏳ Implement `TuiApp` for `DatabaseApp`
7. ⏳ Create `AppVariant` enum for feature selection
8. ⏳ Update main binary to use feature-selected app
9. ⏳ Remove database-specific code from core `app.rs`, `ui.rs`
10. ⏳ Add tests for each feature module
11. ⏳ Verify `cargo check --no-default-features` passes cleanly

## Success Criteria

- ✅ `cargo check --no-default-features -p botticelli_tui` compiles with zero warnings
- ✅ `just check-features` passes for all feature combinations
- ✅ Each feature module is self-contained and independently testable
- ✅ Adding new features (narrative, bot) doesn't require modifying core TUI code
- ✅ Documentation clearly describes feature organization

## Future Features

Once this architecture is in place, adding new TUI features becomes straightforward:

- **Narrative execution view** - Monitor running narratives, see progress, inspect outputs
- **Bot management view** - Configure bot commands, view security policies, test bot interactions
- **Configuration editor** - Edit botticelli.toml with validation and preview
- **Log viewer** - Real-time tracing output with filtering and search

Each feature gets its own module under `src/` and registers itself with the core TUI framework.
