//! Deterministic, UI-independent PGN-like analysis projection.
//!
//! Runs retain their node identity: only actual SAN moves and authored move_ref
//! spans are interactive. Prose (including notation-looking prose), punctuation
//! and move numbers are never inferred as links. Layout may wrap or paginate
//! these runs without rebuilding the analysis traversal.

use crate::analysis::{AnalysisTree, ContentSpan, NodeHandle};
use crate::presentation::analysis_text;
use crate::puzzle::Puzzle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunOrigin {
    Prose,
    TreeMove,
    ExplicitMoveRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisRun {
    pub text: String,
    pub origin: RunOrigin,
    /// Present only for a tree SAN move or an explicitly authored move_ref.
    pub target: Option<NodeHandle>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnalysisDocument {
    pub runs: Vec<AnalysisRun>,
}

impl AnalysisDocument {
    /// Lossless, unwrapped text for diagnostics, accessibility and tests.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for run in &self.runs {
            text.push_str(&run.text);
        }
        text
    }
}

/// Project a parsed puzzle into ordered semantic runs, without mutating its
/// board or analysis state. Works for both rich trees and legacy descriptions.
///
/// The parser has already resolved content fallbacks and validated move_refs.
/// As in the existing reader text model, identical root and preface content
/// is shown only once.
pub fn project_analysis(puzzle: &Puzzle) -> AnalysisDocument {
    let view = analysis_text(puzzle);
    let mut builder = Builder::default();
    if !view.preface.is_empty() {
        builder.spans(view.preface, puzzle.analysis.as_ref());
        builder.break_line();
    }
    if let Some(root) = view.root_content {
        builder.spans(root, puzzle.analysis.as_ref());
        builder.break_line();
    }

    if let Some(tree) = puzzle.analysis.as_ref() {
        // A position emits its first ordered move, then each remaining child
        // as a parenthesized RAV, then the first move's continuation. Roles
        // never change presentation order. An explicit stack bounds traversal
        // by heap memory rather than by the call stack.
        enum Work {
            Position(NodeHandle, bool),
            Branch(NodeHandle),
            Move(NodeHandle, bool),
            Open,
            Close,
        }

        let mut work = vec![Work::Position(tree.root(), true)];
        while let Some(item) = work.pop() {
            match item {
                Work::Position(parent, force_number) => {
                    let children = tree.node(parent).expect("validated parent").children();
                    let Some((&first, variations)) = children.split_first() else {
                        continue;
                    };
                    let first_node = tree.node(first).expect("validated child");
                    work.push(Work::Position(
                        first,
                        !variations.is_empty() || !first_node.content().is_empty(),
                    ));
                    for &sibling in variations.iter().rev() {
                        work.push(Work::Close);
                        work.push(Work::Branch(sibling));
                        work.push(Work::Open);
                    }
                    work.push(Work::Move(first, force_number));
                }
                Work::Branch(node) => {
                    let content = tree.node(node).expect("validated child").content();
                    work.push(Work::Position(node, !content.is_empty()));
                    work.push(Work::Move(node, true));
                }
                Work::Move(node, force_number) => builder.tree_move(tree, node, force_number),
                Work::Open => {
                    builder.separator();
                    builder.plain("(");
                }
                Work::Close => builder.plain(")"),
            }
        }
    }

    AnalysisDocument { runs: builder.runs }
}

#[derive(Default)]
struct Builder {
    runs: Vec<AnalysisRun>,
    last: Option<char>,
}

impl Builder {
    fn push(&mut self, text: &str, origin: RunOrigin, target: Option<NodeHandle>) {
        if text.is_empty() {
            return;
        }
        // Adjacent inert text can share a run. Never merge links, even when
        // two explicitly authored references have the same visible label.
        if origin == RunOrigin::Prose {
            if let Some(run) = self.runs.last_mut() {
                if run.origin == RunOrigin::Prose {
                    run.text.push_str(text);
                    self.last = text.chars().last();
                    return;
                }
            }
        }
        self.runs.push(AnalysisRun {
            text: text.to_owned(),
            origin,
            target,
        });
        self.last = text.chars().last();
    }

    fn plain(&mut self, text: &str) {
        self.push(text, RunOrigin::Prose, None);
    }

    fn separator(&mut self) {
        if self.last.is_some_and(|c| !c.is_whitespace() && c != '(') {
            self.plain(" ");
        }
    }

    fn break_line(&mut self) {
        if self.last.is_some_and(|c| c != '\n') {
            self.plain("\n");
        }
    }

