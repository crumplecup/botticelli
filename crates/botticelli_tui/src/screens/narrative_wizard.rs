//! Interactive narrative creation wizard.
//!
//! Drives the user through `NarrativeGenerator`'s field sequence one step at a
//! time with a dynamically-built `VecDeque<WizardStep>`.  Steps are injected as
//! the user makes choices (Vec "add another?" loops and enum branch selection),
//! so the wizard always shows exactly the next question.
//!
//! At each step the wizard presents a numbered **action menu**.  Press a digit
//! key to act immediately — no separate insert mode required.
//!
//! - **Leaf step** — `1` opens text entry, `2` delegates to agent (next only),
//!   `3` delegates all remaining steps, `4` cancels the wizard.
//! - **Affirm step** — `1` = Yes, `2` = No, `3` = Cancel.
//! - **Select step** — `1`..`N` pick the N-th option; `N+1` cancels.

use std::collections::VecDeque;
use std::path::PathBuf;

use botticelli_narrative::NarrativeGenerator;
use crossterm::event::{KeyCode, KeyEvent};
use elicit_ratatui::{
    BlockJson, BordersJson, ConstraintJson, DirectionJson, ParagraphText, TuiNode, WidgetJson,
};
use tracing::{debug, info, instrument};

use crate::context::BotScreenContext;
use crate::screen::{BotScreen, BotTransition};

// ── Step types ────────────────────────────────────────────────────────────────

/// The interaction paradigm for a single wizard step.
#[derive(Debug, Clone)]
enum WizardStepKind {
    /// Free-text input (accessed via the action menu).
    Leaf,

    /// Pick one from a numbered list.
    ///
    /// `options` are display labels; `answers` are the strings pushed to the
    /// communicator (may equal `options` for enum variants, or be model IDs for
    /// a model picker).  `branches` lists zero or more sub-steps to inject into
    /// the pending queue after the choice is made.
    Select {
        options: Vec<String>,
        answers: Vec<String>,
        branches: Vec<Vec<WizardStep>>,
    },

    /// Binary yes/no.
    ///
    /// When `is_vec_loop` is true and the user says yes, the step re-enqueues
    /// itself after `on_yes` so the user is prompted again for each additional
    /// item.
    Affirm {
        on_yes: Vec<WizardStep>,
        is_vec_loop: bool,
    },
}

/// A single step in the dynamic wizard queue.
#[derive(Debug, Clone)]
struct WizardStep {
    path: Vec<String>,
    prompt: String,
    kind: WizardStepKind,
}

// ── Screen ────────────────────────────────────────────────────────────────────

/// Interactive wizard that guides the user through `NarrativeGenerator`.
pub struct NarrativeWizardScreen {
    /// Destination path (`None` for a new file until the first save).
    path: Option<PathBuf>,

    /// Steps yet to be presented.  The front is the active step.
    pending: VecDeque<WizardStep>,

    /// Raw answers in communicator order — fed to `BufferedCommunicator` on
    /// completion.
    answers: Vec<String>,

    /// Back-navigation history.  Each entry is the step that was answered and
    /// how many steps it injected into `pending` so `go_back` can undo exactly.
    history: Vec<(WizardStep, usize)>,

    /// Text buffer when the user is typing a Leaf answer.
    input: String,

    /// Whether the user is currently typing text for a Leaf step.
    typing: bool,

    /// Whether the agent is running in the background.
    agent_working: bool,

    /// Most-recent agent error, if any.
    agent_error: Option<String>,
}

impl NarrativeWizardScreen {
    /// Create a wizard for a new narrative.
    ///
    /// `current_model` is the model currently loaded in the driver (used to
    /// pre-populate the model selection list).
    #[instrument]
    pub fn new(path: Option<PathBuf>, current_model: Option<String>) -> Self {
        let pending = build_initial_steps(current_model);
        Self {
            path,
            pending,
            answers: Vec::new(),
            history: Vec::new(),
            input: String::new(),
            typing: false,
            agent_working: false,
            agent_error: None,
        }
    }

    // ── Public accessors ──────────────────────────────────────────────────────

    /// Number of steps completed so far.
    pub fn current(&self) -> usize {
        self.answers.len()
    }

    /// Whether all steps have been answered.
    pub fn is_complete(&self) -> bool {
        self.pending.is_empty()
    }

