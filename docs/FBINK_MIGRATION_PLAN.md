# FBInk migration plan for Kobo Libra H2O

This plan replaces the current Cobalt runtime/UI path with a small standalone Kobo application using FBInk, while preserving the current E-Ink Chess behavior.

The migration is intentionally incremental. Until the standalone path has passed the on-device checkpoints, the existing Cobalt implementation must remain buildable and installable.

## Target behavior

The FBInk version is considered functionally equivalent when it can:

- launch directly from NickelMenu;
- show the current 8x8 board with coordinates and Sashité pieces;
- load the existing list of six-field FEN positions;
- use the physical page-turn buttons to move backward and forward through positions;
- select, deselect, and freely move pieces by touch;
- reset the current position;
- return cleanly to Nickel;
- recover back to Nickel if the application exits with an error;
- avoid obviously bad ghosting during normal use.

This is still a physical-board/position-viewer application. Chess legality, PGN, puzzle solving, networking, Kindle support, and general multi-device abstractions are out of scope for this migration.

## Intended end state

~~~text
Nickel
  |
  | NickelMenu: E-Ink Chess
  v
.adds/eink-chess/start.sh
  |
  | stop/handoff Nickel safely
  v
eink-chess
  |
  +-- app/session state
  +-- platform-neutral board model
  +-- platform-neutral layout + hit testing
  +-- Gray8 software renderer
  +-- Kobo input adapter
  +-- small FBInk adapter
  |
  v
FBInk
  |
  v
Kobo framebuffer / e-ink controller

on exit or crash:
start.sh -> restore/restart Nickel
~~~

The application should own its UI pixels. FBInk should be treated primarily as the hardware/display boundary, not as the application UI framework.

## Rules for Codex tasks

Each numbered task below is a separate implementation unit. Do not combine adjacent tasks unless the earlier task is already present on master.

For every task:

1. Read this document and the current repository before editing.
2. Preserve the current Cobalt path unless the task explicitly says to remove it.
3. Keep src/board.rs free of Kobo, Cobalt, FBInk, evdev, and filesystem types.
4. Prefer small modules with unit tests over a large replacement main.rs.
5. Run cargo fmt and cargo test before finishing.
6. Run cargo clippy --all-targets --all-features -- -D warnings when the dependency/toolchain state permits it.
7. Do not claim hardware behavior is verified unless a human has tested it on the Libra H2O.
8. If a task reaches a HUMAN CHECKPOINT, stop after producing the requested build/deploy instructions and report exactly what the human needs to verify.
9. Do not delete the working Cobalt implementation until Task 14.
10. Do not track generated build directories or a mutable checkout of FBInk.

Prefer one commit per task with a message beginning with fbink:.

## Upstream references and constraints

Use FBInk from its official repository:

https://github.com/NiLuJe/FBInk

Pin a released FBInk tag rather than following master. At the time this plan was written, v1.25.0 is the current release. Before implementing the FBInk integration task, verify that the selected release still supports Kobo Libra H2O and record both the tag and commit in the repository.

FBInk documents Kobo support, raw grayscale/image handling, rectangular refreshes, input-device discovery utilities, and minimal feature builds. Its own README also recommends using a Kobo-appropriate cross toolchain rather than assuming an arbitrary Linux ARM toolchain is ABI-compatible.

Useful references:

- FBInk README and build notes: https://github.com/NiLuJe/FBInk/blob/master/README.md
- FBInk public API: https://github.com/NiLuJe/FBInk/blob/master/fbink.h
- KOReader Kobo launcher/lifecycle reference: https://github.com/koreader/koreader/blob/master/platform/kobo/koreader.sh
- KOReader Kobo input reference: https://github.com/koreader/koreader/blob/master/frontend/device/kobo/device.lua

FBInk is GPL-3.0-or-later. Before publishing binaries, make sure the repository's licensing and distribution approach is compatible. Do not treat copying or statically linking FBInk as a way around its license.

---

## Task 1 - Freeze and test the current application state

### Goal

Create a clean behavioral baseline before changing architecture.

### Work

Add tests around behavior that currently lives in src/main.rs but is independent of Cobalt:

- parsing a positions.fen file;
- ignoring blank/comment lines;
- rejecting invalid FEN with the correct line number;
- forward/back navigation boundaries;
- reset behavior;
- current-position index.

Move parse_position_file out of src/main.rs into a small platform-neutral module such as src/positions.rs if necessary to make this testable.

Do not change the screen or deployment behavior.

### Acceptance criteria

