//! Lossless, bounded text pages for the normal Kobo board's reading area.
//! Cobalt silently caps a paragraph at 16 links and 256 style spans.

use crate::analysis::NodeHandle;
use crate::pgn_projection::{AnalysisDocument, RunOrigin};

// Intentionally conservative: the full-size chessboard is never resized.
pub const PAGE_CHARS: usize = 32;
pub const PAGE_NEWLINES: usize = 1;
pub const PAGE_LINKS: usize = 3;
/// Bound unbreakable words without inserting invisible characters into prose.
pub const PAGE_WORD_CHARS: usize = 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageFragment {
    pub source_run: usize,
    pub text: String,
    pub origin: RunOrigin,
    /// A very long link label is clickable on its first fragment only.
    pub target: Option<NodeHandle>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnalysisPage {
    pub fragments: Vec<PageFragment>,
}

impl AnalysisPage {
    pub fn text(&self) -> String {
        self.fragments
            .iter()
            .map(|fragment| fragment.text.as_str())
            .collect()
    }

    pub fn link_count(&self) -> usize {
        self.fragments
            .iter()
            .filter(|fragment| fragment.target.is_some())
            .count()
    }
}

/// Stable link identity across pagination/reflow, distinct for duplicate SANs.
pub fn link_action(run: usize) -> String {
    format!("analysis-link-{run}")
}

/// No silent truncation, byte-splitting or repeated semantic links. A page
/// contains one rich paragraph with at most PAGE_LINKS style spans.
pub fn paginate(document: &AnalysisDocument) -> Vec<AnalysisPage> {
    let mut pages = Vec::new();
    let mut page = AnalysisPage::default();
    let mut used = 0;
    let mut newlines = 0;
    let mut links = 0;
    let mut unbroken = 0;

    for (source_run, run) in document.runs.iter().enumerate() {
        let mut remaining = run.text.as_str();
        let mut target = run.target;
        while !remaining.is_empty() {
            let front: String = remaining.chars().take(PAGE_CHARS + 1).collect();
            let fits_whole = front.chars().count() <= PAGE_CHARS && front.len() == remaining.len();
            let extra_newlines = front.chars().filter(|ch| *ch == '\n').count();
            if used == PAGE_CHARS
                || (target.is_some() && links == PAGE_LINKS)
                || (target.is_some()
                    && fits_whole
                    && (front.chars().count() > PAGE_CHARS - used
                        || newlines + extra_newlines > PAGE_NEWLINES))
            {
                if !page.fragments.is_empty() {
                    pages.push(std::mem::take(&mut page));
                }
                used = 0;
                newlines = 0;
                links = 0;
                unbroken = 0;
            }

            let mut end = 0;
            let mut count = 0;
            let mut breaks = 0;
            let mut word = unbroken;
            for (offset, ch) in remaining.char_indices() {
                if used + count == PAGE_CHARS
                    || (ch == '\n' && newlines + breaks == PAGE_NEWLINES)
                    || (!ch.is_whitespace() && word == PAGE_WORD_CHARS)
                {
                    break;
                }
                count += 1;
                if ch == '\n' {
                    breaks += 1;
                }
                if ch.is_whitespace() {
                    word = 0;
                } else {
                    word += 1;
                }
                end = offset + ch.len_utf8();
            }
            if end == 0 {
                // A second newline cannot fit in this page.
                debug_assert!(!page.fragments.is_empty());
                pages.push(std::mem::take(&mut page));
                used = 0;
                newlines = 0;
                links = 0;
                unbroken = 0;
                continue;
            }

            page.fragments.push(PageFragment {
                source_run,
                text: remaining[..end].to_owned(),
                origin: run.origin,
                target,
            });
            if target.is_some() {
                links += 1;
            }
            used += count;
            newlines += breaks;
            unbroken = word;
            target = None;
            remaining = &remaining[end..];
            if !remaining.is_empty() {
                pages.push(std::mem::take(&mut page));
                used = 0;
                newlines = 0;
                links = 0;
                unbroken = 0;
            }
        }
    }
    if !page.fragments.is_empty() || pages.is_empty() {
        pages.push(page);
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pgn_projection::{project_analysis, AnalysisRun};
    use crate::puzzle::parse_puzzle_file;

    fn rich() -> crate::puzzle::Puzzle {
        parse_puzzle_file(include_bytes!(
            "../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json"
        ))
        .unwrap()
        .puzzles
        .remove(0)
    }

    fn assert_lossless(document: &AnalysisDocument) {
        let pages = paginate(document);
        assert!(!pages.is_empty());
        assert_eq!(
            pages.iter().map(AnalysisPage::text).collect::<String>(),
            document.text()
        );
        assert!(pages.iter().all(|page| page.link_count() <= PAGE_LINKS));
        assert!(pages.iter().all(|page| page
            .fragments
            .iter()
            .map(|f| f.text.chars().count())
            .sum::<usize>()
            <= PAGE_CHARS));
        assert!(pages
            .iter()
            .all(|page| page.text().matches('\n').count() <= PAGE_NEWLINES));
        assert!(pages.iter().all(|page| page
            .text()
            .split(char::is_whitespace)
            .all(|word| word.chars().count() <= PAGE_WORD_CHARS)));
        let actual: Vec<_> = pages
            .iter()
            .flat_map(|page| page.fragments.iter())
            .filter(|fragment| fragment.target.is_some())
            .map(|fragment| (fragment.source_run, fragment.target))
            .collect();
        let expected: Vec<_> = document
            .runs
            .iter()
            .enumerate()
            .filter(|(_, run)| run.target.is_some())
            .map(|(index, run)| (index, run.target))
            .collect();
        assert_eq!(actual, expected, "every semantic move/ref exactly once");
    }

    #[test]
    fn rich_preface_variations_and_links_survive_every_page() {
        assert_lossless(&project_analysis(&rich()));
    }

    #[test]
    fn dense_references_above_cobalt_limits_do_not_truncate() {
        let puzzle = rich();
        let tree = puzzle.analysis.as_ref().unwrap();
        let target = tree.node(tree.root()).unwrap().children()[0];
        let mut runs = vec![AnalysisRun {
            text: "Черные начинают — ".into(),
            origin: RunOrigin::Prose,
            target: None,
        }];
        for i in 0..330 {
            runs.push(AnalysisRun {
                text: format!("e4{i}"),
                origin: RunOrigin::ExplicitMoveRef,
                target: Some(target),
            });
            runs.push(AnalysisRun {
                text: " ".into(),
                origin: RunOrigin::Prose,
                target: None,
            });
        }
        let document = AnalysisDocument { runs };
        assert_lossless(&document);
        assert!(paginate(&document).len() > 16);
    }

    #[test]
    fn long_cyrillic_words_and_multibyte_labels_keep_the_last_page() {
        let puzzle = rich();
        let tree = puzzle.analysis.as_ref().unwrap();
        let target = tree.node(tree.root()).unwrap().children()[0];
        let document = AnalysisDocument {
            runs: vec![
                AnalysisRun {
                    text: "Я".repeat(900),
                    origin: RunOrigin::Prose,
                    target: None,
                },
                AnalysisRun {
                    text: "\n\nПривет ♞\n".into(),
                    origin: RunOrigin::Prose,
                    target: None,
                },
                AnalysisRun {
                    text: "длинная_ссылка_♞".repeat(30),
                    origin: RunOrigin::ExplicitMoveRef,
                    target: Some(target),
                },
                AnalysisRun {
                    text: " конец!".into(),
                    origin: RunOrigin::Prose,
                    target: None,
                },
            ],
        };
        assert_lossless(&document);
        assert!(paginate(&document)
            .last()
            .unwrap()
            .text()
            .ends_with("конец!"));
    }

    #[test]
    fn empty_tree_still_has_a_first_page() {
        let pages = paginate(&AnalysisDocument::default());
        assert_eq!(pages.len(), 1);
        assert!(pages[0].fragments.is_empty());
    }
}
