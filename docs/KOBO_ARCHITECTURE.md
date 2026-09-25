# Kobo launch architecture

E-Ink Chess deliberately separates the **chess/study application** from the
**Kobo hardware integration**. The chess code in `src/board.rs` does not know
about Cobalt, NickelMenu, the framebuffer, or touch devices.

This document records the two Kobo architectures that matter for the project:
the architecture used now, and the lower-level architecture that can be explored
later as a learning exercise.

## Architecture A — direct NickelMenu app, Cobalt runtime underneath

This is the current architecture.

```text
Nickel
  |
  | NickelMenu: "E-Ink Chess"
  v
.adds/cobalt-launch.sh
  |
  v
Cobalt start.sh
  |
  | kobod --present kobo-eink-chess
  v
kobod
  |
  +-- stops/starts Nickel safely
  +-- owns the framebuffer during the session
  +-- reads touch input
  +-- applies device/orientation profiles
  +-- renders Cobalt UI
  +-- plans e-ink refreshes
  +-- supervises the app process
  |
  v
kobo-eink-chess
  |
  +-- src/main.rs: Kobo/Cobalt UI adapter
  +-- src/board.rs: pure Rust board model
```

From the reader's point of view there is no Cobalt launcher step:

```text
NickelMenu -> E-Ink Chess -> chessboard
```

Cobalt is still installed because the current Rust application is a `kobo-sdk`
client. `kobo_sdk::run(...)` communicates with `kobod`; the app is not a
standalone framebuffer program.

A short Cobalt takeover splash may still be visible while `kobod` stops Nickel
and takes ownership of the panel. That is runtime handoff, not the Cobalt app
launcher.

### Why keep Cobalt for now?

It gives the project a working, tested Kobo hardware layer while the learning
focus stays on Rust and the chess/study application. In particular, Cobalt
currently supplies:

- Kobo framebuffer ownership and restoration;
- touch input and coordinate mapping;
- e-ink refresh planning;
- device and orientation profiles;
- application lifecycle and supervision;
- safe handoff back to Nickel;
- Cobalt screen/layout primitives;
- the renderer used for the chess-specific SVG artwork;
- simulator and device tooling.

The app now has a pinned **Return to Kobo reader** action. It calls
`Context::exit()`. Because Chess is the root application passed to
`kobod --present`, closing it ends the panel session and Cobalt restores Nickel.

### Repository integration

The application repository itself stays independent of a modified Cobalt fork.
`scripts/prepare_cobalt.py` applies the small integration to the pinned Cobalt
checkout before a device package is built. At the pinned Cobalt revision it:

1. copies `src/main.rs`, `src/board.rs`, and the bundled FEN examples into a
   Cobalt workspace example;
2. registers `kobo-eink-chess` in the device package;
3. registers the app metadata used by Cobalt;
4. changes Cobalt's packaged `start.sh` root app from `kobo-launcher` to
   `kobo-eink-chess`;
5. changes the generated NickelMenu label from `Cobalt` to `E-Ink Chess`.

The stable `.adds/cobalt-launch.sh` bootstrap is intentionally retained. It is
part of Cobalt's safe installation/recovery path and avoids duplicating that
logic in this project.

### Chess presentation patch

Cobalt still supplies the board layout, vector rasterizer, touch handling, and
e-ink refresh infrastructure. The integration script applies the project's
chess presentation patch inside the pinned Cobalt checkout. It renders the
12 Sashité Western SVG pieces with their original light and dark layers,
keeps each piece at 80% of its square, and adds a frame with file and rank
coordinates. The playable squares are sized to the panel and the lines use a
uniform gray rule; a thinner black rule surrounds the board. The 64 squares
retain their touch actions. The source SVGs and their generated vector paths
live in `assets/sashite-western`; the integration script applies
`patches/cobalt-ui.patch`. `src/board.rs` remains pure board state and parses
standard six-field FEN positions.

The app's FEN list is kept in Cobalt's durable per-app store as
`.adds/cobalt/state/eink-chess/positions.fen`. The first launch copies the ten
bundled positions there. The file contains one FEN per line and can be edited
over USB while the app is closed. The reader's physical page-turn buttons move
through the saved positions; **Reset** restores the current line's FEN.

## Architecture B — fully standalone Kobo application

This is a possible future learning project, not the current implementation.

```text
Nickel
  |
  | NickelMenu: "E-Ink Chess"
  v
our launch script
  |
  v
standalone eink-chess Kobo executable
  |
  +-- Nickel handoff/recovery
  +-- framebuffer access
  +-- touch device access
  +-- orientation/device profiles
  +-- e-ink update ioctls/waveforms
  +-- refresh/ghosting policy
  +-- power/watchdog handling
  |
  v
Kobo hardware
```

Here Cobalt is removed from the device/runtime path entirely. The application
would need its own Kobo platform crate or backend.

This can eventually be attractive because it gives complete control over the UI
and makes the Kobo frontend conceptually closer to the future Kindle frontend.
It is also substantially more work: framebuffer/touch code is only a small part
of a reliable e-reader application. Safe Nickel shutdown/restart, panel
restoration after crashes, model-specific transforms, refresh policy, and power
handling all become this project's responsibility.

## What remains shared in either architecture

The intended long-term boundary does not change:

```text
pure Rust core
    |
    +-- board/chess state
    +-- PGN parsing
    +-- study document model
    +-- comments/variations
    |
    +-- Kobo UI/platform adapter
    +-- Kindle/Desktop UI adapter
```

`src/board.rs` should therefore remain free of Cobalt-specific types even if the
Kobo platform layer is replaced later.

## When to consider Architecture B

Do not switch merely because one Cobalt UI primitive is visually unsuitable.
Piece artwork, board styling, and study layout can be improved while retaining
the runtime.

Architecture B becomes a useful experiment when the goal itself is to learn the
Kobo platform layer, or when Cobalt's application protocol prevents a UI or
hardware behavior the study app genuinely requires.
