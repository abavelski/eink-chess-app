# Task 01 — Check puzzle solutions

**Status:** Ready  
**Depends on:** nothing  
**Primary files:** `src/board.rs`, `src/main.rs`, tests in those modules

## Outcome

Turn the existing puzzle viewer into a basic solver. The user makes only the solver-side moves. Correct moves are accepted, wrong moves are rejected, stored opponent replies are applied automatically, and the app clearly reports completion.

The current version-1 `solution` array is interpreted as alternating plies:

- index 0: solver move;
- index 1: opponent reply;
- index 2: solver move;
- index 3: opponent reply;
- and so on.

## Required behavior

1. A completed board move must produce its origin and destination squares so `main.rs` can form UCI such as `e2e4`. Selection/deselection alone is not a move.
2. In Solution mode, compare each user move with the next expected solver move.
3. On a wrong move:
   - restore the exact board position from before the attempted move;
   - do not advance solution progress;
   - show an obvious attention message such as **Wrong move — try again**.
4. On a correct move:
   - keep the move on the board;
   - show a clear **Correct** message;
   - if the next stored ply is an opponent reply, apply that exact UCI move automatically;
   - then wait for the next solver move.
5. When no solution plies remain, show **Solution complete** prominently.
6. After completion, ignore further square taps in Solution mode until Reset, puzzle navigation, or a later Free-board mode switch.
7. Reset and selecting another puzzle both clear the current attempt and feedback.
8. Flip only changes presentation; it must not change square/UCI interpretation or solution progress.
9. Do not add chess legality. The only correctness rule is equality with the stored UCI line.

For this task, user-entered promotion moves are not required; Task 02 adds them. Non-promotion opponent replies must be applied by coordinate even if they would be illegal chess moves.

## Implementation notes

Prefer changing the board API so callers can distinguish selection changes from a completed move. For example, return an enum such as `NoChange | SelectionChanged | Moved { from, to }`, or expose a separate move operation. Avoid deriving a move later by diffing two board snapshots.

Add small square/UCI helpers with tests. Board indexes are currently `0 = a8` and `63 = h1`.

Keep attempt state in the app, for example a cursor pointing at the next solver ply. Do not persist it yet.

When applying an automatic reply, use an explicit board move-by-squares API rather than simulating taps.

## Automated acceptance tests

Add tests proving at least:

- selecting and deselecting does not produce a completed move;
- `e2 -> e4` produces `e2e4` regardless of board flip;
- a wrong solver move leaves the board and solution cursor unchanged;
- the one-move example puzzle completes after its correct move;
- the existing mate-in-two example accepts `e2e6`, automatically applies `f7f8`, then completes after `e6f7`;
- Reset returns that mate-in-two puzzle to its original FEN and solution start;
- changing puzzles clears transient Correct/Wrong/Complete feedback;
- all screen diagnostics remain clean on Libra H2O metrics.

## Physical-device test

Deploy using the existing device update flow.

1. Open the first mate-in-one sample.
2. Make an obviously wrong move. Confirm:
   - **Wrong move — try again** is visible;
   - the moved piece returns to its prior square.
3. Enter the correct move `d7e8`. Confirm **Solution complete**.
4. Try another board tap. Confirm the solved board does not change.
5. Browse to puzzle `lichess-000hf`.
6. Play `e2e6`. Confirm **Correct**, then confirm the stored opponent move `f7f8` appears without user input.
7. Play `e6f7`. Confirm **Solution complete**.
8. Press Reset and confirm the original FEN and a fresh attempt.
9. Flip the board and solve again to confirm orientation does not affect checking.

## Out of scope

- persisted solved state;
- promotion selection;
- alternate solution branches;
- explanatory solution text;
- legal-move validation.
