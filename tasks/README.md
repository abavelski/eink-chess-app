# Implementation tasks

These tasks turn the ideas in [docs/NEXT_STEPS.md](../docs/NEXT_STEPS.md) into implementation-ready work. They are ordered by dependency and by value on the device, not by the order in which the ideas were recorded.

## Order

1. [01-solution-checking.md](01-solution-checking.md) — check solver moves, show feedback, and auto-play stored opponent replies.
2. [02-pawn-promotion.md](02-pawn-promotion.md) — make promotion a real move with a piece-choice dialog.
3. [03-free-board-mode.md](03-free-board-mode.md) — restore the physical-board workflow as an explicit mode.
4. [04-multiple-puzzle-files.md](04-multiple-puzzle-files.md) — discover and switch between several puzzle collections.
5. [05-persist-progress.md](05-persist-progress.md) — remember solved puzzles, active file, and current puzzle.
6. [06-unsolved-only-navigation.md](06-unsolved-only-navigation.md) — optionally browse only puzzles that are not solved.
7. [07-rich-solutions.md](07-rich-solutions.md) — add descriptions and multiple valid solution lines with a backward-compatible format.

Tasks 1–3 are intentionally small enough to land independently. Tasks 4–6 build the collection/progress workflow. Task 7 is intentionally larger and should come after the basic solving state machine is stable.

## Common definition of done

Every task must:

- keep the application usable on Kobo Libra H2O;
- keep the board model platform-neutral where practical;
- preserve the existing direct NickelMenu launch/return flow;
- keep `cargo test` passing and add automated tests for new state transitions;
- run screen diagnostics for every new UI state that can appear on the Libra H2O metrics;
- avoid relying on color; selected/active/error/success state must be clear in monochrome;
- avoid unnecessary full-screen changes on e-ink;
- use the Cobalt revision currently pinned in `Cargo.toml` as the source of truth;
- be tested on the physical reader using the existing update procedure in `docs/DEVICE_UPDATE_DIRECT_LAUNCH.md`.

The app is still not a chess engine. Unless a task explicitly says otherwise, do not add legal-move validation, check/checkmate detection, turn legality, castling logic, or en-passant logic.

## Store/file conventions used by these tasks

Uploaded puzzle collections remain source data. Do not rewrite them to store learning progress. Task 5 introduces a separate versioned progress record in the app store.

For multi-file support, use store keys:

- `puzzles.json` for the existing/default collection;
- `puzzles-<name>.json` for additional collections.

`<name>` must use characters accepted by the Cobalt store key rules. Files copied directly into the app state directory therefore remain discoverable through `context.store().list()`.

When a task needs a modal/list/filter UI, prefer existing Cobalt primitives such as `ScreenBuilder::modal`, `rows`, `choose`, `chips`, and `banner` rather than adding custom rendering to Cobalt.
