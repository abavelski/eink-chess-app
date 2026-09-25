# E-Ink Chess

A deliberately tiny Rust experiment for using an e-reader as a physical chessboard.

New to Rust? Start with **[Local development setup](docs/LOCAL_SETUP.md)**.

For the Kobo runtime/launch design, see **[Kobo launch architecture](docs/KOBO_ARCHITECTURE.md)**.

To install on a connected Kobo, follow **[USB device installation](docs/DEVICE_INSTALL.md)**.
If the reader already has the older `NickelMenu -> Cobalt -> Chess` build,
follow **[Update to direct launch](docs/DEVICE_UPDATE_DIRECT_LAUNCH.md)**.

The first target is **Kobo Libra H2O** using the [Cobalt](https://github.com/BandarLabs/Cobalt) SDK. There is no chess engine and no chess rules yet.

## MVP

- 8×8 board
- load positions from standard FEN notation
- browse saved positions with the Kobo page-turn buttons
- touch a piece to select it
- touch any square to move it
- touch the selected piece again to deselect it
- moving onto another piece simply replaces it
- reset the board to the current FEN position
- return cleanly to the Kobo reader

That is intentional: version 0.1 behaves like a physical board and position
viewer, not a chess game. It does not enforce legal moves or play turns.

On first launch, the app creates `.adds/cobalt/state/eink-chess/positions.fen`
from the ten positions in [examples/positions.fen](examples/positions.fen).
Edit that file over USB while the app is closed. Put one six-field FEN position
on each line; blank lines and lines starting with `#` are ignored. Forward and
back page-turn buttons move through the list, and **Reset** restores the
position for the current FEN line.

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
b9f46f21af1e209af3c4030e7af90a20c722395c
```

Cobalt provides the 8×8 touch board and partial e-ink refresh planning. This
project adds the public-domain [Sashité Western chess pieces](https://sashite.dev/assets/chess/),
a coordinate frame, and 80%-square piece sizing through its pinned Cobalt fork.
Board lines use a uniform gray rule and the frame around the playable squares
uses a thinner black line.

### Install the Cobalt CLI

Clone Cobalt once and install its development CLI:

```sh
git clone https://github.com/BandarLabs/Cobalt.git
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
