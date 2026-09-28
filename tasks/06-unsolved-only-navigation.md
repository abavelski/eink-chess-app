# Task 06 — Unsolved-only navigation

**Status:** Ready  
**Depends on:** Task 05  
**Primary files:** `src/main.rs`, tests

## Outcome

Add an optional filter so the physical page-turn buttons browse only puzzles that have not yet been solved.

## Required behavior

1. Add a clearly visible toggle control with two states:
   - **All**;
   - **Unsolved only**.
2. Default to **All** on each app launch. Do not persist the filter in this task.
3. In All mode, previous/next behavior remains unchanged.
4. In Unsolved-only mode, page-turn navigation skips puzzle IDs in the active collection's solved set.
5. Preserve the current no-wrap navigation behavior:
   - forward stops when there is no later unsolved puzzle;
   - backward stops when there is no earlier unsolved puzzle.
6. When enabling the filter while the current puzzle is solved:
   - choose the next unsolved puzzle after the current one when available;
   - otherwise choose the nearest earlier unsolved puzzle;
   - if none exists, keep the current board visible and show **All puzzles solved**.
7. Completing the currently displayed puzzle while the filter is enabled does **not** immediately jump away. Keep **Solution complete** visible. The next page-turn then skips to the next unsolved puzzle.
8. When switching puzzle files, keep the filter mode for the current app session and apply it to the new file's solved set.
9. Switching back to All immediately restores normal navigation across solved and unsolved puzzles.
10. The active filter state must be obvious in monochrome and must not crowd the existing board/toolbar.

## Implementation notes

Centralize "find next index" logic in a small pure function that receives direction, current index, puzzle IDs, solved set, and filter mode. Do not duplicate skipping loops in event handlers.

Use a standard selected-state UI such as chips or an equivalent Cobalt control if it fits better than another square toolbar button.

## Automated acceptance tests

Cover:

- All mode visits every adjacent puzzle;
- Unsolved mode skips solved IDs forward and backward;
- no-wrap behavior at both ends;
- enabling on a solved puzzle chooses next, then previous fallback;
- all-solved collection shows the explicit empty result/message;
- completing a puzzle does not auto-jump;
- switching files applies the same session filter to the other solved set;
- turning filter off restores access to solved puzzles;
- every filter/empty-state screen passes Libra H2O diagnostics.

## Physical-device test

Create a collection with at least five puzzles and mark a non-contiguous subset solved through normal solving.

1. In All mode, page through all five puzzles.
2. Enable Unsolved only.
3. Page forward/back and confirm solved puzzles are skipped.
4. While viewing a solved puzzle in All mode, enable Unsolved only and confirm it moves according to the next/previous rule.
5. Solve the current unsolved puzzle; confirm **Solution complete** remains visible.
6. Press page forward and confirm the next unsolved puzzle appears.
7. Solve every puzzle in the collection and confirm **All puzzles solved** rather than apparently broken navigation.
8. Switch back to All and confirm every puzzle can be browsed again.