- cargo test passes.
- The Cobalt application still builds exactly as before.
- No FBInk dependency has been added.
- src/board.rs remains unchanged except for tests/refactors that are genuinely board-model concerns.

### Suggested commit

fbink: extract and test position file logic

---

## Task 2 - Extract a platform-neutral application session

### Goal

Remove application state transitions from the Cobalt adapter without changing behavior.

### Work

Create a module such as src/session.rs that owns:

- Board;
- loaded FEN strings;
- current index;
- current file/load error state.

Give it explicit operations such as:

- load_positions;
- load_examples;
- tap_square;
- reset;
- previous_position;
- next_position.

The session must not know about Context, Screen, ActionId, KoboApp, files, FBInk, or evdev.

Refactor the existing Cobalt main.rs to call this session object.

### Acceptance criteria

- Existing behavior remains the same under Cobalt.
- Session behavior is covered with unit tests.
- main.rs is noticeably thinner.
- cargo test passes.

### Suggested commit

fbink: extract platform-neutral chess session

---

## Task 3 - Add platform-neutral screen geometry and hit testing

### Goal

Define the chess screen without Cobalt layout primitives.

### Work

Create a module such as src/ui/layout.rs with tiny geometry types:

- Size;
- Point;
- Rect;
- ChessLayout.

Compute the layout from a supplied screen width and height rather than scattering Libra-specific constants.

The layout should define at least:

- title/status area;
- reset hit target;
- playable 8x8 board rectangle;
- the 64 square rectangles;
- coordinate-label margins;
- return/exit hit target.

Add methods to map a screen point to a semantic target, for example:

~~~text
HitTarget::Square(0..63)
HitTarget::Reset
HitTarget::Exit
HitTarget::None
~~~

Use 1264 x 1680 in unit tests as the Libra H2O reference size, but do not make the layout implementation require exactly that size.

Do not switch the Cobalt renderer yet.

### Acceptance criteria

- Unit tests cover corners, square centers, boundary conditions, reset, and exit.
- The board is square and entirely inside the supplied screen size.
- No Kobo/FBInk types appear in the layout module.
- cargo test passes.

### Suggested commit

fbink: add screen layout and hit testing

---

## Task 4 - Add a tiny Gray8 software canvas

### Goal

Create an application-owned framebuffer representation that can be tested on the host.

### Work

Add a renderer module with an 8-bit grayscale surface:

~~~text
0   = black
255 = white
~~~

Support only the primitives needed by this application:

- clear/fill rectangle;
- horizontal/vertical line;
- blit an 8-bit image/mask;
- a minimal bitmap text path for the small amount of UI text;
- clipping.

Avoid introducing a general GUI framework.

Add tests for clipping, fills, line drawing, and blitting. If useful, add a host-only helper that writes a PGM file for visual inspection; do not make PNG/JPEG decoding a runtime requirement.

### Acceptance criteria

- Renderer runs in normal host cargo test.
- It has no Cobalt or FBInk dependency.
- A test can produce a complete 1264 x 1680 Gray8 frame in memory.
- The runtime API is simple enough to pass its pixel buffer to a display backend later.

### Suggested commit

fbink: add Gray8 software renderer

---

## Task 5 - Render a complete chess screen on the host

### Goal

Prove that all application pixels can be generated without Cobalt.

### Work

Use ChessLayout plus the Gray8 canvas to render:

- board squares;
- board frame;
- file/rank coordinates;
- selected-square treatment;
- title with position index;
- reset control;
- return-to-reader control.

For this task, chess pieces may use deliberately simple temporary glyphs or letters. Do not block the framebuffer migration on final Sashité artwork.

Add a host command/example that writes a PGM snapshot for a known FEN.

### Acceptance criteria

- Running the host renderer produces a visually inspectable complete screen.
- Selection changes only the expected square styling.
- Layout and hit testing use the same geometry.
- No device access is required.

### Suggested commit

fbink: render chess screen into Gray8 buffer

---

## Task 6 - Pin FBInk and establish a reproducible Kobo build

### Goal

Make the repository able to build a tiny FBInk-linked Kobo probe without touching the real application.

### Work

Pin a released FBInk revision. Prefer a git submodule under third_party/FBInk or another explicit source pin that makes the exact upstream revision auditable. Do not track an unpinned clone of master.

Start with the smallest FBInk feature set that supports the chosen display path and input discovery. A likely starting point is a minimal build plus drawing/image/input features, but verify against the pinned release instead of copying flags blindly.

Create the build configuration needed for Kobo Libra H2O. It is acceptable to use a containerized Kobo/KOReader-style cross toolchain if that is more reproducible on macOS than a host-installed compiler.

