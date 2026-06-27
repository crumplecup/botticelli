//! IR tests for [`NarrativeWizardScreen`] — no terminal required.

#![recursion_limit = "256"]

use botticelli_tui::screen::BotScreen;
use botticelli_tui::{BotScreenContext, BotTransition, NarrativeWizardScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctx() -> BotScreenContext {
    BotScreenContext::mock()
}

fn wizard() -> NarrativeWizardScreen {
    NarrativeWizardScreen::new(None, None)
}

/// Enter typing mode on a Leaf step (press '1') and type text without submitting.
fn type_text(s: &mut NarrativeWizardScreen, text: &str) {
    s.handle_key(key(KeyCode::Char('1')), &ctx()); // '1' enters typing mode for Leaf
    for c in text.chars() {
        s.handle_key(key(KeyCode::Char(c)), &ctx());
    }
}

/// Enter typing mode, type text, and submit (Enter).  Use on Leaf steps.
fn leaf_submit(s: &mut NarrativeWizardScreen, text: &str) {
    type_text(s, text);
    s.handle_key(key(KeyCode::Enter), &ctx());
}

/// Submit an Affirm step: '1' = Yes, '2' = No.
fn affirm_submit(s: &mut NarrativeWizardScreen, yes: bool) {
    let d = if yes { '1' } else { '2' };
    s.handle_key(key(KeyCode::Char(d)), &ctx());
}

/// Submit a Select step by pressing digit `n` (1-based).
fn select_digit(s: &mut NarrativeWizardScreen, n: u8) {
    let d = char::from(b'0' + n);
    s.handle_key(key(KeyCode::Char(d)), &ctx());
}

/// Minimal answer sequence that completes a one-act (Prompt) narrative.
///
/// Matches exactly what `NarrativeGenerator::elicit` expects from the
/// `BufferedCommunicator`:
/// 1. name           (Leaf)
/// 2. description    (Leaf)
/// 3. "true"         → Vec<acts>: add first act?
/// 4. act name       (Leaf)
/// 5. "Prompt"       → NarrativeActContent Select
/// 6. act prompt text (Leaf)
/// 7. "false"        → Vec<acts>: no more acts
/// 8. "false"        → model Option: no
fn minimal_answers() -> Vec<String> {
    vec![
        "test_name".into(),
        "A test narrative".into(),
        "true".into(),
        "research".into(),
        "Prompt".into(),
        "Research the topic thoroughly.".into(),
        "false".into(),
        "false".into(),
    ]
}

// ── Render ────────────────────────────────────────────────────────────────────

#[test]
fn renders_without_panic_on_first_step() {
    let s = wizard();
    let _node = s.to_tui_node();
}

#[test]
fn screen_name_is_narrative_wizard() {
    let s = wizard();
    assert_eq!(s.screen_name(), "Narrative Wizard");
}

#[test]
fn starts_at_step_zero() {
    let s = wizard();
    assert_eq!(s.current(), 0);
}

#[test]
fn starts_not_complete() {
    let s = wizard();
    assert!(!s.is_complete());
}

#[test]
fn has_a_current_prompt() {
    let s = wizard();
    assert!(
        s.current_prompt().is_some(),
        "first step should have a prompt"
    );
}

// ── Mode switching ────────────────────────────────────────────────────────────

#[test]
fn i_key_does_nothing_in_menu_mode() {
    let mut s = wizard();
    // 'i' is not bound in MENU mode
    s.handle_key(key(KeyCode::Char('i')), &ctx());
    assert_eq!(s.current(), 0, "i should not advance the step");
}

#[test]
fn esc_in_typing_returns_to_menu_without_navigating_away() {
    let mut s = wizard();
    s.handle_key(key(KeyCode::Char('1')), &ctx()); // '1' enters typing on Leaf
    let t = s.handle_key(key(KeyCode::Esc), &ctx()); // Esc returns to MENU
    assert!(
        matches!(t, BotTransition::Stay),
        "Esc in typing should return to MENU, not cancel the wizard"
    );
    assert_eq!(s.current(), 0, "step should not have changed");
}

#[test]
fn esc_in_menu_cancels_wizard() {
    let mut s = wizard();
    let t = s.handle_key(key(KeyCode::Esc), &ctx());
    assert!(matches!(t, BotTransition::GoToNarratives));
}

#[test]
fn key_one_enters_typing_mode_on_leaf_step() {
    let mut s = wizard();
    // '1' on a Leaf step enters typing (does not advance step)
    let t = s.handle_key(key(KeyCode::Char('1')), &ctx());
    assert!(matches!(t, BotTransition::Stay));
    // Typing and submitting now advances the step
    s.handle_key(key(KeyCode::Char('x')), &ctx());
    s.handle_key(key(KeyCode::Enter), &ctx());
    assert_eq!(
        s.current(),
        1,
        "should have advanced after typing and submitting"
    );
}

// ── User input (must be in typing mode) ──────────────────────────────────────

#[test]
fn chars_in_menu_mode_do_not_advance() {
    let mut s = wizard();
    // Unbound keys in MENU mode do nothing
    s.handle_key(key(KeyCode::Char('h')), &ctx());
    s.handle_key(key(KeyCode::Char('j')), &ctx());
    s.handle_key(key(KeyCode::Char('k')), &ctx());
    assert_eq!(s.current(), 0);
}

#[test]
fn typing_in_typing_mode_does_not_advance_step() {
    let mut s = wizard();
    type_text(&mut s, "hello");
    assert_eq!(s.current(), 0);
}

#[test]
fn backspace_in_typing_mode_removes_last_char() {
    let mut s = wizard();
    type_text(&mut s, "hi");
    s.handle_key(key(KeyCode::Backspace), &ctx()); // remove 'i' → "h"
    s.handle_key(key(KeyCode::Enter), &ctx()); // submit "h"
    assert_eq!(s.current(), 1);
}

#[test]
fn enter_with_empty_input_does_not_advance() {
    let mut s = wizard();
    s.handle_key(key(KeyCode::Char('1')), &ctx()); // enter typing
    s.handle_key(key(KeyCode::Enter), &ctx()); // submit empty — should stay
    assert_eq!(s.current(), 0);
}

#[test]
fn enter_with_non_empty_input_advances() {
    let mut s = wizard();
    leaf_submit(&mut s, "x");
    assert_eq!(s.current(), 1);
}

#[test]
fn answer_is_trimmed_before_storing() {
    let mut s = wizard();
    // " v" trims to "v" — non-empty, so advances
    leaf_submit(&mut s, " v");
    assert!(s.current() >= 1);
}

#[test]
fn n_in_typing_mode_adds_to_buffer_not_agent() {
    let mut s = wizard();
    type_text(&mut s, "n"); // 'n' while typing goes to buffer
    let t = s.handle_key(key(KeyCode::Enter), &ctx()); // submit "n"
    assert!(matches!(t, BotTransition::Stay));
    assert_eq!(
        s.current(),
        1,
        "'n' in typing should be buffered, not trigger agent"
    );
}

#[test]
fn a_in_typing_mode_adds_to_buffer_not_agent() {
    let mut s = wizard();
    type_text(&mut s, "a"); // 'a' while typing goes to buffer
    let t = s.handle_key(key(KeyCode::Enter), &ctx());
    assert!(matches!(t, BotTransition::Stay));
    assert_eq!(
        s.current(),
        1,
        "'a' in typing should be buffered, not trigger agent"
    );
}

// ── Agent delegation (MENU mode, option 2 = next, 3 = all) ──────────────────

#[test]
fn digit_two_on_leaf_emits_agent_fill_next() {
    let mut s = wizard();
    let t = s.handle_key(key(KeyCode::Char('2')), &ctx());
    assert!(
        matches!(t, BotTransition::AgentFillNarrativeNext { .. }),
        "'2' on Leaf in MENU should emit AgentFillNarrativeNext"
    );
}

#[test]
fn agent_fill_next_carries_prompt_text() {
    let mut s = wizard();
    let prompt_before = s.current_prompt().unwrap().to_string();
    let t = s.handle_key(key(KeyCode::Char('2')), &ctx());
    if let BotTransition::AgentFillNarrativeNext { prompt } = t {
        assert_eq!(prompt, prompt_before);
    } else {
        panic!("expected AgentFillNarrativeNext");
    }
}

#[test]
fn digit_three_on_leaf_emits_agent_fill_all() {
    let mut s = wizard();
    let t = s.handle_key(key(KeyCode::Char('3')), &ctx());
    assert!(
        matches!(t, BotTransition::AgentFillNarrativeAll { .. }),
        "'3' on Leaf in MENU should emit AgentFillNarrativeAll"
    );
}

#[test]
fn agent_fill_all_carries_schema_and_remaining_prompts() {
    let mut s = wizard();
    let t = s.handle_key(key(KeyCode::Char('3')), &ctx());
    if let BotTransition::AgentFillNarrativeAll {
        remaining_prompts,
        json_schema,
    } = t
    {
        assert!(
            !remaining_prompts.is_empty(),
            "remaining_prompts should not be empty"
        );
        assert!(!json_schema.is_empty(), "json_schema should not be empty");
        serde_json::from_str::<serde_json::Value>(&json_schema)
            .expect("json_schema must be valid JSON");
    } else {
        panic!("expected AgentFillNarrativeAll");
    }
}

// ── Prompts are human-readable (baked into types) ────────────────────────────

#[test]
fn prompts_are_not_bare_field_names() {
    let s = wizard();
    let prompt = s.current_prompt().expect("should have a prompt");
    assert!(
        prompt.len() > 10,
        "prompt should be a descriptive sentence, got: {prompt:?}"
    );
    assert!(
        prompt.contains(' '),
        "prompt should contain spaces (be a sentence), got: {prompt:?}"
    );
}

// ── Dynamic step injection ────────────────────────────────────────────────────

#[test]
fn affirm_yes_injects_item_steps() {
    let mut s = wizard();
    // Submit name + description (two Leaf steps)
    leaf_submit(&mut s, "my_narrative");
    leaf_submit(&mut s, "test description");
    let before = s.current();
    // Step 3 is the acts Vec Affirm — press '1' for Yes
    affirm_submit(&mut s, true);
    assert!(
        s.current() > before,
        "yes to Vec affirm should advance and inject act steps"
    );
    // Next prompt should be the act name prompt
    let p = s.current_prompt().unwrap();
    assert!(
        p.contains("Act name") || p.contains("act name"),
        "after yes-to-acts, next step should be act name, got: {p:?}"
    );
}

#[test]
fn select_step_branches_into_prompt_variant() {
    let mut s = wizard();
    leaf_submit(&mut s, "my_narrative");
    leaf_submit(&mut s, "description");
    affirm_submit(&mut s, true); // add act
    leaf_submit(&mut s, "research"); // act name
    // At the content Select — press '1' for Prompt
    select_digit(&mut s, 1);
    let p = s.current_prompt().unwrap();
    assert!(
        p.to_lowercase().contains("prompt") && p.to_lowercase().contains("llm"),
        "choosing Prompt should inject prompt-text step, got: {p:?}"
    );
}

#[test]
fn select_step_branches_into_narrativeref_variant() {
    let mut s = wizard();
    leaf_submit(&mut s, "my_narrative");
    leaf_submit(&mut s, "description");
    affirm_submit(&mut s, true); // add act
    leaf_submit(&mut s, "ref_act"); // act name
    // Press '2' for NarrativeRef
    select_digit(&mut s, 2);
    let p = s.current_prompt().unwrap();
    assert!(
        p.to_lowercase().contains("key") || p.to_lowercase().contains("narrative"),
        "choosing NarrativeRef should inject key step, got: {p:?}"
    );
}

#[test]
fn select_step_branches_into_carousel_variant() {
    let mut s = wizard();
    leaf_submit(&mut s, "my_narrative");
    leaf_submit(&mut s, "description");
    affirm_submit(&mut s, true); // add act
    leaf_submit(&mut s, "carousel_act"); // act name
    // Press '3' for Carousel
    select_digit(&mut s, 3);
    let p = s.current_prompt().unwrap();
    assert!(
        p.to_lowercase().contains("carousel") || p.to_lowercase().contains("iteration"),
        "choosing Carousel should inject carousel steps, got: {p:?}"
    );
}

#[test]
fn vec_loop_repeats_after_yes() {
    let mut s = wizard();
    leaf_submit(&mut s, "my_narrative");
    leaf_submit(&mut s, "description");
    // First act
    affirm_submit(&mut s, true); // add first act
    leaf_submit(&mut s, "act1"); // act name
    select_digit(&mut s, 1); // Prompt
    leaf_submit(&mut s, "Do research"); // prompt text
    // Should hit the Vec Affirm again ("Add an act?")
    let p = s.current_prompt().unwrap();
    assert!(
        p.to_lowercase().contains("act") || p.to_lowercase().contains("add"),
        "after first act, should ask add another, got: {p:?}"
    );
    // Say yes to add a second act
    affirm_submit(&mut s, true);
    let p2 = s.current_prompt().unwrap();
    assert!(
        p2.to_lowercase().contains("act name") || p2.to_lowercase().contains("identifier"),
        "after yes-again, should ask for second act name, got: {p2:?}"
    );
}

// ── Agent callback ────────────────────────────────────────────────────────────

#[test]
fn on_wizard_field_filled_advances_step() {
    let mut s = wizard();
    let before = s.current();
    s.on_wizard_field_filled("test answer".to_string());
    assert_eq!(s.current(), before + 1);
}

#[test]
fn on_wizard_fields_filled_with_minimal_completes_wizard() {
    let mut s = wizard();
    s.on_wizard_fields_filled(minimal_answers());
    assert!(
        s.is_complete(),
        "providing minimal answers should complete the wizard"
    );
}

// ── Review / completion ───────────────────────────────────────────────────────

#[test]
fn completing_all_steps_enters_review() {
    let mut s = wizard();
    for answer in minimal_answers() {
        s.on_wizard_field_filled(answer);
    }
    assert!(s.is_complete());
}

#[test]
fn review_renders_without_panic() {
    let mut s = wizard();
    for answer in minimal_answers() {
        s.on_wizard_field_filled(answer);
    }
    let _node = s.to_tui_node();
}

#[test]
fn review_enter_emits_wizard_complete() {
    let mut s = wizard();
    for answer in minimal_answers() {
        s.on_wizard_field_filled(answer);
    }
    let t = s.handle_key(key(KeyCode::Enter), &ctx());
    assert!(
        matches!(t, BotTransition::NarrativeWizardComplete { .. }),
        "Enter on review should emit NarrativeWizardComplete"
    );
}

#[test]
fn review_complete_carries_all_answers() {
    let mut s = wizard();
    let answers = minimal_answers();
    for answer in &answers {
        s.on_wizard_field_filled(answer.clone());
    }
    let t = s.handle_key(key(KeyCode::Enter), &ctx());
    if let BotTransition::NarrativeWizardComplete {
        answers: collected,
        path,
    } = t
    {
        assert_eq!(
            collected, answers,
            "collected answers must match what was submitted"
        );
        assert!(path.is_none(), "new wizard has no path");
    } else {
        panic!("expected NarrativeWizardComplete");
    }
}

// ── Agent blocks further input ────────────────────────────────────────────────

#[test]
fn agent_working_blocks_all_keys() {
    let mut s = wizard();
    s.handle_key(key(KeyCode::Char('2')), &ctx()); // '2' on Leaf triggers agent
    let t = s.handle_key(key(KeyCode::Char('1')), &ctx()); // blocked
    assert!(matches!(t, BotTransition::Stay));
    let t2 = s.handle_key(key(KeyCode::Char('3')), &ctx()); // blocked
    assert!(matches!(t2, BotTransition::Stay));
}

// ── Browser navigation ────────────────────────────────────────────────────────

#[test]
fn n_in_browser_opens_wizard() {
    use botticelli_tui::NarrativeBrowserScreen;
    let mut s = NarrativeBrowserScreen::new(None);
    let t = s.handle_key(key(KeyCode::Char('n')), &ctx());
    assert!(
        matches!(t, BotTransition::GoToNarrativeWizard { path: None }),
        "n in browser should open wizard for new file"
    );
}

#[test]
fn w_in_browser_opens_wizard_with_selected_path() {
    use botticelli_tui::NarrativeBrowserScreen;
    let mut s = NarrativeBrowserScreen::new(None);
    let t = s.handle_key(key(KeyCode::Char('w')), &ctx());
    assert!(
        matches!(t, BotTransition::GoToNarrativeWizard { .. }),
        "w in browser should open wizard"
    );
}

// ── Editor navigation ─────────────────────────────────────────────────────────

#[test]
fn w_in_editor_opens_wizard() {
    use botticelli_tui::NarrativeEditorScreen;
    let mut s = NarrativeEditorScreen::new_file();
    let t = s.handle_key(key(KeyCode::Char('w')), &ctx());
    assert!(
        matches!(t, BotTransition::GoToNarrativeWizard { .. }),
        "w in editor should open wizard"
    );
}
