//! Tests for intent parser.

use botticelli_chat::{parse_intent, Command};

#[test]
fn test_parse_exit() {
    assert!(parse_intent("exit").unwrap().is_exit());
    assert!(parse_intent("quit").unwrap().is_exit());
    assert!(parse_intent("bye").unwrap().is_exit());
    assert!(parse_intent("q").unwrap().is_exit());
}

#[test]
fn test_parse_help() {
    assert!(parse_intent("help").unwrap().is_help());
    assert!(parse_intent("h").unwrap().is_help());
    assert!(parse_intent("?").unwrap().is_help());
}

#[test]
fn test_parse_create_narrative() {
    let cmd = parse_intent("create narrative about a robot").unwrap();
    assert!(cmd.is_narrative());

    match cmd {
        Command::Narrative(narrative_cmd) => {
            assert!(narrative_cmd.is_mutating());
        }
        _ => panic!("Expected narrative command"),
    }
}

#[test]
fn test_parse_load_narrative() {
    let cmd = parse_intent("load narrative from test.toml").unwrap();
    assert!(cmd.is_narrative());
}

#[test]
fn test_parse_save_narrative() {
    let cmd = parse_intent("save narrative to output.toml").unwrap();
    assert!(cmd.is_narrative());
}

#[test]
fn test_parse_update_model() {
    let cmd = parse_intent("update narrative model to gpt-4").unwrap();
    assert!(cmd.is_narrative());
}

#[test]
fn test_parse_show_narrative() {
    let cmd = parse_intent("show narrative").unwrap();
    assert!(cmd.is_narrative());

    match cmd {
        Command::Narrative(narrative_cmd) => {
            assert!(narrative_cmd.is_readonly());
        }
        _ => panic!("Expected narrative command"),
    }
}

#[test]
fn test_parse_validate_narrative() {
    let cmd = parse_intent("validate narrative").unwrap();
    assert!(cmd.is_narrative());
}

// TODO: Improve parser for bot creation commands
// #[test]
// fn test_parse_create_bot() {
//     let cmd = parse_intent("create bot named test-bot").unwrap();
//     assert!(cmd.is_bot());
// }

#[test]
fn test_parse_list_bots() {
    let cmd = parse_intent("list bot").unwrap();
    assert!(cmd.is_bot());
}

// TODO: Improve parser for social scheduling commands
// #[test]
// fn test_parse_schedule_post() {
//     let cmd = parse_intent("schedule using mybot on twitter at 2024-01-01T00:00:00Z").unwrap();
//     assert!(cmd.is_social());
// }

#[test]
fn test_parse_unknown_command() {
    let result = parse_intent("foobar unknown command");
    assert!(result.is_err());
}

#[test]
fn test_parse_case_insensitive() {
    assert!(parse_intent("EXIT").unwrap().is_exit());
    assert!(parse_intent("HELP").unwrap().is_help());
    assert!(parse_intent("CREATE NARRATIVE").unwrap().is_narrative());
}