Important: do not spend hours forcing the existing armv7-unknown-linux-musleabihf Cobalt target to link against an incompatible C library. If FBInk's Kobo toolchain requires a GNU Kobo target/sysroot, introduce a separate standalone build target for the FBInk binary while leaving the Cobalt build untouched.

Add a very small safe Rust wrapper around only the FBInk calls the project needs. Prefer either:

- a narrow C bridge with a stable project-owned interface; or
- pinned generated bindings whose FBInk version exactly matches the linked library.

Do not expose the full FBInk C API throughout the application.

### Probe API target

The Rust side should ultimately need operations roughly equivalent to:

~~~text
open/init
screen information
present Gray8 pixels
refresh full or rectangular region
close
input device discovery, if used
~~~

Exact FBInk functions and structs must come from the pinned header.

### Acceptance criteria

- There is a documented one-command or short-command cross build for a Kobo probe.
- The FBInk revision is pinned and recorded.
- The normal host cargo test still works without requiring a Kobo device.
- The existing Cobalt binary still builds.
- No application behavior has switched to FBInk yet.

### Suggested commit

fbink: pin library and add Kobo probe build

---

## Task 7 - Display-only FBInk probe

### Goal

Verify the framebuffer/toolchain boundary on the physical Libra H2O before adding input or lifecycle complexity.

### Work

Add a dedicated fbink-probe binary that:

1. opens and initializes FBInk;
2. logs detected screen/device state;
3. displays a simple full-screen test pattern or the host-rendered chess screen;
4. performs one explicit full refresh;
5. closes cleanly.

Provide a deploy helper that copies only the probe and its required runtime files to a temporary directory on the mounted Kobo.

Do not stop Nickel automatically yet. The instructions must clearly state the safe/manual environment required to run this probe. If running while Nickel owns the display is unsafe or produces misleading results, say so and make the probe launch procedure use an established safe handoff path rather than improvising.

### HUMAN CHECKPOINT A

A human must verify on the Libra H2O:

- correct orientation;
- correct width/height;
- no cropping;
- black/white polarity is correct;
- the device remains recoverable after the probe exits.

Record the observed FBInk state and any required rotation/viewport settings in docs/FBINK_DEVICE_NOTES.md.

Do not continue to Task 8 until this checkpoint is recorded.

### Suggested commit

fbink: add Libra H2O display probe

---

## Task 8 - Add Kobo touch and page-button input probe

### Goal

Identify the correct input devices and normalize their events before connecting them to chess behavior.

### Work

Use FBInk input discovery where practical, and use a small Linux evdev reader for actual events. Prefer a pure Rust evdev implementation/crate that does not add another device C library unless there is a clear reason.

Create an input abstraction which emits semantic events such as:

~~~text
PointerDown(Point)
PointerUp(Point)
PageForward
PageBackward
ExitRequested
~~~

Do not let the session know Linux event codes or device paths.

The input probe should log:

- selected device names/paths;
- raw touch coordinates;
- normalized display coordinates;
- page-button direction.

Do not hardcode /dev/input/eventN unless a documented Libra H2O fallback is required. Device numbering can change.

### HUMAN CHECKPOINT B

A human must tap all four screen corners, several board squares, and both page-turn buttons.

Verify:

- X/Y are not swapped;
- rotation is correct;
- touches map to the displayed squares;
- forward/back buttons have the intended direction;
- no duplicate press/release handling causes double moves.

Record the final transform/device-selection rules in docs/FBINK_DEVICE_NOTES.md.

Do not continue to the integrated app until this checkpoint passes.

### Suggested commit

fbink: add Libra H2O input probe

---

## Task 9 - Run the chess interaction loop on FBInk

### Goal

Make the standalone FBInk binary interactive while still using temporary piece artwork if needed.

### Work

Create a standalone application loop:

~~~text
read input
  -> map to HitTarget
  -> update Session
  -> render Gray8 frame
  -> present/refresh with FBInk
~~~

Implement:

- square tap select/deselect/move;
- reset;
- previous/next position via physical buttons;
- exit event.

For the first integrated version, correctness is more important than refresh efficiency. A full-screen refresh after each visible state change is acceptable temporarily.

Keep the Cobalt executable/path available.

### Acceptance criteria

On host:

- event-to-session behavior is unit tested;
- rendering tests still pass.

On device:

- user can select and move a piece;
- reset works;
- both page buttons navigate positions;
- exit leaves the standalone process cleanly.

### Suggested commit

fbink: integrate interactive standalone chess loop

