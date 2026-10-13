# Puzzle files (version 1)

Use JSON: it has standard parsers, is easy to generate, and keeps solution
moves and descriptions together. The app discovers UTF-8 puzzle collections in:

```text
KOBOeReader/.adds/cobalt/state/eink-chess/
```

The backward-compatible default collection is `puzzles.json`. Additional
collections must use `puzzles-<name>.json`, for example
`puzzles-endgames.json` or `puzzles-mate-in-two.json`. Files with other names
are ignored by the collection picker.

Close E-Ink Chess, connect the Kobo by USB, and copy or edit the files. Eject
the reader before disconnecting, then reopen the app. No rebuild is needed.
Each file must be no larger than 256 KiB (Cobalt's store limit). Use ordinary
JSON without comments or trailing commas.

## Example

```json
{
  "version": 1,
  "puzzles": [
    {
      "id": "lichess-001cr",
      "fen": "8/3B2pp/p5k1/6P1/1ppp1K2/8/1P6/8 w - - 0 39",
      "description": "Solution begins with d7e8.\nFollow the line one move at a time.\nReplay it to practice the idea.",
      "solution": ["d7e8"],
      "source": "https://lichess.org/training/001cr"
    }
  ]
}
```

The root requires `version: 1` and a nonempty `puzzles` array. Puzzles appear
in array order. Optional `title` is used as the collection name in the
on-device **Puzzles** picker; if it is missing or blank, the filename is shown.
Other collection metadata such as `source` and `license` is ignored by the
current app.

| Puzzle field | Required | Meaning |
| --- | --- | --- |
| `id` | Yes | A nonempty string, unique within the file. |
| `fen` | Yes | The complete six-field FEN at the moment the solver should move. |
| `description` | No | An explanation displayed below the board and toolbar after solving or when the description toggle is active. Use `\n` for line breaks. May be omitted, empty, or `null`. |
| `difficulty` | No | A string or number shown in parentheses after the puzzle ID in the header, for example `"Hard"` or `4`. May be omitted or `null`. |
| `solution` | Yes | A nonempty array of UCI moves, beginning with the solver's move and including alternating opponent replies. |
| `source` | No | Optional provenance URL, ignored by the current app. |

## Whose turn and board orientation

The second FEN field defines the turn: `w` means White and `b` means Black.
There is no separate turn field to keep in sync. That side is the solving color
and is placed at the bottom of the board whenever a puzzle is selected.

The **Flip** control still works. **Reset** restores the current FEN's pieces
and preserves a manual flip. Selecting another puzzle restores its initial
position and chooses its orientation from its FEN.

The **orientation lock** key icon below the board keeps the current orientation
when browsing puzzles or switching collections. Its gray fill indicates that
the lock is active. **Flip** still works while locked; the new orientation is
then kept for subsequent puzzles. Turning the lock off keeps the current board
as it is and resumes automatic orientation when another puzzle is selected.
The lock starts off when the app is reopened.

## Solutions

UCI notation uses the origin and destination squares: `e2e4`. Promotions add
the lowercase piece: `a7a8q`, `a7a8r`, `a7a8b`, or `a7a8n`. Standard castling
uses the king's move, such as `e1g1`.

For a two-move solution, the array normally contains the solver's first move,
the opponent's reply, and the solver's second move. The file represents one
main solution line; branching alternatives can be introduced in a later
format version.

The app validates FENs and move notation, displays the turn,
and uses the solution as an exact move sequence. The user enters only the solver
plies (indexes 0, 2, 4, ...). A correct solver move is kept, the following stored
opponent ply is applied automatically, and the app then waits for the next
solver move. A wrong move is restored immediately and does not advance the
solution. A correct intermediate move adds **Correct** to the turn line.
Finishing the sequence shows a centered thumbs up over the board; an incorrect
move shows a thumbs down. Completing the solution also reveals its optional
description below the board and toolbar. Reset, changing puzzles, or entering
Free board hides the description. The note icon below the board toggles the
description on or off at any time, including before solving and in Free board.
Its gray fill indicates that visibility is on. Revealing the description does
not advance the solution or mark the puzzle as solved. Reset removes the result
icon.

This is solution matching, not general chess-rule enforcement: the app still
does not decide whether an ordinary move is legal, whether a king is in check,
or whether a position is checkmate.

Pawn promotion is implemented. When a pawn is moved to its last rank, the board
does not change until the user chooses **Queen**, **Rook**, **Bishop**, or
**Knight** in the promotion dialog. The selected piece is appended to UCI as
`q`, `r`, `b`, or `n` before solution checking. Cancel leaves the pawn on
its original square. Stored opponent promotion replies are applied
automatically without opening the dialog.

The board has two modes. **Solution** checks moves as described above. **Free
board** keeps the currently visible position but accepts arbitrary physical-board
moves without grading or advancing the solution. Reset still restores the
current FEN, Flip still changes orientation, and page buttons still browse
puzzles. Returning from Free board to Solution restores the puzzle FEN and
restarts its solution while preserving the current orientation.
The leftmost grid icon below the board toggles the modes. It has a gray fill
while Free board is active.

## Multiple collections, missing files, and invalid files

At startup the app lists keys matching `puzzles.json` or
`puzzles-<name>.json`, sorts them by filename, and reads their metadata for the
picker. If valid, `puzzles.json` is the preferred initial collection;
otherwise the first valid discovered collection is used.

Press **Puzzles** to switch collections. A successful switch opens the selected
file at its first puzzle. If a selected file is missing, unreadable, or invalid,
the current collection and board stay active and an error names the file that
failed. Uploaded files are never rewritten just because parsing failed.

If no matching puzzle collection exists at all, the app creates and loads the
bundled examples as `puzzles.json`. If matching files exist but none are valid,
the app shows the bundled examples in memory while leaving the uploaded files
untouched so they can be fixed over USB.

Existing `positions.fen` files are left in place but are no longer loaded.

## Progress

Learning progress is kept separately from uploaded puzzle collections in the
Cobalt store record `progress.v1`. Puzzle JSON files are never modified to
record progress.

The progress record remembers the active collection, the current puzzle ID for
each collection, and the set of solved puzzle IDs. Durable references use
puzzle IDs rather than array positions, so reordering a collection does not move
progress to the wrong puzzle. Returning to a collection restores its remembered
puzzle when that ID still exists; otherwise the first puzzle is used. On app
startup the remembered active collection is preferred when it still exists,
with `puzzles.json` and then the first valid sorted collection as fallbacks.

A puzzle becomes **Solved** only when its complete solution line is accepted.
Reset and Free board do not clear or add solved state. Save failures leave the
newest progress in memory and display **Progress not saved**; the app retries on
the next progress change or lifecycle save opportunity. A malformed or future
`progress.v1` is not overwritten automatically: the app warns and continues
without durable progress until that record is deliberately fixed or cleared.

## Lichess imports

The [Lichess puzzle database](https://database.lichess.org/#puzzles) is CC0.
Its CSV `FEN` is **before** the opponent's setup move. Apply the first move of
`Moves` with a chess library, export the resulting FEN, and use all remaining
moves as `solution`. Simply copying the CSV FEN would present the wrong turn
and position. See [example provenance](../examples/README.md).
