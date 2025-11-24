# Multi-Narrative TOML Design

## Problem Statement

Currently, one TOML file = one narrative. This leads to:
- File proliferation for similar workflows with different focuses
- Duplication of shared resources (media, bots, acts)
- No way to batch-run related narratives easily

## Proposed Solution

Support multiple narratives in a single TOML file using `[narratives.name]` sections.

## TOML Structure

```toml
# Shared resources (available to all narratives)
[media.context]
file = "./BOTTICELLI_CONTEXT.md"

[bots.stats]
platform = "discord"
command = "server.get_stats"

# Shared act definitions
[acts.generate]
model = "gemini-2.5-flash-lite"
[[acts.generate.input]]
type = "media"
reference = "media.context"
# ... more inputs

[acts.critique]
# ... act definition

[acts.refine]
# ... act definition

# === Narrative 1: Feature Posts ===
[narratives.feature]
name = "gen_posts_feature"
description = "Generate feature-focused Discord posts"
template = "potential_posts"

[narratives.feature.carousel]
iterations = 10

[narratives.feature.toc]
order = ["generate", "critique", "refine"]

# === Narrative 2: Usecase Posts ===
[narratives.usecase]
name = "gen_posts_usecase"
description = "Generate usecase-focused Discord posts"
template = "potential_posts"

[narratives.usecase.carousel]
iterations = 10

[narratives.usecase.toc]
order = ["generate", "critique", "refine"]

# ... 3 more narratives
```

## Backwards Compatibility

Files with single `[narrative]` still work:

```toml
[narrative]
name = "single_narrative"
# ... works as before
```

## Loading Behavior

### CLI: `botticelli run --narrative file.toml`
- If file has `[narrative]`: Load single narrative (current behavior)
- If file has `[narratives.*]`: Error - must specify which one

### CLI: `botticelli run --narrative file.toml --narrative-name feature`
- Load specific narrative from multi-narrative file
- Falls back to single `[narrative]` if name not found in `[narratives.*]`

### Actor: `narrative_file = "file.toml", narrative_name = "feature"`
- Explicitly load named narrative from file

### Just: `just narrate file.feature`
- Parse as `file.toml` + narrative name `feature`
- Searches for `[narratives.feature]` or falls back to `[narrative]`

## Implementation Plan

### 1. Update TomlNarrativeFile Structure

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TomlNarrativeFile {
    /// Single narrative (backwards compatible)
    #[serde(default)]
    pub narrative: Option<TomlNarrative>,
    
    /// Multiple narratives (new feature)
    #[serde(default)]
    pub narratives: HashMap<String, TomlNarrativeDefinition>,
    
    /// TOC for single narrative
    #[serde(default)]
    pub toc: Option<TomlToc>,
    
    /// Shared act definitions
    #[serde(default)]
    pub acts: HashMap<String, TomlAct>,
    
    /// Shared resources
    #[serde(default)]
    pub bots: HashMap<String, TomlBotDefinition>,
    #[serde(default)]
    pub tables: HashMap<String, TomlTableDefinition>,
    #[serde(default)]
    pub media: HashMap<String, TomlMediaDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlNarrativeDefinition {
    pub name: String,
    pub description: String,
    pub template: Option<String>,
    #[serde(default)]
    pub skip_content_generation: bool,
    #[serde(default)]
    pub carousel: Option<CarouselConfig>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub toc: TomlToc,  // Each narrative has its own TOC
    
    /// Optional narrative-specific acts (override shared acts)
    #[serde(default)]
    pub acts: HashMap<String, TomlAct>,
}
```

### 2. Update Loader Logic

```rust
impl TomlNarrativeFile {
    /// Load a specific narrative by name
    pub fn load_narrative(&self, name: Option<&str>) -> Result<ResolvedNarrative, Error> {
        match (name, &self.narrative, &self.narratives) {
            // Explicit name provided
            (Some(n), _, narratives) if !narratives.is_empty() => {
                let def = narratives.get(n)
                    .ok_or_else(|| Error::NarrativeNotFound(n.to_string()))?;
                self.resolve_narrative(def)
            },
            
            // No name, but single [narrative] exists (backwards compat)
            (None, Some(single), _) => {
                self.resolve_single_narrative(single)
            },
            
            // No name, multiple [narratives.*] exist
            (None, _, narratives) if !narratives.is_empty() => {
                Err(Error::AmbiguousNarrative(narratives.keys().cloned().collect()))
            },
            
            _ => Err(Error::NoNarrativeFound),
        }
    }
    
    fn resolve_narrative(&self, def: &TomlNarrativeDefinition) -> Result<ResolvedNarrative, Error> {
        // Merge shared acts with narrative-specific acts
        let mut acts = self.acts.clone();
        acts.extend(def.acts.clone());  // Narrative acts override shared
        
        // Build resolved narrative
        Ok(ResolvedNarrative {
            metadata: def.to_metadata(),
            toc: def.toc.clone(),
            acts,
            bots: self.bots.clone(),
            tables: self.tables.clone(),
            media: self.media.clone(),
        })
    }
}
```

### 3. Update CLI Arguments

```rust
#[derive(Parser)]
struct RunArgs {
    #[arg(short = 'n', long)]
    narrative: PathBuf,
    
    /// Specific narrative name (for multi-narrative files)
    #[arg(long)]
    narrative_name: Option<String>,
    
    // ... other args
}
```

### 4. Update Just Recipe

```makefile
narrate PATTERN:
    #!/usr/bin/env bash
    # Support file.narrative_name syntax
    if [[ "{{PATTERN}}" == *.* ]]; then
        FILE="${{PATTERN%.*}}"
        NAME="${{PATTERN##*.}}"
        cargo run --release --features gemini -- run \
            --narrative "$FILE.toml" \
            --narrative-name "$NAME" \
            --save --verbose
    else
        # Existing behavior
        cargo run --release --features gemini -- run \
            --narrative "$(find . -name '{{PATTERN}}*.toml')" \
            --save --verbose
    fi
```

## Testing Strategy

1. **Backwards compatibility**: Ensure existing single-narrative files work unchanged
2. **Multi-narrative loading**: Test loading specific narratives by name
3. **Resource sharing**: Verify shared acts/media accessible to all narratives
4. **Override behavior**: Test narrative-specific acts override shared acts
5. **Error handling**: Test ambiguous loads, missing narratives

## Migration Path

Existing files: No changes required - single `[narrative]` continues to work

New multi-narrative files:
1. Create shared resources at root
2. Define narratives under `[narratives.name]`
3. Each narrative has own `toc`
4. Reference with `--narrative-name` or `file.name` syntax

## Benefits

✅ Avoids file proliferation
✅ Shares resources efficiently
✅ Backwards compatible
✅ Clear separation of concerns
✅ Easy to run individual narratives or all of them
✅ Actor can reference multiple narratives from one file

## Example: Generation Carousel

Instead of 5 files, one file with 5 narratives:

```toml
[media.context]
file = "./BOTTICELLI_CONTEXT.md"

[acts.generate]
# ... shared definition

[narratives.feature]
name = "gen_posts_feature"
template = "potential_posts"
[narratives.feature.carousel]
iterations = 10
[narratives.feature.toc]
order = ["generate", "critique", "refine"]

[narratives.usecase]
# ... similar structure

# ... 3 more
```

Run individually: `just narrate generation_carousel.feature`
Run all: Actor config with 5 narrative_names
