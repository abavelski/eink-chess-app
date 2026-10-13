# Kobo puzzle-format and analysis parity plan

**Plan date:** 2026-10-13. **Target:** `abavelski/eink-chess-app`, default branch `master`. **Reference:** `abavelski/kindle-chess-puzzles` `main` at `80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8`.

## Objective and boundaries

Load the **same unmodified Kindle puzzle collection files** on the Kobo Libra H2O, including rich analysis trees, annotated inline move lines, clickable explicit references, exact precomputed post-move FEN previews and puzzle descriptions. Preserve Kobo's existing full 8×8 board, solution grading, progress and direct NickelMenu/return path. This is **puzzle/analysis parity only**, not general Kindle feature parity.

**In scope:** root v1 additive rich analysis contract, larger files, `topic`, `description_content`, comments, SAN, variations, NAGs, `move_ref` hyperlinks, read-only preview, paginated Cobalt presentation, Analysis toolbar control, description display and tests.

**Explicitly out of scope:** game-review data/picker/workspace, small board/sidebar or resizing existing board, on-device PGN importer/converter, chess engine/legality engine, task 07's proposed root-v2 branching `solutions`, configurable settings UI, toolbar preference state, changes to grading alternatives or storage schema. Kindle's offline `tools/pgn_converter.py` remains the **single** authoring path for rich JSON; no duplicate converter is needed on Kobo.

## Observed code-level gap

| Capability | Kindle reference | Current Kobo `master` | Required work |
| --- | --- | --- | --- |
| Base collections | `crates/chess-core/src/puzzle.rs` root v1 | `src/puzzle.rs` root v1 | Preserve |
| Size | 8 MiB parser cap | 256 KiB parser and Cobalt store | Chunked/bounded device loading + parser cap |
| Rich data | `analysis.version=1`, indexed tree, flat nodes | No modeled `analysis` | Strict parser/model |
| Descriptions | Plain + `description_content` spans | Plain `description` | Rich text/fallback with explicit links |
| Topic | Optional Unicode topic shown in solve status | Not parsed/displayed | Add presentation |
| Analysis line | Ordered SAN, NAG, prose, RAVs (plain book prose) | Only UCI `solution` grading | Deterministic inline projection |
| Analysis preview | Selected node's stored FEN; live solve untouched | No preview state | Separate read-only board/browser |
| UI | Analysis open/close, move refs and pages | NOTE/description toggle, no Analysis | Swap toolbar control, paginated pane/taps |
| Solving | Strict legacy `solution` even with sidelines | Strict legacy `solution` | Preserve; never grade sidelines |
| Large book input | Plain `puzzles-*.json` | `context.store().load` per file | Honor same bytes/names despite Cobalt caps |

**Hard Kobo blocker:** pinned `abavelski/Cobalt@ff4ec9950e178243807082bf818e42c8538d080e` defines `kobo-protocol::MAX_STORE_VALUE = 256*1024` and `MAX_FRAME_LEN = 8*1_048_576`. An 8 MiB file cannot fit in a single framed store response. Merely bumping `src/puzzle.rs::MAX_PUZZLE_FILE_BYTES` is insufficient; Task 01 must deliver safe bounded multi-part loading using a narrowly scoped Cobalt adapter. Small existing store records, especially progress, remain unchanged.

**UI opportunity/limit:** pinned `ScreenBuilder::rich_text_linking` supports styled inline runs and clickable byte ranges, but `kobo-ui::MAX_TEXT_LINKS = 16` per rich-text node, `MAX_RICH_TEXT_SPANS = 256`, and `kobo-protocol::MAX_NODES = 512`. Plan deterministic overflow pages that split *without dropping* annotated moves/references. The screen still holds the same full-size board, so no implicit assumption about unlimited vertical space.

## Intended UX

- Keep normal board size and existing Free Board, lock, Reset, Flip actions.
- **Analysis** is visible in the toolbar by default and replaces NOTE; **NOTE is hidden**, no settings/settings toggle is implemented.
- Rich puzzle: open Analysis to read complete description preface, main path and ordered sidelines/alternatives with comment prose and NAGs; page through. Tap actual SAN moves and explicitly linked prose refs to preview the node's **stored FEN**, not a recomputed legal move. Closing restores the exact live solve board.
- Legacy puzzle without tree but with description: Analysis opens a description-only page (no invented variation positions); an empty legacy puzzle keeps a visible but disabled Analysis action. This is a deliberate Kobo accessibility fallback, while rich analysis semantics follow Kindle.
- The current automatic description reveal on successful solve remains functional with NOTE removed. The normal Solution status shows optional topic, while Free Board keeps its status.
- Solving still accepts only `solution`, applying stored opponent replies. Analysis browsing does not grade, modify solve progress or mark solved. Opening/closing, navigation and promotion have explicit tested state transitions.

## Agent work plan and gates

[Task index and dependencies](../tasks/kindle-puzzle-parity/README.md). Parallelizable work: 01 (large-read transport) alongside 00/02; 03/04/05 in parallel after 02. Then 06 layout, 07 actions, 08 integrated host gate, 09 human physical device acceptance.

**Host testing:** `cargo test`, `cargo fmt --all -- --check`, Cobalt simulator diagnostics for Libra H2O, pinned rich-analysis positive/negative fixtures and live-board/progress identity tests. Add workflow coverage at Task 08. A successful host build is **not** physical validation.

**Physical testing:** Execute Task 09 on the Libra H2O *after* all automated gates; verify real touch/e-ink readability, long books, unchanged JSON hashes, correct progress writes, Nickel exit/relaunch and restoration. Record explicit PASS/FAIL/NOT TESTED and obtain owner acceptance.

## Superseded prior proposal

`tasks/07-rich-solutions.md` was a different, unimplemented proposal: root JSON `version: 2` with `solutions` branches that would change accepted solver moves. It conflicts with Kindle's implemented root-v1 `analysis.version: 1` where variants are browse-only. Do **not** implement it; use the tasks linked above. This plan introduces no code or settings changes by itself.
