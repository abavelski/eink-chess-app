//! Pure, platform-independent puzzle text presentation.
//! The rich parser has already applied content/comment and description fallbacks.
//! This module never changes a puzzle, grading state, or saved progress.

use crate::analysis::ContentSpan;
use crate::puzzle::Puzzle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusMode {
    Solution,
    FreeBoard,
}

/// Text for the ordinary board. A solved attempt reveals an accessible *plain*
/// projection of the authoritative description spans; move refs retain their
/// authored labels (or resolve to SAN), but are not clickable in this view.
#[derive(Debug, Eq, PartialEq)]
pub struct SolutionText {
    pub status: String,
    pub revealed_description: Option<String>,
}

/// The analysis reader's independent text source. The preface is accessible
/// before solving, and must be emitted only once, not alongside the fallback
/// compatibility `description`. Task 06 will handle link layout/pagination.
#[derive(Debug)]
pub struct AnalysisText<'a> {
    pub preface: &'a [ContentSpan],
    pub root_content: Option<&'a [ContentSpan]>,
    has_tree: bool,
}

impl AnalysisText<'_> {
    /// Rich trees are browsable even when their preface and root are blank.
    /// A legacy description-only collection can still open a reader.
    pub fn available(&self) -> bool {
        self.has_tree || !self.preface.is_empty()
    }
}

pub fn analysis_text(puzzle: &Puzzle) -> AnalysisText<'_> {
    let preface = puzzle.description_content.as_slice();
    let root_content = puzzle
        .analysis
        .as_ref()
        .and_then(|tree| tree.node(tree.root()))
        .map(|root| root.content())
        // Duplicate authored root prose should not repeat the preface.
        .filter(|content| !content.is_empty() && *content != preface);

    AnalysisText {
        preface,
        root_content,
        has_tree: puzzle.analysis.is_some(),
    }
}

pub fn solution_text(
    puzzle: &Puzzle,
    mode: StatusMode,
    correct: bool,
    solved: bool,
    description_revealed: bool,
) -> SolutionText {
    if mode == StatusMode::FreeBoard {
        // Free Board has its own status; never show puzzle topics, feedback,
        // or a stale auto-revealed description in this mode.
        return SolutionText {
            status: "Free board".into(),
            revealed_description: None,
        };
    }

    // Preserve authored whitespace, newlines, and Unicode; only *blank*
    // topics fall back to the useful side-to-move label.
    let base = puzzle
        .topic
        .as_deref()
        .filter(|topic| !topic.trim().is_empty())
        .unwrap_or_else(|| match puzzle.side_to_move() {
            crate::board::Color::White => "White to move",
            crate::board::Color::Black => "Black to move",
        });
    let status = if correct {
        format!("{base} · Correct")
    } else if solved {
        format!("{base} · Solved")
    } else {
        base.to_owned()
    };

    SolutionText {
        status,
        revealed_description: if description_revealed {
            plain_preface(puzzle)
        } else {
            None
        },
    }
}

