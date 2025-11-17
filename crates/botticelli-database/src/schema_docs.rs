//! Schema documentation generation for content generation prompts.
//!
//! This module automatically generates LLM-friendly schema documentation
//! from database table structures, eliminating boilerplate in narrative files.

use crate::{ColumnInfo, TableSchema};
use crate::{DatabaseResult, reflect_table_schema};
use diesel::pg::PgConnection;

/// Type hints and documentation for common Discord field patterns
struct FieldDocumentation {
    pattern: &'static str,
    description: &'static str,
}

const DISCORD_FIELD_DOCS: &[FieldDocumentation] = &[
    FieldDocumentation {
        pattern: "id",
        description: "Unique 18-digit Discord snowflake ID",
    },
    FieldDocumentation {
        pattern: "owner_id",
        description: "Discord user ID of the owner (18-digit snowflake)",
    },
    FieldDocumentation {
        pattern: "guild_id",
        description: "Discord server/guild ID (18-digit snowflake)",
    },
    FieldDocumentation {
        pattern: "channel_id",
        description: "Discord channel ID (18-digit snowflake)",
    },
    FieldDocumentation {
        pattern: "user_id",
        description: "Discord user ID (18-digit snowflake)",
    },
    FieldDocumentation {
        pattern: "role_id",
        description: "Discord role ID (18-digit snowflake)",
    },
    FieldDocumentation {
        pattern: "name",
        description: "Name/title",
    },
    FieldDocumentation {
        pattern: "description",
        description: "Text description",
    },
    FieldDocumentation {
        pattern: "icon",
        description: "Icon hash (32-char hex string or null)",
    },
    FieldDocumentation {
        pattern: "member_count",
        description: "Number of members (integer)",
    },
    FieldDocumentation {
        pattern: "verification_level",
        description: "Security/verification level (0-4)",
    },
    FieldDocumentation {
        pattern: "premium_tier",
        description: "Server boost level (0-3)",
    },
    FieldDocumentation {
        pattern: "features",
        description: "Array of feature flags",
    },
];

/// Generate human-readable documentation for a database column
fn document_column(column: &ColumnInfo) -> String {
    let base_type = format_data_type(&column.data_type, column.character_maximum_length);

    let description = DISCORD_FIELD_DOCS
        .iter()
        .find(|doc| column.name.ends_with(doc.pattern))
        .map(|doc| doc.description)
        .unwrap_or("");

    let nullable = if column.is_nullable == "YES" {
        " (optional)"
    } else {
        ""
    };

    if description.is_empty() {
        format!("- {}: {}{}", column.name, base_type, nullable)
    } else {
        format!(
            "- {}: {} - {}{}",
            column.name, base_type, description, nullable
        )
    }
}

/// Format a PostgreSQL data type into human-readable form
fn format_data_type(pg_type: &str, max_length: Option<i32>) -> String {
    match pg_type {
        "bigint" => "64-bit integer".to_string(),
        "integer" => "integer".to_string(),
        "smallint" => "small integer".to_string(),
        "character varying" => {
            if let Some(len) = max_length {
                format!("text (max {} chars)", len)
            } else {
                "text".to_string()
            }
        }
        "text" => "text".to_string(),
        "boolean" => "boolean".to_string(),
        "timestamp without time zone" => "timestamp".to_string(),
        "ARRAY" => "array".to_string(),
        _ => pg_type.to_string(),
    }
}

/// Generate LLM-friendly schema documentation from a table structure
pub fn generate_schema_prompt(schema: &TableSchema) -> String {
    let mut prompt = String::new();

    prompt.push_str("Create a JSON object with the following schema:\n\n");

    let required_fields: Vec<_> = schema
        .columns
        .iter()
        .filter(|c| c.is_nullable == "NO")
        .collect();

    let optional_fields: Vec<_> = schema
        .columns
        .iter()
        .filter(|c| c.is_nullable == "YES")
        .collect();

    if !required_fields.is_empty() {
        prompt.push_str("**Required Fields:**\n");
        for column in required_fields {
            prompt.push_str(&document_column(column));
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    if !optional_fields.is_empty() {
        prompt.push_str("**Optional Fields:**\n");
        for column in optional_fields {
            prompt.push_str(&document_column(column));
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt
}

/// Universal JSON formatting requirements for LLM responses
pub const JSON_FORMAT_REQUIREMENTS: &str = r#"**CRITICAL OUTPUT REQUIREMENTS:**
- Output ONLY valid JSON with no additional text, explanations, or markdown
- Do not use markdown code blocks (no ```json)
- Start your response with { and end with }
- Use realistic, production-ready values
- Ensure all required fields are present
- Use appropriate data types (numbers as numbers, not strings)
"#;

/// Platform-specific context for Discord content generation
pub const DISCORD_PLATFORM_CONTEXT: &str =
    "You are generating data for Discord (a chat and community platform).";

/// Assemble a complete prompt from template schema and user content focus
pub fn assemble_prompt(
    conn: &mut PgConnection,
    template: &str,
    user_content_focus: &str,
) -> DatabaseResult<String> {
    let schema = reflect_table_schema(conn, template)?;

    let schema_docs = generate_schema_prompt(&schema);

    let is_discord_template = template.starts_with("discord_");
    let platform_context = if is_discord_template {
        DISCORD_PLATFORM_CONTEXT
    } else {
        ""
    };

    let mut prompt = String::new();

    if !platform_context.is_empty() {
        prompt.push_str(platform_context);
        prompt.push_str("\n\n");
    }

    prompt.push_str(&schema_docs);
    prompt.push_str(user_content_focus);
    prompt.push_str("\n\n");
    prompt.push_str(JSON_FORMAT_REQUIREMENTS);

    Ok(prompt)
}

/// Detect if a prompt is user-written content focus or explicit full prompt
///
/// Heuristics:
/// - Prompts containing schema keywords are explicit (checked first)
/// - Short prompts without keywords are likely content focus
pub fn is_content_focus(prompt: &str) -> bool {
    let trimmed = prompt.trim();
    let lowercase = trimmed.to_lowercase();

    // Check for schema documentation keywords first
    let schema_keywords = [
        "required fields",
        "optional fields",
        "json object",
        "critical output",
        "schema",
        "data type",
    ];

    for keyword in &schema_keywords {
        if lowercase.contains(keyword) {
            return false; // Explicit schema documentation present
        }
    }

    // No keywords found - this is content focus
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_content_focus_short() {
        let short = "Create a welcoming creative community for artists and musicians.";
        assert!(is_content_focus(short));
    }

    #[test]
    fn test_is_content_focus_long_with_keywords() {
        let explicit = r#"
        Create a JSON object with the following schema:
        
        **Required Fields:**
        - id: bigint
        - name: varchar(100)
        "#;
        assert!(!is_content_focus(explicit));
    }

    #[test]
    fn test_is_content_focus_critical_keyword() {
        let explicit = "Some text... **CRITICAL OUTPUT REQUIREMENTS:** ...";
        assert!(!is_content_focus(explicit));
    }

    #[test]
    fn test_format_data_type_varchar() {
        assert_eq!(
            format_data_type("character varying", Some(100)),
            "text (max 100 chars)"
        );
    }

    #[test]
    fn test_format_data_type_bigint() {
        assert_eq!(format_data_type("bigint", None), "64-bit integer");
    }
}
