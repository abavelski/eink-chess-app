# Puzzle files (version 1)

Use JSON: it has standard parsers, is easy to generate, and keeps solution
moves and descriptions together. The app reads one UTF-8 file on each launch:

```text
KOBOeReader/.adds/cobalt/state/eink-chess/puzzles.json
```

Close E-Ink Chess, connect the Kobo by USB, and copy or edit this file. Eject the
reader before disconnecting, then reopen the app. No rebuild is needed to
change the puzzles. The file must be no larger than 256 KiB (Cobalt's store
limit). Use ordinary JSON without comments or trailing commas.

## Example

```json
{
  "version": 1,
  "puzzles": [
    {
      "id": "lichess-001cr",
      "fen": "8/3B2pp/p5k1/6P1/1ppp1K2/8/1P6/8 w - - 0 39",
      "description": "Find mate in one.",
      "solution": ["d7e8"],
      "source": "https://lichess.org/training/001cr"
    }
  ]
}
```

The root requires `version: 1` and a nonempty `puzzles` array. Puzzles appear in
array order. Optional collection metadata such as `title`, `source`, and
`license` is retained in the file and ignored by the current app.

| Puzzle field | Required | Meaning |
| --- | --- | --- |
| `id` | Yes | A nonempty string, unique within the file. |
| `fen` | Yes | The complete six-field FEN at the moment the solver should move. |
| `description` | No | A short instruction displayed below the board and toolbar. May be omitted, empty, or `null`. |
| `solution` | Yes | A nonempty array of UCI moves, beginning with the solver's move and including alternating opponent replies. |
| `source` | No | Optional provenance URL, ignored by the current app. |

## Whose turn and board orientation

The second FEN field defines the turn: `w` means White and `b` means Black.
There is no separate turn field to keep in sync. That side is the solving color
and is placed at the bottom of the board whenever a puzzle is selected.

The **Flip** control still works. **Reset** restores the current FEN's pieces
and preserves a manual flip. Selecting another puzzle restores its initial
position and chooses its orientation from its FEN.

## Solutions

UCI notation uses the origin and destination squares: `e2e4`. Promotions add
the lowercase piece: `a7a8q`, `a7a8r`, `a7a8b`, or `a7a8n`. Standard castling
uses the king's move, such as `e1g1`.

For a two-move solution, the array normally contains the solver's first move,
the opponent's reply, and the solver's second move. The file represents one
main solution line; branching alternatives can be introduced in a later
format version.

The app validates FENs and move notation, displays the description and turn,
and uses the solution as an exact move sequence. The user enters only the solver
plies (indexes 0, 2, 4, ...). A correct solver move is kept, the following stored
opponent ply is applied automatically, and the app then waits for the next
solver move. A wrong move is restored immediately and does not advance the
solution. Finishing the sequence shows **Solution complete**.

This is solution matching, not general chess-rule enforcement: the app still
does not decide whether an ordinary move is legal, whether a king is in check,
or whether a position is checkmate.

Pawn promotion is implemented. When a pawn is moved to its last rank, the board
does not change until the user chooses **Queen**, **Rook**, **Bishop**, or
**Knight** in the promotion dialog. The selected piece is appended to UCI as
`q`, `r`, `b`, or `n` before solution checking. Cancel leaves the pawn on
its original square. Stored opponent promotion replies are applied
automatically without opening the dialog.

## Missing or invalid files

If `puzzles.json` is missing, the app saves and loads the bundled examples.
If the file is invalid, the app shows an error and displays the examples
without replacing the invalid file. Fix it over USB and relaunch. Existing
`positions.fen` files are left in place but are no longer loaded.

## Lichess imports

The [Lichess puzzle database](https://database.lichess.org/#puzzles) is CC0.
Its CSV `FEN` is **before** the opponent's setup move. Apply the first move of
`Moves` with a chess library, export the resulting FEN, and use all remaining
moves as `solution`. Simply copying the CSV FEN would present the wrong turn
and position. See [example provenance](../examples/README.md).
