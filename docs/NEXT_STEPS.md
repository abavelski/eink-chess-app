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

## 2. Support multiple puzzle files

Allow several puzzle JSON files to be uploaded to the device instead of relying on a single `puzzles.json` collection.

### Goal

Make it easy to keep separate puzzle sets on the Kobo and switch between them without reconnecting the device or replacing the current file.

### Expected behavior

- Discover the available puzzle files stored for the app.
- Add a visible button to the main UI for choosing the active puzzle file.
- Pressing the button opens a simple selection dialog listing the available uploaded puzzle files.
- Selecting a file loads that puzzle collection and closes the dialog.
- After switching files, show the puzzles from the newly selected collection and start from an appropriate initial puzzle in that collection.
- Keep the existing puzzle browsing controls working within the currently selected file.
- Handle an invalid or unreadable selected file with a clear error instead of crashing or silently switching to another collection.

### Notes

- Reuse the existing puzzle JSON format; multiple-file support should not require a new puzzle schema.
- The file-selection UI should be simple and high-contrast for the e-ink display, with large touch targets.
- The exact storage naming rules and whether the last selected file should be remembered across launches can be decided during implementation.

## 3. Persist progress for each puzzle file

Keep learning progress with each puzzle file so that switching collections or restarting the app does not lose the user's place.

### Goal

Remember which puzzles have already been solved correctly and which puzzle the user was viewing in each puzzle file.

### Expected behavior

- Track whether each puzzle has been solved correctly.
- When browsing puzzles, clearly indicate puzzles that have already been solved.
- Persist this solved state so it survives app restarts.
- Persist the current puzzle for each puzzle file.
- When switching away from a puzzle file and later selecting it again, restore the puzzle that was active in that file rather than starting from the beginning.
- Progress in one puzzle file must not affect another puzzle file.
- A puzzle should only be marked solved after its complete solution has been entered correctly.

### Notes

- The persisted state should be associated with the puzzle file itself, including both per-puzzle solved status and the file's current puzzle.
- Prefer stable puzzle IDs for saved progress rather than relying only on array positions, so reordering puzzles does not move solved status to the wrong puzzle.
- The exact representation can be decided during implementation; if progress is written directly into the puzzle JSON, the puzzle format/versioning and compatibility with externally supplied files will need to be updated deliberately.
- Solved-state indicators should remain clear on a monochrome e-ink display and should not depend on color.

## 4. Browse only unsolved puzzles

Add an optional navigation filter so the existing puzzle browsing controls can skip puzzles that have already been solved.

### Goal

Make practice sessions more efficient by letting the user move through only the puzzles that still need work.

### Expected behavior

- Add a visible toggle button for switching between **All puzzles** and **Unsolved only** browsing.
- When the filter is off, navigation behaves as it does today and moves through every puzzle in the active file.
- When the filter is on, previous/next navigation skips puzzles already marked solved and lands only on unsolved puzzles.
- The toggle state should be visually obvious on the monochrome e-ink display.
- If the current puzzle is solved when the filter is enabled, move to an appropriate unsolved puzzle if one is available.
- If no unsolved puzzles remain in the active file, show a clear message rather than making navigation appear broken.
- Switching the filter off should immediately restore normal browsing across the full collection.

### Notes

- This feature depends on the persisted solved-state tracking described above.
- The exact wording and placement of the toggle can be refined with the rest of the toolbar/navigation UI.
- Whether the filter state itself should persist across app restarts can be decided during implementation.

## 5. Richer solution mode

Expand the current single-line solution concept into a richer solution mode that can represent more than one fixed sequence of UCI moves.

### Goal

Support puzzles where the solution may include explanatory text, alternative continuations, or multiple valid branches instead of assuming one linear move sequence.

### Ideas to explore

- Allow solution steps or branches to include optional descriptions or explanations.
- Support multiple valid solution branches where more than one move or continuation is acceptable.
- Allow branch-specific follow-up moves and explanations.
- Make the solution UI able to communicate why a move is correct, not only whether it matches.
- Keep the representation suitable for authored learning material, not only imported tactical puzzles.
- Consider how richer solution data should be displayed progressively so the answer is not revealed too early.

### Notes

- This is likely a larger change involving both the puzzle data format and the solution-checking state machine.
- Do not lock in the schema yet; the interaction model and file format should be designed together once the basic solution-checking flow is working.
- Backward compatibility with the current simple `solution: ["e2e4", ...]` format should be considered when this is implemented.

## 6. Solution mode and free-board mode

Add an explicit mode switch between normal puzzle solving and a temporary free-board workspace.

### Goal

Let the user explore a position freely, like using a physical chessboard, without affecting puzzle progress or the expected solution sequence.

### Expected behavior

- Add a visible toggle button for switching between **Solution mode** and **Free board**.
- In **Solution mode**, moves are checked against the puzzle solution and normal puzzle progress/feedback applies.
- In **Free board** mode, the user can move pieces around freely without moves being graded or solution progress being changed.
- Board rotation and the other normal board controls should remain available in Free board mode.
- The user can make as many exploratory moves as desired while in Free board mode.
- When switching from Free board back to Solution mode, reset the puzzle to its original starting position.
- After the reset, solution checking should resume from the beginning of the puzzle as normal.
- Free-board exploration must not mark a puzzle solved or modify its persisted solved state.

### Notes

- Free board should feel like a scratch board attached to the current puzzle rather than a separate chess game.
- The active mode should be very obvious on the e-ink display so the user knows whether moves are currently being graded.
- Whether board orientation should reset when returning to Solution mode can be decided during implementation; the puzzle position itself should definitely reset.

## 7. Pawn promotion

Implement pawn promotion when a pawn is moved onto the last rank.

### Goal

Handle one essential chess rule that cannot be represented correctly by simply moving the pawn piece to its destination square.

### Expected behavior

- Detect when a pawn move ends on the last rank for that pawn.
- Before finalizing the move, open a simple promotion dialog.
- Let the user choose one of the standard promotion pieces: **Queen**, **Rook**, **Bishop**, or **Knight**.
- Replace the pawn with the selected piece on the destination square.
- In Solution mode, include the selected promotion piece when converting the move to UCI, for example `a7a8q`.
- Only compare the move with the expected solution after the promotion choice has been made.
- In Free board mode, promotion should still work, but without grading the move.
- The promotion dialog should use large, clear touch targets suitable for the e-ink display.

### Notes

- Promotion applies to both White reaching rank 8 and Black reaching rank 1.
- The current puzzle format already supports promoted UCI moves such as `a7a8q`, so solution files do not need a schema change for this feature.
- Underpromotion to rook, bishop, or knight should be supported from the start rather than assuming promotion always means queen.