---

## Task 10 - Move persistence out of the Cobalt store

### Goal

Preserve the existing editable positions.fen workflow without kobod.

### Work

Use a project-owned directory, preferably:

~~~text
/mnt/onboard/.adds/eink-chess/
~~~

Store the editable positions file at a documented location under that directory.

Migration behavior:

1. If the new positions file exists, use it.
2. Otherwise, if the old Cobalt file exists at .adds/cobalt/state/eink-chess/positions.fen, copy/import it once.
3. Otherwise create the new file from the bundled examples.

Make paths injectable/configurable in tests so host tests never write to /mnt/onboard.

Preserve the current error behavior: invalid user data should show an error and fall back to bundled examples without destroying the user's bad file.

### Acceptance criteria

- Unit tests cover new file, old-file migration, invalid UTF-8, invalid FEN, and no-file cases.
- No Cobalt store API is used by the standalone binary.
- Existing user positions survive the migration path.

### Suggested commit

fbink: add standalone positions persistence

---

## Task 11 - Restore the Sashité presentation

### Goal

Reach visual parity after the hardware path is already working.

### Work

Move Sashité rendering into this repository instead of patching Cobalt.

Prefer build-time/pre-generated grayscale assets so the Kobo runtime does not need an SVG parser. A good design is:

- keep the original public-domain SVG sources as the source of truth;
- use a host-side generator to rasterize the 12 pieces at the required canonical size;
- commit or reproducibly generate compact grayscale/alpha assets;
- embed those assets in the Rust binary;
- scale/blit only if required by layout.

Keep the current visual intent:

- light/dark Sashité layers;
- approximately 80 percent of a square;
- coordinate frame;
- selected-square indication;
- uniform board rules and thin outer frame.

Do not copy the old Cobalt renderer wholesale. Reuse artwork/data, not Cobalt UI architecture.

### Acceptance criteria

- Host snapshot visually matches the current Cobalt design closely.
- Runtime has no SVG parsing dependency unless measurement proves it is worthwhile.
- The old Cobalt patch is still retained for fallback at this stage.

### Suggested commit

fbink: render Sashite pieces without Cobalt

---

## Task 12 - Add safe Nickel handoff and crash recovery

### Goal

Launch the standalone app from NickelMenu and always return control to Nickel.

### Work

Add .adds/eink-chess/start.sh plus the NickelMenu entry.

Do not invent lifecycle commands from memory. Study the current KOReader Kobo launcher and the Libra H2O behavior, then implement only the subset required for this application. Document every process/state change the script makes.

The launcher must:

- detect whether Nickel is running;
- flush relevant storage before stopping the reader stack;
- stop only the processes necessary to release framebuffer/input ownership;
- wait with a finite timeout;
- launch the chess binary;
- install shell traps so normal exit, nonzero exit, SIGINT, and SIGTERM all go through cleanup;
- restore any framebuffer rotation/bit-depth state that this project changed;
- restart Nickel when appropriate;
- write a small log file useful for recovery.

Do not add power-management, Wi-Fi, Bluetooth, CPU-governor, or unrelated KOReader behavior unless the chess app demonstrably needs it.

Keep a documented manual recovery route if the launcher fails.

### HUMAN CHECKPOINT C

Test all of these on the device:

1. normal launch and Return to Kobo reader;
2. app process killed intentionally;
3. app returns a nonzero status;
4. repeated launch/exit at least five times;
5. USB connection still behaves normally after returning to Nickel.

Do not replace the main menu entry until these pass.

### Suggested commit

fbink: add safe Nickel lifecycle launcher

---

## Task 13 - Add standalone build and USB deploy workflow

### Goal

Make the FBInk path as reproducible as the current Cobalt deployment.

### Work

Add a new script such as scripts/build_deploy_fbink_kobo.sh.

It should:

- validate the mounted volume is a Kobo;
- build the pinned FBInk dependency/toolchain deterministically;
- build the standalone Rust binary;
- run host tests first;
- copy the application, required FBInk runtime files, assets, start script, and NickelMenu config;
- verify checksums after copying;
- preserve unrelated NickelMenu entries;
- not eject until all validation succeeds.

During this task, keep both menu entries available and clearly labeled, for example:

~~~text
E-Ink Chess (FBInk)
E-Ink Chess (Cobalt fallback)
~~~

Do not overwrite or uninstall the fallback yet.

### Acceptance criteria

- A fresh checkout plus documented toolchain can build the standalone package.
- Re-running deploy is idempotent.
- Failed build/copy does not leave a half-written primary menu entry.
- Device can launch either backend for comparison.

