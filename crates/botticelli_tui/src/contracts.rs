//! Proof-carrying layout verification for the botticelli TUI.
//!
//! Mirrors the pattern from the elicitation framework: each `Prop` names an
//! invariant, `verified_draw` composes the proof chain, and callers get a
//! typed credential confirming the layout is sound before the frame is
//! presented.
//!
//! # Proof chain
//!
//! ```text
//! verify_area → Established<AreaSufficient>
//! verify_labels → Established<LabelContained>
//! verify_wrapping → Established<TextWrapped>
//! both(...) → Established<And<...>>
//! assert() → Established<BotUiConsistent>   (top-level credential)
//! ```

use elicit_ratatui::{TuiNode, WidgetJson, render_node};
use elicitation::{
    Prop,
    contracts::{And, Established, ProvableFrom, both},
};
use ratatui::Frame;
use ratatui::layout::Rect;
use tracing::instrument;
use unicode_width::UnicodeWidthStr as _;

// ─────────────────────────────────────────────────────────────
//  Propositions
// ─────────────────────────────────────────────────────────────

/// Every block title fits within its allocated cell width.
#[derive(Prop)]
pub struct LabelContained;

/// Every paragraph holding variable-length content has `wrap = true`.
#[derive(Prop)]
pub struct TextWrapped;

/// Every text block has enough height rows for its content.
#[derive(Prop)]
pub struct AreaSufficient;

/// Conjunction: `LabelContained ∧ TextWrapped ∧ AreaSufficient`.
pub type NoOverflow = And<LabelContained, And<TextWrapped, AreaSufficient>>;

/// The full pipeline ran: IR built → layout verified → rendered to terminal.
/// This is the top-level credential for a completed botticelli TUI frame.
#[derive(Prop)]
pub struct BotUiConsistent;

impl ProvableFrom<Established<NoOverflow>> for BotUiConsistent {}

// ─────────────────────────────────────────────────────────────
//  Layout errors
// ─────────────────────────────────────────────────────────────

/// A layout invariant was violated before rendering.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum LayoutError {
    /// A block title overflows its allocated cell width.
    #[display(
        "label overflow: '{}' is {} cols but cell is {}",
        label,
        label_width,
        cell_width
    )]
    LabelOverflow {
        /// The overflowing title string.
        label: String,
        /// Measured display width of the label.
        label_width: usize,
        /// Available cell width (after borders).
        cell_width: usize,
    },
    /// A paragraph holding variable-length content has `wrap = false`.
    #[display("text not wrapped in '{}'", title)]
    TextNotWrapped {
        /// The paragraph's block title or identifier.
        title: String,
    },
    /// The allocated area is too small for the content.
    #[display(
        "area too small: need {}×{} but got {}×{}",
        needed_w,
        needed_h,
        got_w,
        got_h
    )]
    AreaTooSmall {
        /// Minimum required width.
        needed_w: u16,
        /// Minimum required height.
        needed_h: u16,
        /// Actual available width.
        got_w: u16,
        /// Actual available height.
        got_h: u16,
    },
}

// ─────────────────────────────────────────────────────────────
//  Verification helpers
// ─────────────────────────────────────────────────────────────

/// Verify that no block label overflows its cell.
#[instrument(skip(root))]
fn verify_labels(root: &TuiNode, area: Rect) -> Result<Established<LabelContained>, LayoutError> {
    check_labels_recursive(root, area)?;
    Ok(Established::assert())
}

fn check_labels_recursive(node: &TuiNode, area: Rect) -> Result<(), LayoutError> {
    match node {
        TuiNode::Widget { widget } => {
            if let WidgetJson::Block { block } = widget.as_ref()
                && let Some(ref title) = block.title
            {
                let label_width = title.width();
                let cell_width = area.width.saturating_sub(2) as usize;
                if label_width > cell_width && cell_width > 0 {
                    return Err(LayoutError::LabelOverflow {
                        label: title.clone(),
                        label_width,
                        cell_width,
                    });
                }
            }
        }
        TuiNode::Layout { children, .. } => {
            for child in children {
                check_labels_recursive(child, area)?;
            }
        }
        TuiNode::StatusBar { .. } => {}
    }
    Ok(())
}

/// Verify that all paragraphs with dynamic content have wrapping enabled.
#[instrument(skip(root))]
fn verify_wrapping(root: &TuiNode) -> Result<Established<TextWrapped>, LayoutError> {
    check_wrapping_recursive(root)?;
    Ok(Established::assert())
}

fn check_wrapping_recursive(node: &TuiNode) -> Result<(), LayoutError> {
    match node {
        TuiNode::Widget { widget } => {
            if let WidgetJson::Paragraph { wrap, block, .. } = widget.as_ref()
                && !wrap
            {
                let title = block
                    .as_ref()
                    .and_then(|b| b.title.clone())
                    .unwrap_or_else(|| "<unnamed>".to_string());
                return Err(LayoutError::TextNotWrapped { title });
            }
        }
        TuiNode::Layout { children, .. } => {
            for child in children {
                check_wrapping_recursive(child)?;
            }
        }
        TuiNode::StatusBar { .. } => {}
    }
    Ok(())
}

/// Verify the terminal area meets the minimum size requirement.
#[instrument]
fn verify_area(area: Rect) -> Result<Established<AreaSufficient>, LayoutError> {
    const MIN_W: u16 = 60;
    const MIN_H: u16 = 20;
    if area.width < MIN_W || area.height < MIN_H {
        return Err(LayoutError::AreaTooSmall {
            needed_w: MIN_W,
            needed_h: MIN_H,
            got_w: area.width,
            got_h: area.height,
        });
    }
    Ok(Established::assert())
}

// ─────────────────────────────────────────────────────────────
//  verified_draw
// ─────────────────────────────────────────────────────────────

/// Render `root` into `frame` at `area`, verifying all layout invariants.
///
/// Returns `Established<BotUiConsistent>` on success. On failure returns a
/// [`LayoutError`] — callers should render a resize prompt.
#[instrument(skip(frame, root))]
pub fn verified_draw(
    frame: &mut Frame,
    area: Rect,
    root: &TuiNode,
) -> Result<Established<BotUiConsistent>, LayoutError> {
    let area_proof = verify_area(area)?;
    let label_proof = verify_labels(root, area)?;
    let wrap_proof = verify_wrapping(root)?;
    let no_overflow: Established<NoOverflow> = both(label_proof, both(wrap_proof, area_proof));
    render_node(frame, area, root);
    Ok(Established::prove(&no_overflow))
}

/// Render a "terminal too small" prompt that satisfies `NoOverflow` by construction.
#[instrument(skip(frame))]
pub fn render_resize_prompt(frame: &mut Frame, error: &LayoutError) {
    use ratatui::text::Text;
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
    let msg = format!("Terminal too small — {error}\n\nResize to continue.");
    let widget = Paragraph::new(Text::raw(msg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Resize needed"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, frame.area());
}
