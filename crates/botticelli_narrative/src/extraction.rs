//! Utilities for extracting structured data from LLM responses.
//!
//! LLM responses often contain JSON or TOML wrapped in markdown code blocks
//! or mixed with explanatory text. This module provides robust extraction
//! utilities that handle common LLM response patterns.

use botticelli_error::BotticelliResult;

/// Extract JSON from a response that may contain markdown or extra text.
///
/// This function tries multiple extraction strategies:
/// 1. Markdown code blocks: ```json ... ```
/// 2. Balanced braces: { ... }
/// 3. Balanced brackets: [ ... ]
///
/// # Errors
///
/// Returns an error if no valid JSON is found in the response.
///
/// # Examples
///
/// ```
/// use botticelli_narrative::extract_json;
///
/// let response = "Here's the data you requested:\n\
///     \n\
///     ```json\n\
///     {\"id\": 123, \"name\": \"Test\"}\n\
///     ```\n";
///
/// let json = extract_json(response).unwrap();
/// assert!(json.contains("123"));
/// ```
pub fn extract_json(response: &str) -> BotticelliResult<String> {
    // Strategy 1: Extract from markdown code blocks
    if let Some(json) = extract_from_code_block(response, "json") {
        return Ok(json);
    }

    // Strategy 2: Try arrays first (prefer complete structures)
    // Find which appears first in the response
    let bracket_pos = response.find('[');
    let brace_pos = response.find('{');

    match (bracket_pos, brace_pos) {
        (Some(b_pos), Some(c_pos)) if b_pos < c_pos => {
            // Array appears first, try extracting it
            if let Some(json) = extract_balanced(response, '[', ']') {
                return Ok(json);
            }
            // Fall back to object
            if let Some(json) = extract_balanced(response, '{', '}') {
                return Ok(json);
            }
        }
        (Some(_), None) => {
            // Only array
            if let Some(json) = extract_balanced(response, '[', ']') {
                return Ok(json);
            }
        }
        _ => {
            // Object appears first or only object exists
            if let Some(json) = extract_balanced(response, '{', '}') {
                return Ok(json);
            }
            // Fall back to array
            if let Some(json) = extract_balanced(response, '[', ']') {
                return Ok(json);
            }
        }
    }

    tracing::error!(
        response_length = response.len(),
        "No JSON found in LLM response"
    );

    Err(botticelli_error::BackendError::new(format!(
        "No JSON found in response (length: {}). Hint: Ensure your prompt explicitly requests JSON output and includes 'Output ONLY valid JSON'.",
        response.len()
    ))
    .into())
}

/// Extract TOML from a response that may contain markdown or extra text.
///
/// This function tries multiple extraction strategies:
/// 1. Markdown code blocks: ```toml ... ```
/// 2. TOML section headers: [section]
///
/// # Errors
///
/// Returns an error if no valid TOML is found in the response.
///
/// # Examples
///
/// ```
/// use botticelli_narrative::extract_toml;
///
/// let response = "Here's your configuration:\n\
///     \n\
///     ```toml\n\
///     [server]\n\
///     name = \"Test Server\"\n\
///     ```\n";
///
/// let toml = extract_toml(response).unwrap();
/// assert!(toml.contains("[server]"));
/// ```
pub fn extract_toml(response: &str) -> BotticelliResult<String> {
    // Strategy 1: Extract from markdown code blocks
    if let Some(toml_str) = extract_from_code_block(response, "toml") {
        return Ok(toml_str);
    }

    // Strategy 2: Look for TOML section headers [...]
    if response.contains('[') && (response.contains(" = ") || response.contains('=')) {
        // Try to find first [ and use everything from there
        if let Some(start) = response.find('[') {
            return Ok(response[start..].trim().to_string());
        }
    }

    Err(botticelli_error::BackendError::new(format!(
        "No TOML found in response (length: {})",
        response.len()
    ))
    .into())
}

/// Extract content from markdown code blocks.
///
/// Looks for patterns like:
/// - ```language\n...\n```
/// - ``` ... ``` (no language specified)
fn extract_from_code_block(response: &str, language: &str) -> Option<String> {
    // Pattern: ```language\n...\n```
    let pattern = format!("```{}", language);

    if let Some(start) = response.find(&pattern) {
        let content_start = start + pattern.len();
        if let Some(end) = response[content_start..].find("```") {
            let content = &response[content_start..content_start + end];
            return Some(content.trim().to_string());
        }
        // No closing fence found - likely truncated response
        // Return content from opening fence to end
        return Some(response[content_start..].trim().to_string());
    }

    // Try without language specifier
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        // Skip to next newline (in case there's a language specifier)
        let skip_to = response[content_start..]
            .find('\n')
            .map(|n| content_start + n + 1)
            .unwrap_or(content_start);

        if let Some(end) = response[skip_to..].find("```") {
            let content = &response[skip_to..skip_to + end];
            return Some(content.trim().to_string());
        }
        // No closing fence found - likely truncated response
        // Return content from opening fence to end
        return Some(response[skip_to..].trim().to_string());
    }

    None
}

