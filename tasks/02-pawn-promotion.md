# Task 02 — Pawn promotion

**Status:** Implemented  
**Depends on:** Task 01  
**Primary files:** `src/board.rs`, `src/main.rs`, possibly a small reusable dialog/state type

## Outcome

When a pawn is moved to its last rank, the move pauses for a promotion choice. The selected piece becomes part of the UCI move and therefore participates correctly in solution checking.

## Required behavior

1. Detect a White pawn moving to rank 8 or a Black pawn moving to rank 1.
2. Do not finalize such a user move immediately.
3. Show a modal with four choices:
   - Queen;
   - Rook;
   - Bishop;
   - Knight.
4. Include a Cancel action. Cancel closes the modal, leaves the pawn on its original square, and preserves the current attempt.
5. Choosing a piece:
   - moves the pawn to the destination;
   - replaces it with the chosen same-color piece;
   - emits UCI with the lowercase suffix `q`, `r`, `b`, or `n`.
6. In Solution mode, only compare the move after the promotion choice is made.
7. If the chosen promotion is wrong for the solution, apply Task 01 wrong-move behavior and restore the pre-move position.
8. Automatic opponent replies that contain a promotion suffix must apply the specified promotion directly without asking the user.
9. Underpromotion must work; never silently assume Queen.
10. The dialog must fit Libra H2O and use touch targets large enough for the device.

## Implementation notes

This is easiest if a move can be staged before mutating the board. Keep enough pending state to identify `from`, `to`, pawn color, and the pre-move board.

Add an explicit board operation that can place the promoted piece atomically. Do not implement promotion by moving the pawn first and then patching the square through UI state.

Use Cobalt's modal/choice/list primitives; do not create a bespoke renderer.

## Automated acceptance tests

Add tests proving:

- White promotion is detected on rank 8;
- Black promotion is detected on rank 1;
- non-pawn moves to the last rank do not trigger promotion;
- each of q/r/b/n produces the expected piece and UCI suffix;
- Cancel leaves the board unchanged;
- wrong underpromotion is rejected and restored in Solution mode;
- a correct underpromotion can complete a puzzle;
- an automatic opponent promotion is applied without opening the modal;
- promotion-modal screen diagnostics pass on Libra H2O metrics.

Add a small test fixture or test-only puzzle containing both a normal queen promotion and an underpromotion so device verification is reproducible.

## Physical-device test

1. Install a build containing the promotion test collection.
2. Open a position with a White pawn on the 7th rank and move it to the 8th.
3. Confirm the promotion dialog appears before the board changes.
4. Cancel and confirm the pawn is still on the 7th rank.
5. Repeat and choose Queen; confirm the destination shows a white queen.
6. Open the underpromotion puzzle, choose an incorrect promotion, and confirm **Wrong move** plus board restoration.
7. Retry with the expected Knight/Rook/Bishop and confirm the move is accepted.
8. Repeat with a Black pawn promoting on rank 1.
9. Confirm Flip does not change which physical rank triggers promotion.

## Out of scope

All other chess rules remain unenforced.
