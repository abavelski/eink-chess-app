# Task 03 — Solution mode / Free board mode

**Status:** Ready  
**Depends on:** Tasks 01 and 02  
**Primary files:** `src/main.rs`, screen diagnostics/tests

## Outcome

Make the app's two purposes explicit: graded puzzle solving and an ungraded physical scratch board.

## Required behavior

1. Add a visible two-state control:
   - **Solution**;
   - **Free board**.
2. The active mode must be obvious without color.
3. Solution mode keeps all behavior from Tasks 01–02.
4. Free board mode:
   - allows unlimited piece moves using the existing physical-board semantics;
   - does not compare moves with the solution;
   - does not advance or complete a solution;
   - does not change any solved/progress state;
   - still supports Reset, Flip, page-turn puzzle navigation, and pawn promotion.
5. Entering Free board starts from the board position currently visible. It does not reset immediately.
6. Reset while in Free board restores the current puzzle's FEN but remains in Free board.
7. Navigating to another puzzle while in Free board loads that puzzle normally and remains in Free board.
8. Switching from Free board back to Solution:
   - resets the current puzzle to its original FEN;
   - clears any temporary selection and solution feedback;
   - restarts the solution attempt from its first solver move;
   - keeps the user's current board orientation unless a later product decision changes this.
9. A puzzle that was previously marked solved in a later task remains solved; Free-board exploration must never clear that durable fact.

## Implementation notes

Represent mode as an explicit enum rather than a boolean if it improves readability.

Do not duplicate board touch logic. Route the completed move either to free-board acceptance or the solution checker.

The current compact toolbar has tight physical sizing. Re-run diagnostics after adding the toggle and move the control to another standard Cobalt surface if three square controls no longer fit comfortably.

## Automated acceptance tests

Add tests proving:

- Free board accepts a move that is wrong according to the solution;
- making free-board moves does not advance solution state;
- returning to Solution resets to the FEN and solution start;
- Flip survives the Free -> Solution transition;
- Reset in Free board restores FEN without changing mode;
- puzzle navigation does not change mode;
- promotion still works in Free board;
- every mode/feedback screen passes Libra H2O diagnostics.

## Physical-device test

1. Open a puzzle in Solution mode and make a wrong move; confirm it is rejected.
2. Switch to Free board.
3. Make several arbitrary moves, including moving onto an occupied square.
4. Flip the board and continue moving pieces.
5. If using a promotion fixture, promote a pawn and confirm the chooser still appears.
6. Press Reset; confirm the puzzle FEN returns and the app remains in Free board.
7. Browse forward/back with the physical buttons; confirm Free board remains active.
8. Return to the original puzzle, make arbitrary changes, then switch to Solution.
9. Confirm the puzzle resets to its FEN, the retained orientation is unchanged, and the correct solution can be entered normally.

## Out of scope

Free board is not a legal chess game and gains no turn/rule enforcement.