    /// Raw answers collected so far.
    pub fn answers(&self) -> &[String] {
        &self.answers
    }

    /// Prompt text for the currently active step, if any.
    pub fn current_prompt(&self) -> Option<&str> {
        self.pending.front().map(|s| s.prompt.as_str())
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    #[instrument(skip(self))]
    fn remaining_prompts(&self) -> Vec<String> {
        self.pending.iter().map(|s| s.prompt.clone()).collect()
    }

    /// Process a raw answer for the current (front) step.
    ///
    /// Pops the step, normalises the answer, pushes it to `self.answers`, and
    /// dynamically injects branch / loop steps as needed.  Records the step and
    /// injected count in `self.history` so `go_back` can undo it exactly.
    #[instrument(skip(self, raw))]
    fn push_answer(&mut self, raw: String) {
        let Some(step) = self.pending.pop_front() else {
            return;
        };
        self.input.clear();
        self.typing = false;
        self.agent_error = None;

        let pending_before = self.pending.len();

        match step.kind.clone() {
            WizardStepKind::Leaf => {
                self.answers.push(raw);
            }

            WizardStepKind::Select {
                options,
                answers,
                branches,
            } => {
                let n = options.len();
                let (idx, answer) = if let Ok(num) = raw.trim().parse::<usize>() {
                    let i = num.saturating_sub(1).min(n.saturating_sub(1));
                    (i, answers[i].clone())
                } else if let Some(i) = options
                    .iter()
                    .position(|o| o.eq_ignore_ascii_case(raw.trim()))
                {
                    (i, answers[i].clone())
                } else if let Some(i) = answers
                    .iter()
                    .position(|a| a.eq_ignore_ascii_case(raw.trim()))
                {
                    (i, answers[i].clone())
                } else {
                    (0, raw.trim().to_string())
                };

                self.answers.push(answer);

                if let Some(branch) = branches.into_iter().nth(idx) {
                    for s in branch.into_iter().rev() {
                        self.pending.push_front(s);
                    }
                }
            }

            WizardStepKind::Affirm {
                on_yes,
                is_vec_loop,
            } => {
                let yes = matches!(raw.trim().to_lowercase().as_str(), "true" | "yes" | "y");
                self.answers
                    .push(if yes { "true" } else { "false" }.to_string());

                if yes {
                    if is_vec_loop {
                        let loop_step = WizardStep {
                            path: step.path.clone(),
                            prompt: step.prompt.clone(),
                            kind: WizardStepKind::Affirm {
                                on_yes: on_yes.clone(),
                                is_vec_loop: true,
                            },
                        };
                        self.pending.push_front(loop_step);
                    }
                    for s in on_yes.into_iter().rev() {
                        self.pending.push_front(s);
                    }
                }
            }
        }

        let injected = self.pending.len().saturating_sub(pending_before);
        debug!(
            injected,
            answers = self.answers.len(),
            "Recorded history entry"
        );
        self.history.push((step, injected));
    }

    /// Undo the last answered step, restoring `pending` and popping the answer.
    #[instrument(skip(self))]
    fn go_back(&mut self) {
        let Some((step, injected)) = self.history.pop() else {
            debug!("go_back called with empty history — nothing to undo");
            return;
        };
        info!(
            injected,
            answers_remaining = self.answers.len().saturating_sub(1),
            step_prompt = %step.prompt,
            "Going back to previous wizard step"
        );
        for _ in 0..injected {
            self.pending.pop_front();
        }
        self.pending.push_front(step);
        self.answers.pop();
        self.typing = false;
        self.input.clear();
        self.agent_error = None;
    }

    /// Whether back-navigation is available.
    fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    #[instrument(skip(self))]
    fn step_kind_label(&self) -> &'static str {
        match self.pending.front().map(|s| &s.kind) {
            Some(WizardStepKind::Leaf) => "LEAF",
            Some(WizardStepKind::Select { .. }) => "SELECT",
            Some(WizardStepKind::Affirm { .. }) => "AFFIRM",
            None => "",
        }
    }

