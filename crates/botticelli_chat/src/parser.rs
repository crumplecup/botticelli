//! Intent parser for converting user input to commands.

use crate::{BotCommand, ChatError, ChatResult, Command, NarrativeCommand, SocialCommand};

/// Parse user text input into a Command.
///
/// This is a basic implementation that can be extended with more
/// sophisticated NLP or LLM-based parsing later.
///
/// # Examples
///
/// ```
/// use botticelli_chat::parse_intent;
///
/// let cmd = parse_intent("create narrative about a robot").unwrap();
/// assert!(cmd.is_narrative());
/// ```
pub fn parse_intent(input: &str) -> ChatResult<Command> {
    let input = input.trim().to_lowercase();

    // Handle exit commands
    if matches!(input.as_str(), "exit" | "quit" | "bye" | "q") {
        return Ok(Command::Exit);
    }

    // Handle help
    if matches!(input.as_str(), "help" | "h" | "?") {
        return Ok(Command::Help);
    }

    // Parse narrative commands
    if input.contains("narrative") || input.contains("story") {
        return parse_narrative_command(&input);
    }

    // Parse bot commands
    if input.contains("bot") {
        return parse_bot_command(&input);
    }

    // Parse social commands
    if input.contains("schedule") || input.contains("post") {
        return parse_social_command(&input);
    }

    Err(ChatError::parse_error(format!(
        "Could not parse command: {}",
        input
    )))
}

fn parse_narrative_command(input: &str) -> ChatResult<Command> {
    // Create narrative
    if input.contains("create") || input.contains("new") || input.contains("generate") {
        // Extract prompt after keywords
        let prompt = extract_after_keywords(
            input,
            &["create", "new", "generate", "narrative", "story", "about"],
        );
        return Ok(Command::Narrative(NarrativeCommand::Create {
            prompt: prompt.unwrap_or_else(|| input.to_string()),
        }));
    }

    // Load narrative
    if input.contains("load") || input.contains("open") {
        let path = extract_after_keywords(input, &["load", "open", "narrative", "from"])
            .ok_or_else(|| ChatError::missing_argument("path"))?;
        return Ok(Command::Narrative(NarrativeCommand::Load { path }));
    }

    // Save narrative
    if input.contains("save") {
        let path = extract_after_keywords(input, &["save", "narrative", "to"])
            .ok_or_else(|| ChatError::missing_argument("path"))?;
        return Ok(Command::Narrative(NarrativeCommand::Save { path }));
    }

    // Update model
    if input.contains("model") && (input.contains("update") || input.contains("change")) {
        let model = extract_after_keywords(input, &["model", "to", "use"])
            .ok_or_else(|| ChatError::missing_argument("model"))?;
        return Ok(Command::Narrative(NarrativeCommand::UpdateModel { model }));
    }

    // Update prompt
    if input.contains("prompt") && (input.contains("update") || input.contains("change")) {
        let prompt = extract_after_keywords(input, &["prompt", "to"])
            .ok_or_else(|| ChatError::missing_argument("prompt"))?;
        return Ok(Command::Narrative(NarrativeCommand::UpdatePrompt {
            prompt,
        }));
    }

    // Update temperature
    if input.contains("temperature") {
        let temp_str = extract_after_keywords(input, &["temperature", "to"])
            .ok_or_else(|| ChatError::missing_argument("temperature"))?;
        let temperature = temp_str
            .parse::<f32>()
            .map_err(|_| ChatError::parse_error("Invalid temperature value"))?;
        return Ok(Command::Narrative(NarrativeCommand::UpdateTemperature {
            temperature,
        }));
    }

    // Update max tokens
    if input.contains("tokens") || input.contains("length") {
        let tokens_str = extract_after_keywords(input, &["tokens", "length", "to"])
            .ok_or_else(|| ChatError::missing_argument("max_tokens"))?;
        let max_tokens = tokens_str
            .parse::<u32>()
            .map_err(|_| ChatError::parse_error("Invalid max_tokens value"))?;
        return Ok(Command::Narrative(NarrativeCommand::UpdateMaxTokens {
            max_tokens,
        }));
    }

    // Validate
    if input.contains("validate") || input.contains("check") {
        return Ok(Command::Narrative(NarrativeCommand::Validate));
    }

    // Show
    if input.contains("show") || input.contains("display") {
        return Ok(Command::Narrative(NarrativeCommand::Show));
    }

    // List
    if input.contains("list") {
        return Ok(Command::Narrative(NarrativeCommand::List));
    }

    Err(ChatError::parse_error("Unknown narrative command"))
}

fn parse_bot_command(input: &str) -> ChatResult<Command> {
    // Create bot
    if input.contains("create") || input.contains("new") {
        let name = extract_after_keywords(input, &["create", "new", "bot", "named"])
            .ok_or_else(|| ChatError::missing_argument("name"))?;
        return Ok(Command::Bot(BotCommand::Create { name }));
    }

    // Assign narrative
    if input.contains("assign") {
        let bot_id = extract_after_keywords(input, &["bot", "assign", "to"])
            .ok_or_else(|| ChatError::missing_argument("bot_id"))?;
        let narrative_path = extract_after_keywords(input, &["narrative", "from"])
            .ok_or_else(|| ChatError::missing_argument("narrative_path"))?;
        return Ok(Command::Bot(BotCommand::AssignNarrative {
            bot_id,
            narrative_path,
        }));
    }

    // Show bot
    if input.contains("show") || input.contains("display") {
        let bot_id = extract_after_keywords(input, &["bot", "show", "display"])
            .ok_or_else(|| ChatError::missing_argument("bot_id"))?;
        return Ok(Command::Bot(BotCommand::Show { bot_id }));
    }

    // List bots
    if input.contains("list") {
        return Ok(Command::Bot(BotCommand::List));
    }

    Err(ChatError::parse_error("Unknown bot command"))
}

fn parse_social_command(input: &str) -> ChatResult<Command> {
    // Schedule post
    if input.contains("schedule") {
        let bot_id = extract_after_keywords(input, &["bot", "using"])
            .ok_or_else(|| ChatError::missing_argument("bot_id"))?;
        let platform = extract_after_keywords(input, &["platform", "on"])
            .ok_or_else(|| ChatError::missing_argument("platform"))?;
        let schedule_time = extract_after_keywords(input, &["at", "time"])
            .ok_or_else(|| ChatError::missing_argument("schedule_time"))?;
        return Ok(Command::Social(SocialCommand::Schedule {
            bot_id,
            platform,
            schedule_time,
        }));
    }

    // Show schedule
    if input.contains("show") {
        return Ok(Command::Social(SocialCommand::ShowSchedule));
    }

    // Cancel scheduled post
    if input.contains("cancel") {
        let schedule_id = extract_after_keywords(input, &["cancel", "schedule"])
            .ok_or_else(|| ChatError::missing_argument("schedule_id"))?;
        return Ok(Command::Social(SocialCommand::Cancel { schedule_id }));
    }

    Err(ChatError::parse_error("Unknown social command"))
}

/// Extract text after any of the given keywords.
fn extract_after_keywords(input: &str, keywords: &[&str]) -> Option<String> {
    for keyword in keywords {
        if let Some(pos) = input.find(keyword) {
            let after = &input[pos + keyword.len()..].trim();
            if !after.is_empty() {
                return Some(after.to_string());
            }
        }
    }
    None
}