/// Extract content between balanced delimiters.
///
/// Finds the first occurrence of `open` and extracts content up to
/// the matching `close`, handling nesting correctly.
fn extract_balanced(response: &str, open: char, close: char) -> Option<String> {
    let start = response.find(open)?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in response[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => escape_next = true,
            '"' => in_string = !in_string,
            c if c == open && !in_string => depth += 1,
            c if c == close && !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(response[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

/// Parse and validate JSON, returning a specific type.
///
/// # Errors
///
/// Returns an error if the JSON string cannot be parsed into type `T`.
///
/// # Examples
///
/// ```
/// use botticelli_narrative::parse_json;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let json = r#"{"id": 123, "name": "Alice"}"#;
/// let user: User = parse_json(json).unwrap();
/// assert_eq!(user.id, 123);
/// ```
pub fn parse_json<T>(json_str: &str) -> BotticelliResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(json_str).map_err(|e| {
        let preview = json_str
            .chars()
            .take(100)
            .collect::<String>();

        tracing::error!(
            error = %e,
            json_preview = %preview,
            "JSON parsing failed"
        );

        botticelli_error::BackendError::new(format!(
            "Failed to parse JSON: {} (JSON: {}...). Hint: Ensure the LLM outputs valid JSON without syntax errors.",
            e,
            preview
        ))
        .into()
    })
}

/// Parse and validate TOML, returning a specific type.
///
/// # Errors
///
/// Returns an error if the TOML string cannot be parsed into type `T`.
///
/// # Examples
///
/// ```
/// use botticelli_narrative::parse_toml;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Config {
///     server_name: String,
/// }
///
/// let toml = r#"server_name = "Test Server""#;
/// let config: Config = parse_toml(toml).unwrap();
/// assert_eq!(config.server_name, "Test Server");
/// ```
pub fn parse_toml<T>(toml_str: &str) -> BotticelliResult<T>
where
    T: serde::de::DeserializeOwned,
{
    toml::from_str(toml_str).map_err(|e| {
        let preview = toml_str
            .char_indices()
            .take(100)
            .last()
            .map(|(idx, _)| &toml_str[..=idx])
            .unwrap_or(toml_str);
        botticelli_error::BackendError::new(format!(
            "Failed to parse TOML: {} (TOML: {}...)",
            e, preview
        ))
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_block() {
        let response = r#"
Here's the JSON you requested:

```json
{
  "id": 123,
  "name": "Test"
}
```

Hope this helps!
"#;
        let json = extract_json(response).unwrap();
        assert!(json.contains("\"id\": 123"));
    }

    #[test]
    fn test_extract_json_balanced_braces() {
        let response = r#"
Sure! Here it is: {"id": 456, "nested": {"value": "test"}}
"#;
        let json = extract_json(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_array() {
        let response = r#"
Here are the items:
[
  {"id": 1},
  {"id": 2}
]
"#;
        let json = extract_json(response).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[test]
    fn test_no_json_found() {
        let response = "This is just plain text with no JSON";
        assert!(extract_json(response).is_err());
    }

    #[test]
    fn test_extract_json_with_string_escapes() {
        let response = r#"{"text": "She said \"hello\""}"#;
        let json = extract_json(response).unwrap();
        assert!(json.contains("She said"));
    }

    #[test]
    fn test_extract_toml_from_code_block() {
        let response = r#"
```toml
[server]
name = "Test"
```
"#;
        let toml = extract_toml(response).unwrap();
        assert!(toml.contains("[server]"));
    }

    #[test]
    fn test_extract_toml_without_code_block() {
        let response = r#"
Here's your config:

[database]
url = "postgres://localhost"
"#;
        let toml = extract_toml(response).unwrap();
        assert!(toml.contains("[database]"));
    }

    #[test]
    fn test_parse_json_into_struct() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug)]
        struct TestData {
            id: i32,
            name: String,
        }

        let json = r#"{"id": 42, "name": "test"}"#;
        let data: TestData = parse_json(json).unwrap();
        assert_eq!(data.id, 42);
        assert_eq!(data.name, "test");
    }

    #[test]
    fn test_parse_toml_into_struct() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug)]
        struct Config {
            port: u16,
        }

        let toml = r#"port = 8080"#;
        let config: Config = parse_toml(toml).unwrap();
        assert_eq!(config.port, 8080);
    }
}