    #[instrument(skip(self))]
    fn mode_label(&self) -> &'static str {
        if self.agent_working {
            "AGENT"
        } else if self.typing {
            "TYPING"
        } else {
            "MENU"
        }
    }

    /// Build the numbered action menu text for the current step.
    ///
    /// Back appears as the second-to-last option (before Cancel) whenever
    /// there is history to return to.
    #[instrument(skip(self))]
    fn action_menu_text(&self) -> String {
        let Some(step) = self.pending.front() else {
            return String::new();
        };
        let back = self.can_go_back();
        match &step.kind {
            WizardStepKind::Leaf => {
                if back {
                    "  1. Type your answer\n  2. Agent fills this step\n  3. Agent fills all remaining\n  4. Back\n  5. Cancel wizard"
                        .to_string()
                } else {
                    "  1. Type your answer\n  2. Agent fills this step\n  3. Agent fills all remaining\n  4. Cancel wizard"
                        .to_string()
                }
            }
            WizardStepKind::Affirm { .. } => {
                if back {
                    "  1. Yes\n  2. No\n  3. Back\n  4. Cancel wizard".to_string()
                } else {
                    "  1. Yes\n  2. No\n  3. Cancel wizard".to_string()
                }
            }
            WizardStepKind::Select { options, .. } => {
                let mut lines: Vec<String> = options
                    .iter()
                    .enumerate()
                    .map(|(i, o)| format!("  {}. {}", i + 1, o))
                    .collect();
                let next = options.len() + 1;
                if back {
                    lines.push(format!("  {}. Back", next));
                    lines.push(format!("  {}. Cancel wizard", next + 1));
                } else {
                    lines.push(format!("  {}. Cancel wizard", next));
                }
                lines.join("\n")
            }
        }
    }

    /// Handle a digit key press in MENU mode.
    #[instrument(skip(self, n))]
    fn handle_menu_digit(&mut self, n: usize) -> BotTransition {
        let step_info = self
            .pending
            .front()
            .map(|s| (s.kind.clone(), s.prompt.clone()));
        let Some((kind, prompt)) = step_info else {
            return BotTransition::Stay;
        };

        let back = self.can_go_back();
        match kind {
            WizardStepKind::Leaf => match n {
                1 => {
                    self.typing = true;
                    BotTransition::Stay
                }
                2 => {
                    self.agent_working = true;
                    BotTransition::AgentFillNarrativeNext { prompt }
                }
                3 => {
                    let remaining = self.remaining_prompts();
                    if remaining.is_empty() {
                        return BotTransition::Stay;
                    }
                    let schema = schemars::schema_for!(NarrativeGenerator);
                    let json_schema = serde_json::to_string_pretty(&schema).unwrap_or_default();
                    self.agent_working = true;
                    BotTransition::AgentFillNarrativeAll {
                        remaining_prompts: remaining,
                        json_schema,
                    }
                }
                4 if back => {
                    self.go_back();
                    BotTransition::Stay
                }
                4 => BotTransition::GoToNarratives,
                5 if back => BotTransition::GoToNarratives,
                _ => BotTransition::Stay,
            },

            WizardStepKind::Affirm { .. } => match n {
                1 => {
                    self.push_answer("true".to_string());
                    BotTransition::Stay
                }
                2 => {
                    self.push_answer("false".to_string());
                    BotTransition::Stay
                }
                3 if back => {
                    self.go_back();
                    BotTransition::Stay
                }
                3 => BotTransition::GoToNarratives,
                4 if back => BotTransition::GoToNarratives,
                _ => BotTransition::Stay,
            },

            WizardStepKind::Select { options, .. } => {
                let n_opts = options.len();
                let back_n = n_opts + 1;
                let cancel_n = if back { n_opts + 2 } else { n_opts + 1 };
                if n == cancel_n {
                    BotTransition::GoToNarratives
                } else if back && n == back_n {
                    self.go_back();
                    BotTransition::Stay
                } else if n >= 1 && n <= n_opts {
                    self.push_answer(n.to_string());
                    BotTransition::Stay
                } else {
                    BotTransition::Stay
                }
            }
        }
    }

    #[instrument(skip(text, title, bordered))]
    fn paragraph(text: impl Into<String>, title: Option<String>, bordered: bool) -> TuiNode {
        TuiNode::Widget {
            widget: Box::new(WidgetJson::Paragraph {
                text: ParagraphText::Plain(text.into()),
                style: None,
                wrap: true,
                scroll: None,
                alignment: None,
                block: if bordered {
                    Some(BlockJson {
                        title,
                        borders: BordersJson::All,
                        border_type: None,
                        style: None,
                        border_style: None,
                        padding: None,
                    })
                } else {
                    None
                },
            }),
        }
    }
}

