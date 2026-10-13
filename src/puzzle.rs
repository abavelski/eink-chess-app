//! Versioned puzzle collections. The FEN active color is the side solving the puzzle.

use crate::analysis::{parse_content, AnalysisTree, ContentSpan};
use crate::board::{Board, Color};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashSet;

pub const MAX_PUZZLE_FILE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Difficulty {
    Text(String),
    Number(serde_json::Number),
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(text) => formatter.write_str(text),
            Self::Number(number) => write!(formatter, "{number}"),
        }
    }
}

/// Capture explicit null instead of conflating it with a missing field.
#[derive(Debug, Default)]
struct OptionalValue(Option<Value>);

impl<'de> Deserialize<'de> for OptionalValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(Some(Value::deserialize(deserializer)?)))
    }
}

#[derive(Debug, Deserialize)]
struct RawPuzzleCollection {
    version: u32,
    #[serde(default)]
    title: Option<String>,
    puzzles: Vec<RawPuzzle>,
}

#[derive(Debug, Deserialize)]
struct RawPuzzle {
    id: String,
    fen: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    topic: Value,
    #[serde(default)]
    description_content: OptionalValue,
    #[serde(default)]
    analysis: OptionalValue,
    #[serde(default)]
    difficulty: Option<Difficulty>,
    solution: Vec<String>,
}

#[derive(Debug)]
pub struct PuzzleCollection {
    pub title: Option<String>,
    pub puzzles: Vec<Puzzle>,
}

#[derive(Debug)]
pub struct Puzzle {
    pub id: String,
    pub fen: String,
    pub description: Option<String>,
    /// Optional presentation metadata, preserved verbatim (including Unicode and newlines).
    pub topic: Option<String>,
    /// Resolved explicit spans, or a plain-text projection of description when absent.
    pub description_content: Vec<ContentSpan>,
    pub analysis: Option<AnalysisTree>,
    pub difficulty: Option<Difficulty>,
    /// Alternating player and opponent moves, starting with the player's move.
    /// This remains the ONLY graded line, regardless of rich analysis branches.
    pub solution: Vec<String>,
}

impl Puzzle {
    pub fn side_to_move(&self) -> Color {
        match self.fen.split_ascii_whitespace().nth(1) {
            Some("b") => Color::Black,
            _ => Color::White,
        }
    }
}

pub fn parse_puzzle_file(contents: &[u8]) -> Result<PuzzleCollection, String> {
    if contents.len() > MAX_PUZZLE_FILE_BYTES {
        return Err("Puzzle file must be at most 8 MiB.".into());
    }
    let file: RawPuzzleCollection = serde_json::from_slice(contents)
        .map_err(|error| format!("Invalid puzzle file: {error}"))?;
    if file.version != 1 {
        return Err(format!(
            "Unsupported puzzle file version {}; expected 1.",
            file.version
        ));
    }
    if file.puzzles.is_empty() {
        return Err("No puzzles found. Add at least one puzzle to the file.".into());
    }
    let title = file
        .title
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty());

    let mut ids = HashSet::new();
    let mut puzzles = Vec::with_capacity(file.puzzles.len());
    for (index, raw) in file.puzzles.into_iter().enumerate() {
        let name = format!("Puzzle {} ({})", index + 1, raw.id);
        if raw.id.trim().is_empty() || !ids.insert(raw.id.clone()) {
            return Err(format!("{name}: id must be nonempty and unique."));
        }
        Board::from_fen(&raw.fen).map_err(|error| format!("{name}: {error}"))?;
        if raw.solution.is_empty() {
            return Err(format!(
                "{name}: solution must contain at least one UCI move."
            ));
        }
        for (move_index, movement) in raw.solution.iter().enumerate() {
            if !is_uci_move(movement) {
                return Err(format!(
                    "{name}: solution move {} must use UCI notation (for example e2e4 or a7a8q).",
                    move_index + 1
                ));
            }
        }

        let analysis = raw
            .analysis
            .0
            .as_ref()
            .map(|value| AnalysisTree::parse(value, &raw.fen, &raw.solution))
            .transpose()
            .map_err(|error| format!("{name}: {error}"))?;
        let description_content = parse_content(
            raw.description_content.0.as_ref(),
            raw.description.as_deref().unwrap_or(""),
            analysis.as_ref(),
            "description_content",
        )
        .map_err(|error| format!("{name}: {error}"))?;

        puzzles.push(Puzzle {
            id: raw.id,
            fen: raw.fen,
            description: raw.description,
            topic: raw.topic.as_str().map(str::to_owned),
            description_content,
            analysis,
            difficulty: raw.difficulty,
            solution: raw.solution,
        });
    }
    Ok(PuzzleCollection { title, puzzles })
}

