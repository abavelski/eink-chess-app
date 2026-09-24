# E-Ink Chess

A deliberately tiny Rust experiment for using an e-reader as a physical chessboard.

New to Rust? Start with **[Local development setup](docs/LOCAL_SETUP.md)**.

To install on a connected Kobo, follow **[USB device installation](docs/DEVICE_INSTALL.md)**.

The first target is **Kobo Libra H2O** using the [Cobalt](https://github.com/BandarLabs/Cobalt) SDK. There is no chess engine and no chess rules yet.

## MVP

- 8×8 board
- normal starting position
- touch a piece to select it
- touch any square to move it
- touch the selected piece again to deselect it
- moving onto another piece simply replaces it
- reset back to the starting position

That is intentional: version 0.1 behaves like a physical board, not a chess game.

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
Cobalt runtime
          |
          v
Kobo Libra H2O
```

Keeping the board model platform-neutral means a future Slint/Kindle frontend can reuse it.

## Cobalt

This project pins the Cobalt SDK to commit:

```text
026ac5561add0157109dd98592272ce9c6eb9343
```

Cobalt already provides the parts this prototype needs: an 8×8 touch board, partial e-ink refresh planning, and built-in monochrome chess piece glyphs.

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

1. Verify the board in the Cobalt simulator.
2. Deploy it to the Libra H2O and check touch coordinates / refresh quality.
3. Add board flipping.
4. Replace the physical-board model only when needed with chess rules/FEN/PGN support.
5. Add a Slint frontend for desktop + Kindle while reusing the pure Rust board core.