impl BotScreen for NarrativeWizardScreen {
    #[instrument(skip(self))]
    fn to_tui_node(&self) -> TuiNode {
        if self.is_complete() {
            return self.review_node();
        }

        let step = self.pending.front();
        let prompt_text = step.map(|s| s.prompt.as_str()).unwrap_or("").to_string();
        let step_path = step.map(|s| s.path.join(" › ")).unwrap_or_default();

        let header = Self::paragraph(
            format!(
                "Completed: {}  ·  {}  [{} / {}]",
                self.answers.len(),
                step_path,
                self.mode_label(),
                self.step_kind_label(),
            ),
            Some("Narrative Wizard".to_string()),
            true,
        );

        let prompt = Self::paragraph(
            format!("  {}", prompt_text),
            Some("Question".to_string()),
            true,
        );

        let upcoming_text = {
            let lines: String = self
                .pending
                .iter()
                .skip(1)
                .take(5)
                .map(|s| format!("  · {}\n", s.path.join(" › ")))
                .collect();
            let remaining = self.pending.len().saturating_sub(1);
            if lines.is_empty() {
                "  (this is the last step)".to_string()
            } else if remaining > 5 {
                format!(
                    "{}  … and {} more (may grow with choices)",
                    lines.trim_end(),
                    remaining - 5
                )
            } else {
                lines
            }
        };
        let upcoming = Self::paragraph(upcoming_text, Some("Upcoming".to_string()), true);

        let error_suffix = self
            .agent_error
            .as_ref()
            .map(|e| format!("  ⚠  {}", e))
            .unwrap_or_default();

        if self.agent_working {
            let working =
                Self::paragraph("  [Agent is thinking…]", Some("Actions".to_string()), true);
            let help = Self::paragraph("  (waiting for agent…)".to_string(), None, false);
            return TuiNode::Layout {
                direction: DirectionJson::Vertical,
                constraints: vec![
                    ConstraintJson::Length { value: 3 },
                    ConstraintJson::Length { value: 7 },
                    ConstraintJson::Fill { value: 1 },
                    ConstraintJson::Fill { value: 1 },
                    ConstraintJson::Length { value: 1 },
                ],
                children: vec![header, prompt, working, upcoming, help],
                margin: None,
            };
        }

        if self.typing {
            let input_display = format!("  ▶ {}", self.input);
            let input_node = Self::paragraph(
                input_display,
                Some("Your answer  (Enter=submit   Esc=back to menu)".to_string()),
                true,
            );
            let help = Self::paragraph(
                format!("  Enter=submit   Esc=back to menu{}", error_suffix),
                None,
                false,
            );
            return TuiNode::Layout {
                direction: DirectionJson::Vertical,
                constraints: vec![
                    ConstraintJson::Length { value: 3 },
                    ConstraintJson::Length { value: 7 },
                    ConstraintJson::Length { value: 3 },
                    ConstraintJson::Fill { value: 1 },
                    ConstraintJson::Length { value: 1 },
                ],
                children: vec![header, prompt, input_node, upcoming, help],
                margin: None,
            };
        }

        // MENU mode — show partitioned action menu
        let action_text = self.action_menu_text();
        let action_node = Self::paragraph(
            action_text,
            Some("Actions  (press a number key)".to_string()),
            true,
        );
        let back_hint = if self.can_go_back() {
            "   (Back option in menu to undo last answer)"
        } else {
            ""
        };
        let help = Self::paragraph(
            format!(
                "  Press a number key to act   Esc=cancel wizard{}{}",
                back_hint, error_suffix
            ),
            None,
            false,
        );

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Length { value: 3 },
                ConstraintJson::Length { value: 7 },
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 1 },
            ],
            children: vec![header, prompt, action_node, upcoming, help],
            margin: None,
        }
    }

    #[instrument(skip(self, _ctx))]
    fn handle_key(&mut self, key: KeyEvent, _ctx: &BotScreenContext) -> BotTransition {
        if self.agent_working {
            return BotTransition::Stay;
        }

        if self.is_complete() {
            return self.handle_review_key(key);
        }

        if self.typing {
            return self.handle_typing_key(key);
        }

        // MENU mode — digit keys act immediately
        match key.code {
            KeyCode::Esc => BotTransition::GoToNarratives,
            KeyCode::Char('1') => self.handle_menu_digit(1),
            KeyCode::Char('2') => self.handle_menu_digit(2),
            KeyCode::Char('3') => self.handle_menu_digit(3),
            KeyCode::Char('4') => self.handle_menu_digit(4),
            KeyCode::Char('5') => self.handle_menu_digit(5),
            KeyCode::Char('6') => self.handle_menu_digit(6),
            KeyCode::Char('7') => self.handle_menu_digit(7),
            KeyCode::Char('8') => self.handle_menu_digit(8),
            KeyCode::Char('9') => self.handle_menu_digit(9),
            _ => BotTransition::Stay,
        }
    }

    fn screen_name(&self) -> &'static str {
        "Narrative Wizard"
    }

    fn on_wizard_field_filled(&mut self, answer: String) {
        self.agent_working = false;
        if !self.pending.is_empty() {
            self.push_answer(answer);
        }
    }

    fn on_wizard_fields_filled(&mut self, answers: Vec<String>) {
        self.agent_working = false;
        for answer in answers {
            if self.pending.is_empty() {
                break;
            }
            self.push_answer(answer);
        }
    }
}

