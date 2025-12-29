//! Tests for command types.

use botticelli_chat::{BotCommand, Command, NarrativeCommand, SocialCommand};

#[test]
fn test_command_is_exit() {
    assert!(Command::Exit.is_exit());
    assert!(!Command::Help.is_exit());
}

#[test]
fn test_command_is_help() {
    assert!(Command::Help.is_help());
    assert!(!Command::Exit.is_help());
}

#[test]
fn test_command_is_narrative() {
    let cmd = Command::Narrative(NarrativeCommand::Show);
    assert!(cmd.is_narrative());
    assert!(!cmd.is_bot());
    assert!(!cmd.is_social());
}

#[test]
fn test_command_is_bot() {
    let cmd = Command::Bot(BotCommand::List);
    assert!(cmd.is_bot());
    assert!(!cmd.is_narrative());
    assert!(!cmd.is_social());
}

#[test]
fn test_command_is_social() {
    let cmd = Command::Social(SocialCommand::ShowSchedule);
    assert!(cmd.is_social());
    assert!(!cmd.is_narrative());
    assert!(!cmd.is_bot());
}

#[test]
fn test_narrative_command_is_mutating() {
    assert!(
        NarrativeCommand::Create {
            prompt: "test".to_string()
        }
        .is_mutating()
    );
    assert!(
        NarrativeCommand::UpdateModel {
            model: "test".to_string()
        }
        .is_mutating()
    );
    assert!(!NarrativeCommand::Show.is_mutating());
    assert!(!NarrativeCommand::Validate.is_mutating());
}

#[test]
fn test_narrative_command_is_readonly() {
    assert!(NarrativeCommand::Show.is_readonly());
    assert!(NarrativeCommand::Validate.is_readonly());
    assert!(
        !NarrativeCommand::Create {
            prompt: "test".to_string()
        }
        .is_readonly()
    );
}
