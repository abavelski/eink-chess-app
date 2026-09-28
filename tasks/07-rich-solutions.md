# Task 07 — Rich solution descriptions and alternative lines

**Status:** Ready  
**Depends on:** Tasks 01–06  
**Primary files:** `src/puzzle.rs`, solution-state code, `src/main.rs`, `docs/PUZZLE_FORMAT.md`

## Outcome

Extend the simple version-1 solution array into a richer, backward-compatible format that can explain moves and accept several valid solution lines.

This is intentionally the largest task in the current plan.

## Version-2 format

Continue accepting existing version-1 files unchanged.

Add puzzle-file `version: 2`. In version 2, each puzzle uses `solutions` instead of the version-1 `solution` array:

```json
{
  "version": 2,
  "puzzles": [
    {
      "id": "example",
      "fen": "7k/8/5KQ1/8/8/8/8/8 w - - 0 1",
      "description": "Find the best continuation.",
      "solutions": [
        {
          "description": "The direct mating line.",
          "steps": [
            { "move": "g6g7", "description": "Take away the final flight square." }
          ]
        }
      ]
    }
  ]
}
```

A solution may contain multiple lines. A step has:

- required `move` in the same UCI syntax already validated;
- optional `description`.

A line has:

- non-empty `steps`;
- optional `description`.

The same alternating semantics remain: even step indexes are solver moves; odd step indexes are automatic opponent replies.

## Required behavior

1. Parse and validate both version 1 and version 2 into one internal solution model.
2. Version-1 behavior must remain unchanged.
3. For version 2, keep a set of candidate lines compatible with the accepted history.
4. On a solver move:
   - accept it if it matches the next solver step in at least one candidate line;
   - reject/restore it if no candidate line matches;
   - after acceptance, discard non-matching candidates.
5. For an opponent reply:
   - if all remaining candidates specify the same next opponent move, auto-apply it;
   - if they specify different opponent moves, choose the reply from the first remaining line in file order, apply it, and discard candidates that chose another reply.
   - This makes device behavior deterministic without adding an engine.
6. Completion occurs when at least one remaining candidate line ends at the current history and no further solver move is required for the chosen continuation.
7. Show explanatory text progressively:
   - after an accepted solver move, display that step's description when non-empty;
   - after an automatic opponent reply, display its description when non-empty;
   - on completion, keep **Solution complete** prominent and also show the chosen line description when present.
8. Explanations must not reveal future moves before they are reached.
9. Pawn-promotion steps continue to use Task 02 behavior.
10. Solved persistence remains puzzle-level: completing any valid line marks the puzzle solved once.
11. Free board ignores all solution-line state and still resets to a fresh solution attempt when returning.
12. Update `docs/PUZZLE_FORMAT.md` with complete v1/v2 examples and authoring rules.

## Validation rules

Reject a v2 puzzle when:

- `solutions` is empty;
- any line has no steps;
- any step contains invalid UCI;
- a line ends immediately after an opponent ply, leaving no completed solver action as the final result.

Duplicate lines may be accepted or rejected, but choose one rule and test/document it.

## Implementation notes

Normalize v1 into the same runtime model used by v2; avoid keeping two separate checkers.

A vector of candidate line indexes plus a current step index is sufficient for this design; no recursive tree is required. Multiple lines intentionally allow repeated prefixes because they are easy to author and easy to validate.

Keep user-facing explanations separate from puzzle descriptions: the existing puzzle description is visible before solving; step/line descriptions are solution feedback and appear only after relevant progress.

## Automated acceptance tests

Add parser and runtime tests for:

- unchanged v1 parsing/solving;
- a v2 single-line puzzle with descriptions;
- two valid first solver moves;
- two lines sharing a prefix and branching later;
- deterministic opponent reply when candidate lines disagree;
- wrong move preserving all candidate lines/history;
- description appears only after its step is reached;
- completion through either valid line marks the puzzle solved;
- invalid empty lines and invalid UCI are rejected;
- promotion inside a v2 line works;
- rich feedback layouts pass Libra H2O diagnostics.

## Physical-device test

Prepare a v2 collection containing:

- one linear annotated puzzle;
- one puzzle with two valid first moves;
- one puzzle that branches after a shared prefix;
- one line containing a promotion.

Then:

1. Open the annotated puzzle and confirm future explanation text is not visible.
2. Make the first correct move and confirm only the reached explanation appears.
3. Finish it and confirm **Solution complete** plus the line description.
4. Open the two-first-move puzzle and solve it once through each accepted first move, resetting between attempts.
5. On the shared-prefix puzzle, enter the common prefix and then each branch on separate attempts.
6. Enter a move that belongs to no remaining line and confirm normal wrong-move restoration.
7. Restart the app and verify solved persistence still works for v2 puzzles.
8. Open an existing version-1 collection and confirm it behaves exactly as before.

## Out of scope

- chess-engine-generated alternatives;
- arbitrary nested instructional UI;
- hints that reveal future moves;
- scoring one valid line above another.