impl NarrativeWizardScreen {
    #[instrument(skip(self))]
    fn handle_typing_key(&mut self, key: KeyEvent) -> BotTransition {
        match key.code {
            KeyCode::Esc => {
                self.typing = false;
                BotTransition::Stay
            }

            KeyCode::Enter => {
                if self.input.trim().is_empty() {
                    BotTransition::Stay
                } else {
                    let answer = self.input.trim().to_string();
                    self.push_answer(answer);
                    BotTransition::Stay
                }
            }

            KeyCode::Backspace => {
                self.input.pop();
                BotTransition::Stay
            }

            KeyCode::Char(c) => {
                self.input.push(c);
                BotTransition::Stay
            }

            _ => BotTransition::Stay,
        }
    }

    fn review_node(&self) -> TuiNode {
        let header = Self::paragraph(
            format!(
                "  {} answers collected — ready to build narrative.",
                self.answers.len()
            ),
            Some("Narrative Wizard — Review".to_string()),
            true,
        );

        let rows: String = self
            .answers
            .iter()
            .enumerate()
            .map(|(i, ans)| {
                let short = if ans.len() > 60 {
                    format!("{}…", &ans[..60])
                } else {
                    ans.clone()
                };
                format!("  {}. {}\n", i + 1, short)
            })
            .collect();

        let answers_block = Self::paragraph(rows, Some("Answers".to_string()), true);

        let help = Self::paragraph(
            "  Enter=save and go to browser   Esc=discard and go to browser   q=quit".to_string(),
            None,
            false,
        );

        TuiNode::Layout {
            direction: DirectionJson::Vertical,
            constraints: vec![
                ConstraintJson::Length { value: 3 },
                ConstraintJson::Fill { value: 1 },
                ConstraintJson::Length { value: 1 },
            ],
            children: vec![header, answers_block, help],
            margin: None,
        }
    }

    #[instrument(skip(self))]
    fn handle_review_key(&self, key: KeyEvent) -> BotTransition {
        match key.code {
            KeyCode::Enter => BotTransition::NarrativeWizardComplete {
                answers: self.answers.clone(),
                path: self.path.clone(),
            },
            KeyCode::Esc => BotTransition::GoToNarratives,
            KeyCode::Char('q') | KeyCode::Char('Q') => BotTransition::Quit,
            _ => BotTransition::Stay,
        }
    }
}

// ── Step templates ─────────────────────────────────────────────────────────────

/// Build the initial step queue for `NarrativeGenerator`.
///
/// The step sequence matches exactly what `NarrativeGenerator::elicit()` will
/// ask via its communicator, so the collected `answers` can be replayed through
/// `BufferedCommunicator` without modification.
#[instrument]
fn build_initial_steps(current_model: Option<String>) -> VecDeque<WizardStep> {
    let mut q: VecDeque<WizardStep> = VecDeque::new();

    // ── 1. Name ───────────────────────────────────────────────────────────────
    q.push_back(WizardStep {
        path: vec!["name".into()],
        prompt: "Narrative name — a short identifier used as the file key (e.g. 'article_draft'):"
            .into(),
        kind: WizardStepKind::Leaf,
    });

    // ── 2. Description ────────────────────────────────────────────────────────
    q.push_back(WizardStep {
        path: vec!["description".into()],
        prompt: "Describe what this narrative does (shown in logs and tooling):".into(),
        kind: WizardStepKind::Leaf,
    });

    // ── 3. Acts Vec — Affirm loop ─────────────────────────────────────────────
    q.push_back(acts_affirm());

    // ── 4. Model Option<String> — Affirm + model select ──────────────────────
    q.push_back(model_affirm(current_model));

    q
}