/// This projection is for the noninteractive solved status area only.
/// The actual interactive spans remain intact for the analysis reader.
fn plain_preface(puzzle: &Puzzle) -> Option<String> {
    let mut text = String::new();
    for span in &puzzle.description_content {
        match span {
            ContentSpan::Text(prose) => text.push_str(prose),
            ContentSpan::MoveRef { node, label } => {
                if let Some(label) = label {
                    text.push_str(label);
                } else if let Some(movement) = puzzle
                    .analysis
                    .as_ref()
                    .and_then(|tree| tree.node(*node))
                    .and_then(|node| node.movement())
                {
                    text.push_str(&movement.san);
                }
            }
        }
    }
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::parse_puzzle_file;
    use serde_json::{json, Value};

    fn sample() -> Value {
        json!({"version": 1, "puzzles": [{
            "id": "plain",
            "fen": "7k/8/5KQ1/8/8/8/8/8 w - - 0 1",
            "solution": ["g6g7"]
        }]})
    }

    fn parse(value: Value) -> Puzzle {
        parse_puzzle_file(&serde_json::to_vec(&value).unwrap())
            .unwrap()
            .puzzles
            .remove(0)
    }

    fn view(puzzle: &Puzzle, revealed: bool) -> SolutionText {
        solution_text(puzzle, StatusMode::Solution, false, false, revealed)
    }

    #[test]
    fn no_topic_or_description_and_free_board_have_independent_status() {
        let puzzle = parse(sample());
        assert_eq!(view(&puzzle, false).status, "White to move");
        assert_eq!(view(&puzzle, true).revealed_description, None);
        let reader = analysis_text(&puzzle);
        assert!(!reader.available());
        assert!(reader.preface.is_empty());
        assert!(reader.root_content.is_none());
        let free = solution_text(&puzzle, StatusMode::FreeBoard, true, true, true);
        assert_eq!(free.status, "Free board");
        assert_eq!(free.revealed_description, None);
    }

    #[test]
    fn topic_is_verbatim_unicode_multiline_and_does_not_repeat_turn() {
        let mut data = sample();
        data["puzzles"][0]["topic"] = json!("Цугцванг ♞\nMate in 2");
        let puzzle = parse(data);
        assert_eq!(view(&puzzle, false).status, "Цугцванг ♞\nMate in 2");
        assert_eq!(
            solution_text(&puzzle, StatusMode::Solution, true, false, false).status,
            "Цугцванг ♞\nMate in 2 · Correct"
        );
        assert_eq!(
            solution_text(&puzzle, StatusMode::Solution, false, true, false).status,
            "Цугцванг ♞\nMate in 2 · Solved"
        );
        assert_eq!(
            solution_text(&puzzle, StatusMode::FreeBoard, false, true, false).status,
            "Free board"
        );
    }

    #[test]
    fn blank_null_nonstring_topic_falls_back_to_side_to_move() {
        for topic in [
            json!(null),
            json!(123),
            json!([]),
            json!({}),
            json!(""),
            json!(" \n\t "),
        ] {
            let mut data = sample();
            data["puzzles"][0]["topic"] = topic;
            assert_eq!(view(&parse(data), false).status, "White to move");
        }
        let mut absent = sample();
        absent["puzzles"][0]
            .as_object_mut()
            .unwrap()
            .remove("topic");
        assert_eq!(view(&parse(absent), false).status, "White to move");
        let mut black = sample();
        black["puzzles"][0]["fen"] = json!("8/8/8/8/8/5kq1/8/7K b - - 0 1");
        assert_eq!(view(&parse(black), false).status, "Black to move");
    }

    #[test]
    fn legacy_description_is_preface_before_solve_and_revealed_on_solve() {
        let mut data = sample();
        data["puzzles"][0]["description"] = json!("Legacy Nd5 is plain.\nСтрока два.");
        let puzzle = parse(data);
        let reader = analysis_text(&puzzle);
        assert!(reader.available());
        assert_eq!(
            reader.preface,
            &[ContentSpan::Text(
                "Legacy Nd5 is plain.\nСтрока два.".into()
            )]
        );
        assert!(reader.root_content.is_none());
        assert_eq!(view(&puzzle, false).revealed_description, None);
        assert_eq!(
            view(&puzzle, true).revealed_description.as_deref(),
            Some("Legacy Nd5 is plain.\nСтрока два.")
        );
        assert_eq!(puzzle.solution, ["g6g7"]);
    }

    #[test]
    fn authoritative_rich_spans_resolve_move_refs_without_using_plain_compatibility() {
        let puzzle = parse_puzzle_file(include_bytes!(
            "../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json"
        ))
        .unwrap()
        .puzzles
        .remove(0);
        let reader = analysis_text(&puzzle);
        assert!(reader.available());
        assert_eq!(reader.preface, puzzle.description_content.as_slice());
        assert!(reader.root_content.is_some());
        assert_ne!(reader.root_content.unwrap(), reader.preface);
        assert_eq!(
            view(&puzzle, true).revealed_description.as_deref(),
            Some("Choose 1...h1=Q+ or inspect h1=N. The text h1=N is deliberately plain.")
        );
        assert_eq!(view(&puzzle, false).revealed_description, None);
        // Rich preface appears exactly once, without falling back to description.
        assert_ne!(
            view(&puzzle, true).revealed_description.as_deref(),
            puzzle.description.as_deref()
        );
    }

    #[test]
    fn explicit_empty_overrides_legacy_and_blank_description_remains_hidden() {
        for description in [json!(null), json!(""), json!(" \n ")] {
            let mut data = sample();
            data["puzzles"][0]["description"] = description;
            let puzzle = parse(data);
            assert!(view(&puzzle, true).revealed_description.is_none());
        }

        let mut data = sample();
        data["puzzles"][0]["description"] = json!("Old fallback must not be shown");
        data["puzzles"][0]["description_content"] = json!([]);
        let puzzle = parse(data);
        assert!(analysis_text(&puzzle).preface.is_empty());
        assert!(!analysis_text(&puzzle).available());
        assert!(view(&puzzle, true).revealed_description.is_none());
    }

    #[test]
    fn root_content_takes_precedence_over_comment_and_is_not_duplicated() {
        let mut data: Value = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json"
        ))
        .unwrap();
        data["puzzles"][0]["analysis"]["nodes"][0]["comment"] = json!("Legacy root prose");
        data["puzzles"][0]["analysis"]["nodes"][0]["content"] = json!([]);
        let puzzle = parse(data.clone());
        assert!(analysis_text(&puzzle).root_content.is_none());

        data["puzzles"][0]["analysis"]["nodes"][0]["content"] =
            data["puzzles"][0]["description_content"].clone();
        let puzzle = parse(data);
        let reader = analysis_text(&puzzle);
        assert!(reader.available());
        assert!(reader.root_content.is_none(), "do not repeat the preface");
    }

    #[test]
    fn long_multilingual_topic_and_description_are_not_truncated_by_view_model() {
        let topic = "Шахматы\nÉchecs ♜\n".repeat(300);
        let description = "Русский 中文 العربية ♞\n".repeat(500);
        let mut data = sample();
        data["puzzles"][0]["topic"] = json!(topic);
        data["puzzles"][0]["description"] = json!(description);
        let puzzle = parse(data);
        assert_eq!(view(&puzzle, false).status, topic);
        assert_eq!(
            view(&puzzle, true).revealed_description.as_deref(),
            Some(description.as_str())
        );
        assert_eq!(analysis_text(&puzzle).preface.len(), 1);
    }
}
