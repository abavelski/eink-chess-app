//! Read-only analysis browsing, independent of the live solving board and progress.
//!
//! This state stores a preview built from the author's validated post-move FEN.
//! It never applies moves, grades alternatives or writes puzzle/progress data.

use crate::analysis::{AnalysisTree, NodeHandle};
use crate::board::Board;
use crate::presentation::analysis_text;
use crate::puzzle::Puzzle;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreviewPosition {
    node: NodeHandle,
    fen: String,
    board: Board,
}

/// Owns only transient reader state. Closing/discarding it restores the live
/// board automatically, since the caller never replaces the solving board.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnalysisBrowser {
    open: bool,
    // Advances every time analysis opens, invalidating stale Cobalt link IDs.
    epoch: u64,
    page: usize,
    preview: Option<PreviewPosition>,
}

impl AnalysisBrowser {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn selected_node(&self) -> Option<NodeHandle> {
        self.preview.as_ref().map(|preview| preview.node)
    }

    pub fn preview_fen(&self) -> Option<&str> {
        self.preview.as_ref().map(|preview| preview.fen.as_str())
    }

    pub fn preview_board(&self) -> Option<&Board> {
        self.preview.as_ref().map(|preview| &preview.board)
    }

    /// Open at the preface, showing the untouched live board until a node is
    /// explicitly selected. A legacy description can open without a tree.
    pub fn open(&mut self, puzzle: &Puzzle) -> bool {
        if self.open || !analysis_text(puzzle).available() {
            return false;
        }
        self.open = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.page = 0;
        self.preview = None;
        true
    }

    /// Clear all reader state before any operation that changes the live board.
    pub fn close(&mut self) -> bool {
        let was_open = self.open;
        self.open = false;
        self.page = 0;
        self.preview = None;
        was_open
    }

    pub fn select_root(&mut self, tree: Option<&AnalysisTree>) -> bool {
        let Some(tree) = tree else {
            return false;
        };
        self.select(Some(tree), tree.root())
    }

    pub fn select_id(&mut self, tree: Option<&AnalysisTree>, id: &str) -> bool {
        let Some((tree, node)) = tree.and_then(|tree| tree.by_id(id).map(|node| (tree, node)))
        else {
            return false;
        };
        self.select(Some(tree), node)
    }

    /// Select only an existing node. The parser has already validated the
    /// stored FEN, and this conversion never replays or checks chess moves.
    pub fn select(&mut self, tree: Option<&AnalysisTree>, node: NodeHandle) -> bool {
        if !self.open || self.selected_node() == Some(node) {
            return false;
        }
        let Some(target) = tree.and_then(|tree| tree.node(node)) else {
            return false;
        };
        let board = Board::from_fen(target.fen()).expect("validated analysis node FEN");
        self.preview = Some(PreviewPosition {
            node,
            fen: target.fen().to_owned(),
            board,
        });
        true
    }

    /// The reader layout (Task 06) supplies its actual page count.
    /// Zero pages and requests outside the finite range never change state.
    pub fn set_page(&mut self, page: usize, page_count: usize) -> bool {
        if !self.open || page >= page_count || page == self.page {
            return false;
        }
        self.page = page;
        true
    }

    pub fn turn_page(&mut self, forward: bool, page_count: usize) -> bool {
        if !self.open {
            return false;
        }
        let next = if forward {
            self.page.checked_add(1)
        } else {
            self.page.checked_sub(1)
        };
        next.is_some_and(|page| self.set_page(page, page_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::parse_puzzle_file;

    const RICH: &[u8] =
        include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json");

    fn rich() -> Puzzle {
        parse_puzzle_file(RICH).unwrap().puzzles.remove(0)
    }

    fn legacy(contents: &str) -> Puzzle {
        parse_puzzle_file(contents.as_bytes())
            .unwrap()
            .puzzles
            .remove(0)
    }

    #[test]
    fn reader_transitions_validate_nodes_and_respect_page_boundaries() {
        let puzzle = rich();
        let tree = puzzle.analysis.as_ref().unwrap();
        let mut browser = AnalysisBrowser::default();

        assert!(!browser.is_open());
        assert!(!browser.select_root(Some(tree)));
        assert!(!browser.select_id(Some(tree), "n2"));
        assert!(!browser.turn_page(true, 3));
        assert!(browser.open(&puzzle));
        assert!(!browser.open(&puzzle));
        assert!(browser.is_open());
        assert_eq!(browser.page(), 0);
        assert_eq!(browser.selected_node(), None);
        assert!(browser.preview_board().is_none());

        assert!(!browser.select_id(Some(tree), "does-not-exist"));
        assert!(!browser.select_id(None, "n2"));
        assert_eq!(browser.selected_node(), None);
        assert!(browser.select_id(Some(tree), "n2"));
        let alternative = tree.node(tree.by_id("n2").unwrap()).unwrap();
        assert_eq!(browser.selected_node(), tree.by_id("n2"));
        assert_eq!(browser.preview_fen(), Some(alternative.fen()));
        assert_eq!(
            browser.preview_board(),
            Some(&Board::from_fen(alternative.fen()).unwrap())
        );
        assert!(!browser.select_id(Some(tree), "n2"));

        assert!(browser.set_page(1, 3));
        assert!(browser.turn_page(true, 3));
        assert_eq!(browser.page(), 2);
        assert!(!browser.turn_page(true, 3));
        assert!(!browser.set_page(3, 3));
        assert!(!browser.set_page(0, 0));
        assert!(browser.turn_page(false, 3));
        assert!(browser.turn_page(false, 3));
        assert!(!browser.turn_page(false, 3));
        assert_eq!(browser.page(), 0);

        assert!(browser.select_root(Some(tree)));
        assert_eq!(browser.preview_fen(), Some(puzzle.fen.as_str()));
        assert_eq!(
            browser.preview_board(),
            Some(&Board::from_fen(&puzzle.fen).unwrap())
        );
        let epoch = browser.epoch();
        assert!(browser.close());
        assert!(!browser.close());
        assert_eq!(browser.epoch(), epoch);
        assert!(browser.open(&puzzle));
        assert_ne!(browser.epoch(), epoch);
        assert!(browser.close());
        assert_eq!(browser.page(), 0);
        assert_eq!(browser.selected_node(), None);
        assert!(browser.preview_board().is_none());
        assert!(!browser.set_page(1, 3));
    }

    #[test]
    fn legacy_descriptions_open_without_inventing_tree_positions() {
        let with_description = legacy(
            r#"{"version":1,"puzzles":[{"id":"legacy","fen":"7k/8/5KQ1/8/8/8/8/8 w - - 0 1","description":"Read this","solution":["g6g7"]}]}"#,
        );
        let without_description = legacy(
            r#"{"version":1,"puzzles":[{"id":"empty","fen":"7k/8/5KQ1/8/8/8/8/8 w - - 0 1","solution":["g6g7"]}]}"#,
        );
        let mut browser = AnalysisBrowser::default();
        assert!(!browser.open(&without_description));
        assert!(browser.open(&with_description));
        assert!(!browser.select_id(None, "invented"));
        assert!(!browser.select_root(None));
        assert_eq!(browser.selected_node(), None);
        assert_eq!(browser.preview_fen(), None);
        assert_eq!(browser.preview_board(), None);
        assert!(browser.close());
        assert!(!browser.open(&without_description));
    }
}
