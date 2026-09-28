# Task 04 — Multiple puzzle files and collection picker

**Status:** Implemented  
**Depends on:** Tasks 01–03  
**Primary files:** `src/puzzle.rs`, `src/main.rs`, `docs/PUZZLE_FORMAT.md`

## Outcome

Allow several uploaded puzzle collections to coexist and let the user switch between them from the app.

## File convention

Discover store keys matching either:

- exactly `puzzles.json`; or
- `puzzles-<name>.json`.

Sort keys lexicographically for deterministic presentation. Ignore unrelated store keys.

Keep `puzzles.json` as the backward-compatible default/fallback collection.

## Required behavior

1. On startup, request `context.store().list()`.
2. From `StoreResult::Keys`, build the available collection list using the naming convention above.
3. If no puzzle collection exists:
   - save the bundled examples as `puzzles.json`;
   - then load it.
4. Refactor parsing to retain collection-level metadata needed by the UI, at least optional `title`, rather than returning only `Vec<Puzzle>`.
5. Add a visible **Puzzles** or **Files** control on the main screen.
6. Pressing it opens a modal/list of discovered collections:
   - display non-empty collection `title` when available;
   - otherwise display the store key.
7. Selecting a collection loads it asynchronously.
8. Do not replace the active collection until the selected file has loaded and parsed successfully.
9. On successful load:
   - activate the selected collection;
   - select its first puzzle for now;
   - close the picker.
10. If loading/parsing the selected collection fails:
    - keep the previous collection and current board active;
    - show a clear error naming the failed collection;
    - allow the picker to be opened again.
11. Existing Reset, Flip, Solution/Free mode, and physical page-turn navigation work inside the active collection.
12. Document how to upload additional files and the required naming convention.

## Implementation notes

Cobalt already exposes `context.store().list()` and returns `StoreResult::Keys(Vec<String>)`.

Keep asynchronous states explicit: listing, loading a requested collection, and active collection. Do not infer which load completed from global mutable strings when the callback already carries the key.

Do not persist the selected file in this task; Task 05 does that.

## Automated acceptance tests

Using `AppRunner`, cover:

- startup issues a store List request;
- only valid puzzle collection keys appear in the picker;
- collection ordering is deterministic;
- no files causes bundled examples to be saved as `puzzles.json`;
- selecting a valid second file switches the collection and starts at puzzle 0;
- invalid second file leaves the first file/board active and reports an error;
- collection title is used when present, filename otherwise;
- page-turn buttons navigate only within the active collection;
- picker/main/error states pass Libra H2O diagnostics.

## Physical-device test

Prepare two visibly different collections, for example:

- `puzzles.json` titled "Tactics";
- `puzzles-endgames.json` titled "Endgames".

Copy both into the app's state directory while the app is closed, using the same USB workflow already documented for `puzzles.json`.

1. Launch the app and open the collection picker.
2. Confirm both titles appear.
3. Select Endgames and confirm its first position/description appears.
4. Browse several Endgames puzzles with the physical page buttons.
5. Open the picker and switch back to Tactics.
6. Add a deliberately malformed `puzzles-broken.json`, relaunch/open picker, select it, and confirm:
   - an error is shown;
   - the previously active valid board remains usable.
7. Remove the malformed file after testing.

## Out of scope

- remembering the selected collection;
- remembering the current puzzle per collection;
- solved-state filtering.
