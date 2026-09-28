# Task 05 — Persist solved progress and per-file position

**Status:** Implemented  
**Depends on:** Tasks 01 and 04  
**Primary files:** new progress/state module, `src/main.rs`, tests

## Outcome

Remember learning progress across app restarts and across collection switches: which puzzles are solved, which collection is active, and which puzzle was last open in each collection.

## Persistence design

Do **not** modify uploaded puzzle JSON files.

Store one separate versioned JSON record under the Cobalt store key:

`progress.v1`

Suggested logical shape:

```json
{
  "version": 1,
  "active_file": "puzzles-endgames.json",
  "files": {
    "puzzles.json": {
      "current_puzzle_id": "lichess-001cr",
      "solved_ids": ["lichess-001cr"]
    }
  }
}
```

Exact Rust structs may differ, but these semantics are required.

Use puzzle IDs, not array indexes, for durable references.

## Required behavior

1. Load `progress.v1` during startup together with collection discovery.
2. Missing progress is a normal first-run state.
3. If the progress record is valid:
   - prefer its `active_file` when that collection still exists;
   - for each collection, restore `current_puzzle_id` when that puzzle ID still exists.
4. If the remembered file or puzzle was removed/renamed, fall back safely:
   - active file: `puzzles.json` if present, otherwise first sorted collection;
   - puzzle: index 0.
5. When a solution reaches **Solution complete**, add that puzzle ID to the active file's solved set and save immediately.
6. When the current puzzle changes, update that file's `current_puzzle_id` and save.
7. When switching collections:
   - first retain the old file's current puzzle ID in memory;
   - activate the new file;
   - restore the new file's remembered puzzle ID if present;
   - update `active_file`;
   - save.
8. Display solved state while browsing. A simple explicit status such as **Solved** next to the side-to-move metadata is sufficient.
9. Solving an already-solved puzzle again is allowed and must not create duplicates.
10. Reset does not clear solved state.
11. Free board never changes solved state.
12. On a save refusal/failure:
    - keep the newest progress in memory;
    - show an attention message such as **Progress not saved**;
    - retry on the next progress mutation and when suspension/background gives an opportunity.
13. A malformed or unsupported future progress version must not be overwritten silently. Show a warning and run without durable progress until the record is fixed/cleared deliberately.

## Implementation notes

Put serialization/validation in a small platform-neutral module rather than embedding JSON handling throughout `main.rs`.

Cobalt store writes are atomic and asynchronous. Track dirty/saving state explicitly enough that a late acknowledgement cannot make newer in-memory progress look saved.

Do not use puzzle collection titles as identity; filenames/store keys are the collection identity.

## Automated acceptance tests

Cover at least:

- missing progress starts clean;
- solved IDs serialize/deserialize without duplicates;
- completing a puzzle marks only that puzzle in only that file;
- Reset and Free board do not alter solved IDs;
- switching files restores each file's remembered puzzle ID;
- restart simulation restores active file and current puzzle;
- missing remembered puzzle falls back to first puzzle;
- missing remembered active file uses the defined fallback;
- corrupt/future progress is preserved and not overwritten automatically;
- save failure keeps dirty progress and exposes the warning;
- solved marker and warning screens pass Libra H2O diagnostics.

## Physical-device test

1. Use two valid collections.
2. In Tactics, browse to a non-first puzzle and solve it.
3. Confirm **Solved** is shown.
4. Switch to Endgames, browse to a different non-first puzzle, then switch back.
5. Confirm Tactics returns to its previous puzzle.
6. Switch again and confirm Endgames returns to its own previous puzzle.
7. Exit to Nickel, reopen the app, and confirm:
   - the previously active file is restored;
   - its current puzzle is restored;
   - solved status is still visible.
8. Reset a solved puzzle and confirm it remains marked solved.
9. Enter Free board, move pieces, return to Solution, and confirm the durable solved marker is unchanged.

## Out of scope

- unsolved-only navigation;
- attempt counts, timestamps, streaks, or statistics.
