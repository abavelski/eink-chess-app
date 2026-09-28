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