    fn spans(&mut self, spans: &[ContentSpan], tree: Option<&AnalysisTree>) {
        for span in spans {
            match span {
                ContentSpan::Text(text) => self.plain(text),
                ContentSpan::MoveRef { node, label } => {
                    let target = tree
                        .and_then(|tree| tree.node(*node))
                        .and_then(|node| node.movement())
                        .expect("validated move_ref must target a move");
                    self.push(
                        label.as_deref().unwrap_or(&target.san),
                        RunOrigin::ExplicitMoveRef,
                        Some(*node),
                    );
                }
            }
        }
    }

    fn content(&mut self, spans: &[ContentSpan], tree: &AnalysisTree) {
        // A leading authored whitespace (including a newline) is itself the
        // boundary; do not silently replace or double it.
        if !matches!(spans.first(), Some(ContentSpan::Text(text))
            if text.chars().next().is_some_and(char::is_whitespace))
        {
            self.separator();
        }
        self.spans(spans, Some(tree));
    }

    fn tree_move(&mut self, tree: &AnalysisTree, handle: NodeHandle, force_number: bool) {
        let node = tree.node(handle).expect("validated move");
        let parent = tree
            .node(node.parent().expect("root cannot be a move"))
            .expect("validated parent");
        let mut fields = parent.fen().split_ascii_whitespace();
        fields.next();
        let white = fields.next() == Some("w");
        let fullmove = parent
            .fen()
            .split_ascii_whitespace()
            .nth(5)
            .expect("validated six-field FEN");
        self.separator();
        if white || force_number {
            let suffix = if white { "." } else { "..." };
            self.plain(&format!("{fullmove}{suffix} "));
        }

        let movement = node.movement().expect("nonroot SAN move");
        let mut label = movement.san.clone();
        for &nag in node.nags() {
            if let Some(glyph) = annotation(nag) {
                label.push_str(glyph);
            }
        }
        self.push(&label, RunOrigin::TreeMove, Some(handle));

        // Kindle attaches recognized punctuation NAGs to the move chip;
        // unrecognized numeric NAGs follow it in their authored order.
        for &nag in node.nags() {
            if annotation(nag).is_none() {
                self.separator();
                let mut numeric = String::from("$");
                numeric.push_str(&nag.to_string());
                self.plain(&numeric);
            }
        }
        if !node.content().is_empty() {
            self.content(node.content(), tree);
        }
    }
}

