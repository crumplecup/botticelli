# Automatic MCP Tool Generation from Elicit Derives

## Vision

The whole point of `Elicit` is to create MCP tools. Therefore, `#[derive(Elicit)]` should generate BOTH the Elicitation trait impl AND the MCP tool function automatically.

**Current state**:
```rust
#[derive(Elicit)]
struct Config {
    timeout: u32,
    mode: Mode,
}
```

Generates: Elicitation trait impl (client-side elicitation logic)

**Desired state**:
```rust
#[derive(Elicit)]
struct Config {
    timeout: u32,
    mode: Mode,
}
```

Generates BOTH:
1. Elicitation trait impl
2. MCP tool function with `#[rmcp::tool]`

**Generated code**:
```rust
// Existing: Elicitation trait impl
impl Elicitation for Config { /* ... */ }

// NEW: MCP tool function (automatic)
#[rmcp::tool]
pub async fn elicit_config(
    client: &rmcp::service::Peer<rmcp::service::RoleClient>,
) -> Result<Config, elicitation::ElicitError> {
    Config::elicit(&elicitation::ElicitClient::new(client)).await
}
```

## Why One Derive

**Philosophy**: Elicitation IS about creating MCP tools. Adding `#[elicit(tool)]` or `#[derive(ElicitTool)]` is redundant - it's a "hat on a hat". If you derive Elicit, you want the tool.

**Benefits**:
- Zero ceremony: One derive does everything
- Clear intent: Elicit = MCP-callable
- No opt-in confusion: If you derive it, you get the tool
- Consistent: Every Elicit type becomes an MCP tool

## Implementation Plan

### Phase 1: Extend Derive Macro to Always Generate Tool

**File**: `elicitation_derive/src/derive_elicit.rs`

Modify expand to always generate tool function:

```rust
pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let elicit_impl = match &input.data {
        Data::Enum(_) => crate::enum_impl::expand_enum(&input),
        Data::Struct(_) => crate::struct_impl::expand_struct(&input),
        // ...
    };
    
    // ALWAYS generate tool function
    let tool_impl = generate_tool_function(&input);
    
    quote! {
        #elicit_impl
        #tool_impl
    }
}
```

### Phase 2: Generate Tool Function

**New file**: `elicitation_derive/src/tool_gen.rs`

```rust
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn generate_tool_function(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let fn_name = format_ident!("elicit_{}", to_snake_case(&name.to_string()));
    
    // Extract custom tool name from #[elicit(tool = "name")]
    let tool_name = extract_tool_name(&input.attrs)
        .unwrap_or_else(|| fn_name.to_string());
    
    quote! {
        /// Auto-generated MCP tool for eliciting [`#name`].
        ///
        /// This function uses the derived `Elicitation` impl to
        /// interactively elicit a value from the user via MCP.
        #[rmcp::tool]
        pub async fn #fn_name(
            client: &rmcp::service::Peer<rmcp::service::RoleClient>,
        ) -> Result<#name, elicitation::ElicitError> {
            use elicitation::{Elicitation, ElicitClient};
            #name::elicit(&ElicitClient::new(client)).await
        }
    }
}
```

### Phase 3: Style-Aware Tools (Optional Enhancement)

Generate tools that accept optional style parameter:

```rust
#[rmcp::tool]
pub async fn elicit_config(
    client: &rmcp::service::Peer<rmcp::service::RoleClient>,
    style: Option<ConfigStyle>,
) -> Result<Config, elicitation::ElicitError> {
    let elicit_client = ElicitClient::new(client);
    
    match style {
        Some(s) => Config::with_style(s).elicit(client).await,
        None => Config::elicit(&elicit_client).await,
    }
}
```

## Usage Pattern

### In Library Crates

```rust
// botticelli_narrative/src/toml_parser/act.rs
use elicitation::Elicit;

#[derive(Debug, Clone, Elicit)]  // Automatically generates tool
pub struct TomlAct {
    pub model: Option<String>,
    pub input: Vec<TomlActInput>,
}
```

### In MCP Server

The generated tool functions can be:

**Option A: Manual registration** (current pattern):
```rust
// botticelli_mcp/src/rmcp_server/tools/narrative.rs
use botticelli_narrative::{elicit_toml_act, elicit_toml_act_input};

impl BotticelliServer {
    // Wrapper that calls generated function
    pub async fn create_act(
        &self,
        client: &Peer<RoleClient>,
    ) -> Result<Json<TomlAct>, ErrorData> {
        let act = elicit_toml_act(client).await
            .map_err(|e| to_mcp_error(e))?;
        Ok(Json(act))
    }
}
```

**Option B: Auto-registration** (future enhancement):
Use a tool scanning macro that finds all functions with `#[rmcp::tool]`:

```rust
#[tool_router(scan = [
    botticelli_narrative::*,
    botticelli_core::*,
])]
impl BotticelliServer {}
```

## Benefits

1. **Zero Boilerplate**: One derive attribute generates both client-side elicitation and MCP tool
2. **Type Safety**: Tool signatures match Elicitation trait automatically
3. **Consistency**: All elicit tools follow same pattern
4. **Discoverability**: Types with Elicit automatically become MCP-callable
5. **Maintainability**: Changes to Elicitation trait propagate to tools automatically

## Migration Path

### Phase 1: Implement in Elicitation Crate
Update derive macro to generate tool functions automatically.

### Phase 2: Immediate Benefit in Botticelli
All 150+ types with `#[derive(Elicit)]` instantly get tool functions:
- No code changes needed in botticelli
- Tools become available for registration
- Manual wrappers can be removed

### Phase 3: Register with tool_router
Use generated tools in botticelli_mcp server via tool scanning or manual registration.

## Design Decisions

1. **Tool visibility**: `pub` - makes them discoverable across crates

2. **Tool naming**: `elicit_{type}` in snake_case
   - `Config` → `elicit_config`
   - `TomlAct` → `elicit_toml_act`
   - Optional: Allow override with `#[prompt(tool_name = "custom")]`

3. **Module placement**: Same module as the type (inline with derive expansion)

4. **Error handling**: Return `ElicitError` directly (caller converts if needed)

5. **Registration**: Generate `#[rmcp::tool]` attribute, let tool_router discover them

## Next Steps

1. Create issue in elicitation repo proposing this feature
2. Implement basic tool generation (Phase 1-3)
3. Test with botticelli types
4. Add style-aware variant (Phase 4)
5. Document usage patterns
6. Consider auto-registration for v0.5.0

## Compatibility

- **Breaking change**: v0.5.0 - all `#[derive(Elicit)]` now generates tools
- **Migration**: None needed - tools are additive, don't break existing code
- **Feature flag**: Optional - could gate on `features = ["tools"]` if users want to skip tool generation
