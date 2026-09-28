# E-Ink Chess

A deliberately tiny Rust experiment for using an e-reader as a physical chessboard.

New to Rust? Start with **[Local development setup](docs/LOCAL_SETUP.md)**.

For the Kobo runtime/launch design, see **[Kobo launch architecture](docs/KOBO_ARCHITECTURE.md)**.

For sleep findings, fixes, and the remaining early-touch issue, see
**[Sleep troubleshooting](docs/SLEEP_TROUBLESHOOTING.md)**.

To install on a connected Kobo, follow **[USB device installation](docs/DEVICE_INSTALL.md)**.
If the reader already has the older `NickelMenu -> Cobalt -> Chess` build,
follow **[Update to direct launch](docs/DEVICE_UPDATE_DIRECT_LAUNCH.md)**.

The first target is **Kobo Libra H2O** using the [Cobalt](https://github.com/BandarLabs/Cobalt) SDK. There is no chess engine or legal-move enforcement; puzzle correctness is checked against the stored UCI solution line.

## MVP

- 8×8 board
- load multiple puzzle collections with FENs, descriptions, and solutions from JSON
- switch collections from the on-device **Puzzles** picker
- browse puzzles in the active collection with the Kobo page-turn buttons
- remember the active collection, each collection's current puzzle, and solved puzzles
- show an explicit **Solved** marker when revisiting completed puzzles
- face the board toward the side to move, with a manual Flip control
- touch a piece to select it
- touch a square to attempt the next solution move
- show a small Correct hint or a centered thumbs result over the board
- automatically apply stored opponent replies between solver moves
- reject a wrong move and restore the previous position
- choose Queen, Rook, Bishop, or Knight when a pawn reaches the last rank
- switch between graded **Solution** mode and an ungraded **Free board**
- touch the selected piece again to deselect it
- reset the board and solution attempt to the current FEN position
- return cleanly to the Kobo reader

The app is still intentionally not a chess game or engine. In **Solution**
mode it does not enforce legal chess moves; it only checks whether the attempted
move matches the next stored UCI move and applies the stored opponent reply.
**Free board** mode turns the current position into a physical scratch board:
moves are not graded and do not advance the solution. The leftmost grid icon in
the three-button toolbar toggles this mode; its gray fill shows when Free board
is active. Tapping it again returns to Solution, restores the puzzle FEN, and
restarts the attempt while preserving the current board orientation.

Place UTF-8 puzzle collections in `.adds/cobalt/state/eink-chess/` on the
mounted reader while the app is closed. The default collection is
`puzzles.json`; additional collections use names such as
`puzzles-endgames.json` or `puzzles-tactics.json`. The on-device **Puzzles**
button opens a picker and uses each file's optional `title` as its display name.
The [puzzle file format](docs/PUZZLE_FORMAT.md) documents the exact naming and
fields. If no puzzle collection exists, the app creates the ten CC0 Lichess
examples as `puzzles.json`.

Page-turn buttons browse puzzles inside the active collection. **Puzzles**
switches collections and restores the last puzzle visited in each collection.
The active collection and per-file positions are restored after restarting the
app. **Reset** restores the current puzzle's FEN and restarts its solution;
**Flip** changes only the viewing side. Each newly selected puzzle automatically faces
the color whose turn is recorded in the FEN.
Solver moves are checked against the stored UCI line. Correct moves are accepted,
stored opponent replies are applied automatically, and the turn line adds
**Correct** until the next move. A wrong move is restored with a centered thumbs
down; completing the solution shows a centered thumbs up. **Reset** removes
either icon. Completed puzzle IDs are stored
separately in `progress.v1`; uploaded puzzle JSON files are never modified.
The old `positions.fen` file is no longer read.

## Architecture

```text
src/board.rs
    pure Rust board state
    no Kobo / Cobalt dependency
          |
          v
src/main.rs
    thin Cobalt adapter
    board rendering + touch actions
          |
          v
Cobalt runtime (kobod)
          |
          v
Kobo Libra H2O
```

On the device, the normal owner-facing path is:

```text
NickelMenu -> E-Ink Chess -> chessboard
```

The Cobalt launcher is skipped. Cobalt remains underneath as the runtime that
takes over/restores Nickel, handles touch/framebuffer access, and performs
e-ink rendering.

Keeping the board model platform-neutral means a future Slint/Kindle frontend can reuse it.

## Cobalt

This project pins the Cobalt SDK to commit:

```text
ff4ec9950e178243807082bf818e42c8538d080e
```

Cobalt provides the 8×8 touch board and partial e-ink refresh planning. This
project adds the public-domain [Sashité Western chess pieces](https://sashite.dev/assets/chess/),
a coordinate frame, and 80%-square piece sizing through its pinned Cobalt fork.
Board lines use a uniform gray rule and the frame around the playable squares
uses a thinner black line.

### Install the Cobalt CLI

Clone Cobalt once and install its development CLI:

```sh
git clone https://github.com/abavelski/Cobalt.git
cd Cobalt
cargo install --path crates/kobo-cli
```

Cobalt currently uses Rust 1.85.1; this repository pins the same toolchain.

### Run the simulator

From this repository:

```sh
cargo test
kobo dev
```

The simulator is the quickest place to verify board sizing and touch behavior before deploying to the reader.

> The Cobalt SDK is AGPL-3.0 licensed. If this project moves beyond a personal prototype, review the distribution/licensing implications before choosing the final application platform.

## Next milestones

1. Add board flipping.
2. Keep the direct NickelMenu launch/update path reproducible.
3. Add optional chess rules and PGN support.
4. Add a Slint frontend for desktop + Kindle while reusing the pure Rust board core.
5. Optionally explore a fully standalone Kobo backend after the study UI is mature.