/// Build the Affirm step that opens (and loops) the act-definition flow.
#[instrument]
fn acts_affirm() -> WizardStep {
    WizardStep {
        path: vec!["acts".into()],
        prompt: "Add an act? Acts are the AI-prompt steps that make up the narrative.".into(),
        kind: WizardStepKind::Affirm {
            on_yes: act_steps(),
            is_vec_loop: true,
        },
    }
}

/// Steps that elicit a single `NarrativeActSpec`.
#[instrument]
fn act_steps() -> Vec<WizardStep> {
    vec![
        WizardStep {
            path: vec!["acts".into(), "name".into()],
            prompt: "Act name — a short identifier used as the key (e.g. 'research', 'draft', 'review'):".into(),
            kind: WizardStepKind::Leaf,
        },
        WizardStep {
            path: vec!["acts".into(), "content".into()],
            prompt: "What type of act is this?".into(),
            kind: WizardStepKind::Select {
                options: vec!["Prompt".into(), "NarrativeRef".into(), "Carousel".into()],
                answers: vec!["Prompt".into(), "NarrativeRef".into(), "Carousel".into()],
                branches: vec![
                    // Prompt branch
                    vec![WizardStep {
                        path: vec!["acts".into(), "content".into(), "Prompt".into()],
                        prompt: "Write the prompt text that will be sent to the LLM for this act:".into(),
                        kind: WizardStepKind::Leaf,
                    }],
                    // NarrativeRef branch
                    vec![WizardStep {
                        path: vec!["acts".into(), "content".into(), "NarrativeRef".into()],
                        prompt: "Enter the key of the narrative to run as this act (the file stem, e.g. 'research'):".into(),
                        kind: WizardStepKind::Leaf,
                    }],
                    // Carousel branch
                    vec![
                        WizardStep {
                            path: vec![
                                "acts".into(),
                                "content".into(),
                                "Carousel".into(),
                                "prompt".into(),
                            ],
                            prompt: "Prompt to use for each carousel iteration:".into(),
                            kind: WizardStepKind::Leaf,
                        },
                        WizardStep {
                            path: vec![
                                "acts".into(),
                                "content".into(),
                                "Carousel".into(),
                                "iterations".into(),
                            ],
                            prompt: "Maximum number of iterations (e.g. 10):".into(),
                            kind: WizardStepKind::Leaf,
                        },
                    ],
                ],
            },
        },
    ]
}

/// Build the model selection Affirm + Select for `Option<String>` model field.
#[instrument]
fn model_affirm(current: Option<String>) -> WizardStep {
    let current_hint = current
        .as_deref()
        .map(|m| format!(" (currently loaded: {})", m))
        .unwrap_or_default();

    let models = known_models(current);

    WizardStep {
        path: vec!["model".into()],
        prompt: format!("Use a custom LLM model for all acts?{}", current_hint),
        kind: WizardStepKind::Affirm {
            on_yes: vec![WizardStep {
                path: vec!["model".into(), "value".into()],
                prompt: "Choose a model for all acts:".into(),
                kind: WizardStepKind::Select {
                    options: models.clone(),
                    answers: models,
                    branches: vec![],
                },
            }],
            is_vec_loop: false,
        },
    }
}

/// Ordered list of known/common model IDs.
///
/// The currently-loaded model (if provided) appears first so option "1" always
/// selects the default in-use model.
#[instrument]
fn known_models(current: Option<String>) -> Vec<String> {
    let mut base: Vec<String> = vec![
        "claude-sonnet-4-6".into(),
        "claude-opus-4-8".into(),
        "claude-haiku-4-5-20251001".into(),
        "gemini-2.0-flash".into(),
        "gemini-1.5-pro-latest".into(),
        "ollama/llama3.2".into(),
    ];
    if let Some(m) = current {
        if !base.contains(&m) {
            base.insert(0, m);
        } else {
            let idx = base.iter().position(|x| *x == m).unwrap_or(0);
            base.remove(idx);
            base.insert(0, m);
        }
    }
    base
}
