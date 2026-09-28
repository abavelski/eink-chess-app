# Next-step ideas

Working notes for planned improvements to E-Ink Chess. These are product/development ideas, kept separate from the implementation and device documentation so they can be refined before coding.

## 1. Check puzzle moves against the solution

Use the existing `solution` array from each puzzle instead of only loading and retaining it.

### Goal

Turn a displayed puzzle into an interactive exercise: after the user moves a piece, compare that move with the next expected move in the puzzle solution and give clear feedback.

### Expected behavior

- Keep track of the current position in the puzzle's `solution` sequence.
- When the user completes a move, represent it in the same UCI form used by the puzzle file and compare it with the next expected solution move.
- If the move matches, show an explicit **Correct** message and advance solution progress.
- If the move does not match, show an explicit **Wrong** message and do not treat that solution step as completed.
- When the final expected move has been matched, show a clear **Solution complete** message.
- Reset solution progress when the puzzle is reset or when another puzzle is selected.

### Notes

- The existing puzzle format already stores solutions as UCI moves, so this should not require a puzzle-file format change.
- The first implementation should focus on solution checking and feedback. How opponent replies in multi-move solutions are handled (for example, user-played versus automatically applied) can be decided separately.
- Feedback should be easy to notice on an e-ink screen and should not rely on color alone.