fn annotation(nag: u64) -> Option<&'static str> {
    match nag {
        1 => Some("!"),
        2 => Some("?"),
        3 => Some("!!"),
        4 => Some("??"),
        5 => Some("!?"),
        6 => Some("?!"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::parse_puzzle_file;
    use serde_json::{json, Value};

    const RICH: &[u8] =
        include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json");
    const BOOK: &[u8] =
        include_bytes!("../tests/fixtures/kindle-parity/pgn-converter/valid-book.json");

    fn parse(value: Value) -> Puzzle {
        parse_puzzle_file(&serde_json::to_vec(&value).unwrap())
            .unwrap()
            .puzzles
            .remove(0)
    }

    fn rich() -> Puzzle {
        parse_puzzle_file(RICH).unwrap().puzzles.remove(0)
    }

    fn book(index: usize) -> Puzzle {
        parse_puzzle_file(BOOK)
            .unwrap()
            .puzzles
            .into_iter()
            .nth(index)
            .unwrap()
    }

    fn linked(document: &AnalysisDocument, tree: &AnalysisTree) -> Vec<(RunOrigin, String, String)> {
        document
            .runs
            .iter()
            .filter_map(|run| {
                run.target.map(|handle| {
                    (
                        run.origin,
                        tree.node(handle).unwrap().id().to_owned(),
                        run.text.clone(),
                    )
                })
            })
            .collect()
    }

    #[test]
    fn rich_promotion_golden_runs_preserve_preface_root_and_link_origins() {
        let puzzle = rich();
        let tree = puzzle.analysis.as_ref().unwrap();
        let document = project_analysis(&puzzle);
        assert_eq!(
            document.text(),
            "Choose 1...h1=Q+ or inspect h1=N. The text h1=N is deliberately plain.\n\
             Main idea: h1=Q+.\n\
             1... h1=Q+! The queen promotion wins immediately. \
             (1... h1=N An explicitly marked alternative.) (1... h1=R+)"
        );
        assert_eq!(
            linked(&document, tree),
            [
                (RunOrigin::ExplicitMoveRef, "n1".into(), "1...h1=Q+".into()),
                (RunOrigin::ExplicitMoveRef, "n2".into(), "h1=N".into()),
                (RunOrigin::ExplicitMoveRef, "n1".into(), "h1=Q+".into()),
                (RunOrigin::TreeMove, "n1".into(), "h1=Q+!".into()),
                (RunOrigin::TreeMove, "n2".into(), "h1=N".into()),
                (RunOrigin::TreeMove, "n3".into(), "h1=R+".into()),
            ]
        );
        assert!(document
            .runs
            .iter()
            .filter(|run| run.origin == RunOrigin::Prose)
            .all(|run| run.target.is_none()));
    }

    #[test]
    fn book_nested_ravs_and_adjacent_white_black_moves_have_golden_order() {
        let puzzle = book(0);
        let tree = puzzle.analysis.as_ref().unwrap();
        let document = project_analysis(&puzzle);
        assert_eq!(
            document.text(),
            "Compare the main knight with c5 and plain Nd5/e2e4.\n\
             1. e4! Central move. Compare 1...e5 with e2e4 in plain text. \
             1... e5 (1... c5 Sicilian alternative. 2. Nf3 Nc6 \
             (2... d6 Nested sideline.)) 2. Nf3 Nc6"
        );
        assert_eq!(
            linked(&document, tree)
                .into_iter()
                .filter(|(origin, _, _)| *origin == RunOrigin::TreeMove)
                .map(|(_, id, _)| id)
                .collect::<Vec<_>>(),
            ["n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8"]
        );
        assert_eq!(document.text().matches('(').count(), 2);
        assert!(!document.text().contains(['{', '}']));
    }

    #[test]
    fn black_start_fullmove_uses_parent_fen_and_numbering_resumes_after_comments() {
        let mut value: Value = serde_json::from_slice(BOOK).unwrap();
        let puzzle = &mut value["puzzles"][1];
        for node in puzzle["analysis"]["nodes"].as_array_mut().unwrap() {
            let fields = node["fen"].as_str().unwrap();
            let mut parts = fields.split_ascii_whitespace().collect::<Vec<_>>();
            let fullmove = parts[5].parse::<u32>().unwrap() + 36;
            let updated = format!("{} {}", parts[..5].join(" "), fullmove);
            node["fen"] = json!(updated);
        }
        puzzle["fen"] = puzzle["analysis"]["nodes"][0]["fen"].clone();
        puzzle["analysis"]["nodes"][0]["comment"] = json!("Root prose.");
        let collection = parse_puzzle_file(&serde_json::to_vec(&value).unwrap()).unwrap();
        let puzzle = &collection.puzzles[1];
        assert_eq!(
            project_analysis(puzzle).text(),
            "Promote with 1...h1=Q+ or inspect h1=N.\n\
             Root prose.\n\
             37... h1=Q+ Promotion comment with plain h1=Q+. \
             (37... h1=N Underpromotion sideline.) 38. Ka2"
        );
    }

    #[test]
    fn siblings_follow_order_not_role_and_identical_labels_keep_distinct_targets() {
        let mut value: Value = serde_json::from_slice(RICH).unwrap();
        let puzzle = &mut value["puzzles"][0];
        puzzle["analysis"]["nodes"][0]["children"] = json!(["n2", "n3", "n1"]);
        puzzle["description_content"] = json!([
            {"type":"move_ref","node":"n1","label":"same"},
            {"type":"text","text":" then "},
            {"type":"move_ref","node":"n2","label":"same"}
        ]);
        let puzzle = parse(value);
        let tree = puzzle.analysis.as_ref().unwrap();
        let document = project_analysis(&puzzle);
        assert!(document.text().contains(
            "1... h1=N An explicitly marked alternative. \
             (1... h1=R+) (1... h1=Q+! The queen promotion wins immediately.)"
        ));
        assert_eq!(
            linked(&document, tree)[..2],
            [
                (RunOrigin::ExplicitMoveRef, "n1".into(), "same".into()),
                (RunOrigin::ExplicitMoveRef, "n2".into(), "same".into()),
            ]
        );
    }

    #[test]
    fn nags_keep_symbolic_and_numeric_order_and_do_not_turn_into_links() {
        let mut value: Value = serde_json::from_slice(RICH).unwrap();
        value["puzzles"][0]["analysis"]["nodes"][1]["nags"] = json!([1, 7, 2, 0, 3, 42, 4, 5, 6]);
        let puzzle = parse(value);
        let document = project_analysis(&puzzle);
        let move_run = document.runs.iter().find(|run| run.origin == RunOrigin::TreeMove).unwrap();
        assert_eq!(move_run.text, format!("h1=Q+{}", ["!", "?", "!!", "??", "!?", "?!"].concat()));
        assert!(document.text().contains("$7 $0 $42 The queen promotion"));
        assert!(document.runs.iter().all(|run| {
            run.origin != RunOrigin::Prose || run.target.is_none()
        }));
    }

    #[test]
    fn authored_unicode_braces_newlines_and_long_words_are_lossless() {
        let mut value: Value = serde_json::from_slice(RICH).unwrap();
        let long_word = "СловоÉchec♞".repeat(150);
        let content = format!("Русский 中文 العربية\n{{author braces}} {long_word}\nNd5 e2e4");
        value["puzzles"][0]["analysis"]["nodes"][1]["content"] =
            json!([{"type":"text","text":content}]);
        let puzzle = parse(value);
        let text = project_analysis(&puzzle).text();
        assert!(text.contains(&content));
        assert_eq!(text.matches(&long_word).count(), 1);
        assert!(text.contains("{author braces}"));
        let tree = puzzle.analysis.as_ref().unwrap();
        let runs = linked(&project_analysis(&puzzle), tree);
        assert_eq!(runs.iter().filter(|(origin, _, _)| *origin == RunOrigin::TreeMove).count(), 3);
    }

    #[test]
    fn legacy_and_empty_descriptions_do_not_create_tree_moves() {
        let base = json!({"version":1,"puzzles":[{
            "id":"legacy", "fen":"7k/8/5KQ1/8/8/8/8/8 w - - 0 1",
            "solution":["g6g7"], "description":"Plain Nd5\nMore text"
        }]});
        let legacy = parse(base.clone());
        let document = project_analysis(&legacy);
        assert_eq!(document.text(), "Plain Nd5\nMore text\n");
        assert!(document.runs.iter().all(|run| run.target.is_none()));
        let mut empty = base;
        empty["puzzles"][0]["description_content"] = json!([]);
        assert!(project_analysis(&parse(empty)).runs.is_empty());
        let mut no_description = serde_json::from_slice::<Value>(RICH).unwrap();
        no_description["puzzles"][0]["description_content"] = json!([]);
        no_description["puzzles"][0]["analysis"]["nodes"][0]["content"] = json!([]);
        let puzzle = parse(no_description);
        assert!(project_analysis(&puzzle).text().starts_with("1... h1=Q+!"));
    }

    #[test]
    fn deep_and_wide_trees_project_iteratively_with_every_move_once() {
        let fen = "7k/8/5KQ1/8/8/8/8/8 w - - 0 1";
        let depth = 3000;
        let mut nodes = Vec::with_capacity(depth + 1);
        nodes.push(json!({"id":"n0","fen":fen,"children":["n1"]}));
        for i in 1..=depth {
            let id = format!("n{i}");
            let parent = format!("n{}", i - 1);
            let children = if i < depth { vec![format!("n{}", i + 1)] } else { Vec::new() };
            nodes.push(json!({
                "id":id, "parent":parent, "fen":fen, "children":children,
                "move":{"uci":"g6g7","san":"Qg7#"}, "role":"main"
            }));
        }
        let puzzle = parse(json!({"version":1,"puzzles":[{
            "id":"deep","fen":fen,"solution":vec!["g6g7"; depth],
            "analysis":{"version":1,"root":"n0","nodes":nodes}
        }]}));
        let document = project_analysis(&puzzle);
        assert_eq!(document.runs.iter().filter(|r| r.origin == RunOrigin::TreeMove).count(), depth);
        assert_eq!(document.text().matches("Qg7#").count(), depth);

        let mut wide: Value = serde_json::from_slice(RICH).unwrap();
        let tree = &mut wide["puzzles"][0]["analysis"];
        for i in 0..600 {
            let mut node = tree["nodes"][3].clone();
            let id = format!("wide{i}");
            node["id"] = json!(id);
            tree["nodes"].as_array_mut().unwrap().push(node);
            tree["nodes"][0]["children"].as_array_mut().unwrap().push(json!(format!("wide{i}")));
        }
        let puzzle = parse(wide);
        let document = project_analysis(&puzzle);
        assert_eq!(document.runs.iter().filter(|r| r.origin == RunOrigin::TreeMove).count(), 603);
        assert_eq!(document.text().matches('(').count(), 602);
        assert_eq!(document.text().matches(')').count(), 602);
    }
}
