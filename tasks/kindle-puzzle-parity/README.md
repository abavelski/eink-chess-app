# Kobo ⇄ Kindle puzzle and analysis parity — agent task index

**Status:** Planned (not implemented). **Implementation target:** Kobo `master`.
**Kindle reference:** `abavelski/kindle-chess-puzzles@80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8` (`main`, 2026-10-09).
**Contract:** [Kindle puzzle format](https://github.com/abavelski/kindle-chess-puzzles/blob/80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8/docs/PUZZLE_FORMAT.md).
**Overview:** [docs/KINDLE_PUZZLE_PARITY_PLAN.md](../../docs/KINDLE_PUZZLE_PARITY_PLAN.md).

This is a list of independently assignable agent tasks, not an instruction to implement them as part of this planning commit. Prefer separate small PRs with focused host tests; coordinate changes to `src/main.rs`, and merge in dependency order.

| Task | Outcome | Depends on | Host gate |
| --- | --- | --- | --- |
| [00](00-contract-and-fixtures.md) | Pin compatibility contract and shared fixture corpus | None | Deterministic corpus/test manifest |
| [01](01-large-file-loading.md) | Read *unchanged* Kindle-sized JSON through Kobo/Cobalt | None | 256 KiB / 1 MiB / 8 MiB boundary and failure tests |
| [02](02-rich-parser-and-tree.md) | Validate/normalize additive v1 analysis tree and spans | 00 | Valid fixture, all invalid fixtures, legacy regression |
| [03](03-topic-and-descriptions.md) | Topic and description presentation models | 02 | Text/Unicode/fallback/no-grading tests |
| [04](04-analysis-browser-state.md) | Separate, read-only preview state | 02 | State transition and progress-identity tests |
| [05](05-pgn-projection.md) | Deterministic Kindle-style inline notation document | 02 | Golden token/structure/ordering tests |
| [06](06-kobo-analysis-ui.md) | Visible Analysis button, hidden NOTE, board + paginated prose | 03, 04, 05 | Cobalt layout and page coverage tests |
| [07](07-analysis-actions.md) | Tap references/moves and preview exact FEN | 04, 06 | End-to-end Cobalt action/tap tests |
| [08](08-parity-integration.md) | Import/collection/CI and regression acceptance | 01–07 | `cargo test`, `cargo fmt --all -- --check`, Cobalt simulator |
| [09](09-physical-device-checkpoint.md) | Owner-checkable on-device release checklist | 08 | Host-gated, then human checkpoint |

**Parallel lanes:** 01 can run independently of 00–07. After 00, 02 starts; after 02, tasks 03, 04 and 05 can be implemented in parallel. 06 integrates those three; 07 follows the UI. 08 is the single integration gate. Only 09 requires physical-device confirmation.

**Common rules for every task**
- Use root collection `version: 1` + optional `analysis.version: 1`; **do not** implement legacy Kobo Task 07's proposed root `version: 2` / `solutions` format.
- Retain `solution` as the *only* graded path, including automatic replies, promotion, wrong-move rollback, solved-ID persistence, Free Board and orientation behavior. `alternative` and `sideline` are browse-only.
- Rich analysis must never write progress, mark a puzzle solved, or edit uploaded JSON.
- Keep current full-size board; no small-board mode, game-review workspace, chess engine, in-device PGN parser, settings page, or configurable toolbar.
- Render **Analysis** in the normal toolbar without adding settings, hide/remove the **NOTE** toolbar control, but retain post-solve description behavior. See Task 06 for legacy/no-analysis behavior.
- Automated tests must run on host without device; use explicit diagnostics/fixtures, not manual-only assertions. Record any unverified physical checks in Task 09.
- Preserve Kobo Libra H2O direct NickelMenu launch, safe return to Nickel, Cobalt pin and progress semantics. Do not require an attached Kobo to pass CI.

**Per-agent handoff:** Each PR should name changed paths, fixture IDs, precise commands/results, new failure tests, unchanged legacy behaviors and any risks. Never mark Task 09 done based only on simulator output.