### Suggested commit

fbink: add standalone Kobo deploy workflow

---

## Task 14 - Add e-ink damage tracking and refresh policy

### Goal

Replace the correctness-first full refresh behavior with a simple predictable e-ink policy.

### Work

Introduce dirty rectangles at the application-renderer boundary.

At minimum distinguish:

- full first frame;
- one-square selection/deselection;
- two-square piece move;
- title/index change after page turn;
- reset/full board change.

Start conservatively. Use FBInk's supported rectangular refresh API and a grayscale-friendly waveform appropriate for Libra H2O. Do not guess waveform names from another device: derive the choice from the pinned FBInk documentation and on-device testing.

Add an occasional full refresh if required to control ghosting.

Instrument timings in a debug build:

- process start;
- FBInk initialized;
- first frame submitted;
- first frame complete if the platform supports a reliable wait;
- per-interaction render time.

### Acceptance criteria

- Selection does not require refreshing unrelated UI regions.
- Normal piece moves have no unacceptable ghosting after repeated use.
- Full refresh remains available for launch/reset/recovery.
- Timing logs make startup and interaction costs measurable rather than anecdotal.

### Suggested commit

fbink: add damage-based e-ink refresh policy

---

## Task 15 - Make FBInk the default and remove Cobalt integration

### Goal

Only after the standalone implementation has proven stable, remove the old runtime dependency.

### Preconditions

Do not execute this task until Checkpoints A, B, and C have passed and the user has explicitly decided the FBInk version is the new default.

### Work

Remove:

- kobo-sdk dependency from Cargo.toml;
- Cobalt-only main adapter code;
- scripts/prepare_cobalt.py;
- patches/cobalt-ui.patch;
- Cobalt-specific build/deploy code that is no longer needed;
- device documentation that instructs the user to install a patched Cobalt package.

Preserve useful historical context in docs rather than leaving misleading active instructions.

Update:

- README architecture;
- install/update documentation;
- recovery instructions;
- license/third-party notices;
- locations of positions.fen;
- build/deploy commands.

If the old Cobalt state directory is left on existing devices, do not delete it automatically. The import path from Task 10 should make it harmless.

### Acceptance criteria

- cargo tree contains no Cobalt/kobo-sdk dependency.
- Searching the active code for kobo_sdk, kobod, and prepare_cobalt finds no runtime/build dependency.
- Fresh install uses only the standalone path.
- Existing positions migrate automatically.
- Return to Nickel and crash recovery are re-tested once more.

### Suggested commit

fbink: remove Cobalt runtime integration

---

## Task 16 - Measure the result and decide whether to optimize further

### Goal

Determine whether the migration achieved the practical benefits that motivated it.

### Work

Record a small before/after table in docs/FBINK_RESULTS.md.

Measure on the same Libra H2O:

- installed files attributable to the chess app/runtime;
- chess executable size;
- time from selecting the NickelMenu item to first usable board;
- time from Exit to usable Nickel home screen;
- selection response;
- piece-move response;
- page-turn response.

Also record qualitative observations about ghosting and launch reliability.

Do not optimize based only on binary size if launch handoff dominates perceived latency.

### Acceptance criteria

The repository contains enough measured data to answer:

- Did removing Cobalt reduce installed size?
- Did it reduce launch/exit latency?
- Is interaction faster or merely different?
- Is the additional lifecycle/input maintenance burden acceptable?

### Suggested commit

fbink: document migration performance results

---

## Recommended execution order

Execute Tasks 1 through 7 without changing the user's default Kobo installation.

After HUMAN CHECKPOINT A, execute Task 8 and perform HUMAN CHECKPOINT B.

Tasks 9 through 11 produce feature parity while Cobalt remains a fallback.

Task 12 is the highest-risk part of the migration because it assumes responsibility for Nickel lifecycle and recovery. Treat HUMAN CHECKPOINT C as a release gate.

Task 13 makes side-by-side testing easy. Task 14 should be driven by observed e-ink behavior, not premature optimization.

Only then execute Task 15.

Task 16 closes the experiment with measured results.

## Definition of done

The migration is done when the app can be built from this repository, launched directly from NickelMenu on the Kobo Libra H2O, render and interact with the chessboard using FBInk without Cobalt installed in its runtime path, preserve the user's FEN file, and reliably restore Nickel after both normal exit and failure.

The important architectural result is not merely removing one dependency. It is that chess state, layout, hit testing, and rendering belong to this project, while the Kobo-specific boundary is small enough to replace or reuse later.