pub(crate) fn is_uci_move(movement: &str) -> bool {
    let bytes = movement.as_bytes();
    (bytes.len() == 4 || bytes.len() == 5)
        && matches!(bytes[0], b'a'..=b'h')
        && matches!(bytes[1], b'1'..=b'8')
        && matches!(bytes[2], b'a'..=b'h')
        && matches!(bytes[3], b'1'..=b'8')
        && bytes[..2] != bytes[2..4]
        && (bytes.len() == 4 || matches!(bytes[4], b'q' | b'r' | b'b' | b'n'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> serde_json::Value {
        json!({"version": 1, "puzzles": [{
            "id": "sample", "fen": "7k/8/5KQ1/8/8/8/8/8 w - - 0 1",
            "solution": ["g6g7"]
        }]})
    }

    fn parse(value: serde_json::Value) -> Result<PuzzleCollection, String> {
        parse_puzzle_file(&serde_json::to_vec(&value).unwrap())
    }

    #[test]
    fn accepts_collection_title_optional_description_and_derives_turn_from_fen() {
        let collection = parse(sample()).unwrap();
        assert!(collection.title.is_none());
        assert!(collection.puzzles[0].description.is_none());
        assert!(collection.puzzles[0].difficulty.is_none());
        assert_eq!(collection.puzzles[0].side_to_move(), Color::White);

        let mut value = sample();
        value["title"] = json!("  Endgames  ");
        value["puzzles"][0]["fen"] = json!("8/8/8/8/8/5kq1/8/7K b - - 0 1");
        value["puzzles"][0]["description"] = json!("Find a mate in one.");
        value["puzzles"][0]["difficulty"] = json!(4);
        value["puzzles"][0]["solution"] = json!(["g3g2"]);
        let collection = parse(value).unwrap();
        assert_eq!(collection.title.as_deref(), Some("Endgames"));
        assert_eq!(
            collection.puzzles[0]
                .difficulty
                .as_ref()
                .unwrap()
                .to_string(),
            "4"
        );
        assert_eq!(collection.puzzles[0].side_to_move(), Color::Black);
        assert_eq!(
            collection.puzzles[0].description.as_deref(),
            Some("Find a mate in one.")
        );

        let mut value = sample();
        value["title"] = json!("   ");
        assert!(parse(value).unwrap().title.is_none());
    }

    #[test]
    fn difficulty_accepts_text_or_numbers_and_displays_without_quotes() {
        for (difficulty, expected) in [
            (json!("Hard"), Some("Hard")),
            (json!("3 stars"), Some("3 stars")),
            (json!(4), Some("4")),
            (json!(2.5), Some("2.5")),
            (json!(-1), Some("-1")),
            (json!(null), None),
        ] {
            let mut value = sample();
            value["puzzles"][0]["difficulty"] = difficulty;
            let collection = parse(value).unwrap();
            assert_eq!(
                collection.puzzles[0]
                    .difficulty
                    .as_ref()
                    .map(ToString::to_string)
                    .as_deref(),
                expected
            );
        }
    }

    #[test]
    fn rejects_malformed_empty_and_future_files() {
        assert!(parse_puzzle_file(b"not json").is_err());
        assert!(parse(json!({"version": 1, "puzzles": []})).is_err());
        let mut value = sample();
        value["version"] = json!(2);
        assert!(parse(value).unwrap_err().contains("version 2"));
        assert!(parse_puzzle_file(&vec![b' '; MAX_PUZZLE_FILE_BYTES + 1]).is_err());
    }

    #[test]
    fn rejects_invalid_positions_ids_and_solutions() {
        let mut value = sample();
        value["puzzles"][0]["fen"] = json!("invalid");
        assert!(parse(value).unwrap_err().contains("FEN"));
        let mut value = sample();
        value["puzzles"][0]["id"] = json!(" ");
        assert!(parse(value).is_err());
        let mut value = sample();
        let duplicate = value["puzzles"][0].clone();
        value["puzzles"].as_array_mut().unwrap().push(duplicate);
        assert!(parse(value).is_err());
        for solution in [
            json!([]),
            json!(["Qg7#"]),
            json!(["e2e2"]),
            json!(["a7a8k"]),
        ] {
            let mut value = sample();
            value["puzzles"][0]["solution"] = solution;
            assert!(parse(value).is_err());
        }
        assert!(is_uci_move("a7a8q"));
        assert!(is_uci_move("e1g1"));
    }
    fn rich() -> serde_json::Value {
        serde_json::from_slice(include_bytes!(
            "../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json"
        ))
        .unwrap()
    }

    fn rejected(mutate: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value = rich();
        mutate(&mut value);
        parse(value).unwrap_err()
    }

    #[test]
    fn pinned_corpus_accepts_valid_books_and_rejects_all_invalid_rich_fixtures() {
        for (name, data) in [
            ("rich", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json")[..]),
            ("book", &include_bytes!("../tests/fixtures/kindle-parity/pgn-converter/valid-book.json")[..]),
            ("legacy", &include_bytes!("../tests/fixtures/kindle-parity/puzzles.json")[..]),
        ] {
            assert!(parse_puzzle_file(data).is_ok(), "{name}");
        }
        for (name, data) in [
            ("cycle", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-cycle.json")[..]),
            ("disconnected", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-disconnected.json")[..]),
            ("duplicate-id", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-duplicate-id.json")[..]),
            ("main-mismatch", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-main-solution-mismatch.json")[..]),
            ("missing-child", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-missing-child.json")[..]),
            ("dangling-ref", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-move-ref-dangling.json")[..]),
            ("root-ref", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-move-ref-root.json")[..]),
            ("root-fen", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-root-fen-mismatch.json")[..]),
            ("bad-label", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-span-bad-label.json")[..]),
            ("missing-text", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-span-missing-text.json")[..]),
            ("span-type", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-span-unknown-type.json")[..]),
            ("uci", &include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-uci.json")[..]),
        ] {
            let error = parse_puzzle_file(data).expect_err(name);
            assert!(error.contains("rich-black-promotion"), "{name}: {error}");
        }
    }

    #[test]
    fn indexed_tree_preserves_order_roles_references_and_plain_text() {
        use crate::analysis::{ContentSpan, MoveRole};
        let file = parse(rich()).unwrap();
        let puzzle = &file.puzzles[0];
        assert_eq!(puzzle.solution, ["h2h1q"]);
        let tree = puzzle.analysis.as_ref().unwrap();
        assert_eq!(tree.nodes().len(), 4);
        let root = tree.root();
        assert_eq!(tree.node(root).unwrap().id(), "n0");
        assert_eq!(tree.node(root).unwrap().parent(), None);
        assert_eq!(tree.node(root).unwrap().fen(), puzzle.fen.as_str());
        assert_eq!(tree.node(root).unwrap().children().iter().map(|&h| tree.node(h).unwrap().id()).collect::<Vec<_>>(), ["n1", "n2", "n3"]);
        let main = tree.by_id("n1").unwrap();
        assert_eq!(main.index(), 1);
        assert_eq!(tree.node(main).unwrap().parent(), Some(root));
        assert_eq!(tree.node(main).unwrap().movement().unwrap().san, "h1=Q+");
        assert_eq!(tree.node(main).unwrap().role(), Some(MoveRole::Main));
        assert_eq!(tree.node(main).unwrap().nags(), [1]);
        assert_eq!(tree.node(tree.by_id("n2").unwrap()).unwrap().role(), Some(MoveRole::Alternative));
        assert_eq!(tree.node(tree.by_id("n3").unwrap()).unwrap().role(), Some(MoveRole::Sideline));
        assert_eq!(tree.node(tree.by_id("n3").unwrap()).unwrap().content(), &[]);
        assert_eq!(puzzle.description_content[0], ContentSpan::Text("Choose ".into()));
        assert_eq!(puzzle.description_content[1], ContentSpan::MoveRef {
            node: main,
            label: Some("1...h1=Q+".into()),
        });
        assert_eq!(puzzle.description_content[4], ContentSpan::Text(". The text h1=N is deliberately plain.".into()));
        assert!(tree.node(tree.by_id("n2").unwrap()).unwrap().content().iter().any(|span| matches!(span, ContentSpan::Text(text) if text == "An explicitly marked alternative.")));
    }

    #[test]
    fn topics_fallback_spans_empty_spans_and_no_analysis() {
        use crate::analysis::ContentSpan;
        let mut value = sample();
        value["puzzles"][0]["description"] = json!("Plain SAN Qg7# stays plain.");
        value["puzzles"][0]["topic"] = json!("Промоция\nMate");
        let puzzle = &parse(value.clone()).unwrap().puzzles[0];
        assert_eq!(puzzle.topic.as_deref(), Some("Промоция\nMate"));
        assert!(puzzle.analysis.is_none());
        assert_eq!(puzzle.description_content, [ContentSpan::Text("Plain SAN Qg7# stays plain.".into())]);
        value["puzzles"][0]["description_content"] = json!([]);
        assert!(parse(value.clone()).unwrap().puzzles[0].description_content.is_empty());
        value["puzzles"][0]["topic"] = json!(["not a string"]);
        assert!(parse(value.clone()).unwrap().puzzles[0].topic.is_none());
        value["puzzles"][0]["topic"] = json!(null);
        assert!(parse(value.clone()).unwrap().puzzles[0].topic.is_none());
        value["puzzles"][0]["description_content"] = json!([{"type":"text","text":"e2e4 is still plain"}]);
        assert_eq!(parse(value).unwrap().puzzles[0].description_content, [ContentSpan::Text("e2e4 is still plain".into())]);

        let mut value = rich();
        value["puzzles"][0]["analysis"]["nodes"][0]["content"] = json!([]);
        let file = parse(value).unwrap();
        let tree = file.puzzles[0].analysis.as_ref().unwrap();
        assert!(tree.node(tree.root()).unwrap().content().is_empty());
    }

    #[test]
    fn rich_graph_and_field_mutations_are_rejected() {
        for (name, change) in [
            ("version", ("version", json!(2))),
            ("version-type", ("version", json!("1"))),
            ("root", ("root", json!("not-there"))),
            ("nodes", ("nodes", json!([]))),
        ] {
            let error = rejected(|v| v["puzzles"][0]["analysis"][change.0] = change.1);
            assert!(error.contains("analysis"), "{name}: {error}");
        }

        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["id"] = json!("n0")).contains("duplicate"));
        assert!(rejected(|v| { v["puzzles"][0]["analysis"]["nodes"][1].as_object_mut().unwrap().remove("parent"); }).contains("parent"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["parent"] = json!("lost")).contains("parent"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["children"] = json!(["n1", "n1", "n2", "n3"])).contains("child"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["children"] = json!(["n1", "n2"])).contains("link"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["children"] = json!(["n2"])).contains("parent/child"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["children"] = json!(["n1", "n2", "n3", "missing"])).contains("missing child"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["children"] = json!(["n0", "n1", "n2", "n3"])).contains("root"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["parent"] = json!("n3")).contains("root"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["move"] = json!({"uci":"e2e4","san":"e4"})).contains("root"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["role"] = json!("main")).contains("root"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["nags"] = json!([])).contains("root"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][0]["fen"] = json!("8/8/8/8/8/8/8/8 w - - 0 1")).contains("root FEN"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["fen"] = json!("invalid")).contains("FEN"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["move"]["uci"] = json!("h2h1k")).contains("uci"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["move"]["san"] = json!("")).contains("san"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["move"]["extra"] = json!(1)).contains("exactly"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][2]["role"] = json!("other")).contains("role"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["nags"] = json!([-1])).contains("NAG"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["nags"] = json!([1.5])).contains("NAG"));
        assert!(rejected(|v| v["puzzles"][0]["solution"] = json!(["h2h1n"])).contains("solution"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][2]["role"] = json!("main")).contains("multiple main"));
    }

    #[test]
    fn rich_span_mutations_are_rejected_but_empty_lists_are_authoritative() {
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"text"}])).contains("text"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"text","text":""}])).contains("text"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"text","text":"ok","extra":1}])).contains("exactly"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"move_ref","node":"n2","extra":true}])).contains("keys"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"move_ref","node":"n2","label":""}])).contains("label"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"move_ref","node":"n0"}])).contains("root"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"move_ref","node":"lost"}])).contains("dangling"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"move_ref"}])).contains("keys"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!([{"type":"link","node":"n2"}])).contains("type"));
        assert!(rejected(|v| v["puzzles"][0]["description_content"] = json!(null)).contains("array"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"]["nodes"][1]["content"] = json!([{"type":"move_ref","node":"lost"}])).contains("dangling"));
        assert!(rejected(|v| v["puzzles"][0]["analysis"] = json!(null)).contains("object"));
        let mut value = sample();
        value["puzzles"][0]["description_content"] = json!([{"type":"move_ref","node":"n1"}]);
        assert!(parse(value).unwrap_err().contains("requires an analysis tree"));
    }

    #[test]
    fn raw_byte_boundary_includes_256k_and_8mib() {
        let bytes = serde_json::to_vec(&sample()).unwrap();
        for size in [262_144, 262_145, 8_388_608] {
            let mut padded = bytes.clone();
            padded.resize(size, b' ');
            assert_eq!(padded.len(), size);
            assert!(parse_puzzle_file(&padded).is_ok(), "size {size}");
        }
        let mut oversized = bytes;
        oversized.resize(8_388_609, b' ');
        assert!(parse_puzzle_file(&oversized).unwrap_err().contains("8 MiB"));
    }

}
