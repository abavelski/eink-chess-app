# 08 — End-to-end shared-file parity and automated release gate

**Dependencies:** 01–07. **Owner surface:** `src/main.rs` integration tests, CI, docs/PUZZLE_FORMAT.md and any Cobalt packaging changes.

## Deliverable
Wire large collection loading, parser, view model, browser, document projection and Cobalt actions together. Ensure loading the **exact same Kindle JSON** (no post-processing) activates valid collections and displays equivalent annotated lines/descriptions. Update Kobo `docs/PUZZLE_FORMAT.md` and `README.md` to explain additive root v1 analysis, 8 MiB cap and its runtime prerequisite, `topic`, structured spans, sources, display and NOTE/Analysis behavior. Preserve collection discovery, filenames, picker errors, active/current IDs, and separate progress.

Add a deterministic end-to-end harness covering rich and legacy books in the simulated Cobalt app. Expand `.github/workflows/ci.yml` to run all new fixtures, `cargo test` and `cargo fmt --all -- --check` (optionally strict Clippy when dependency/toolchain permits); do not depend on physical hardware/USB. If Cobalt patching is needed, test patch reproducibility and pinned build compatibility. Existing screen diagnostics and prior `src/main.rs` tests stay green.

## Automated acceptance
- Load copied Kindle `valid-rich.json` and `valid-book.json` as-is; open Analysis, navigate nested lines/references, preview exact stored FEN, close and solve graded main line; no progress writes caused by browsing.
- Load unchanged legacy Kobo `examples/puzzles.json` and Kindle v1 fixture, preserve solution grading/promotion/picker/flip/free board/reset/progress.
- Invalid fixture in collection picker shows file-specific failure and keeps valid current collection; corrupted/future `progress.v1` remains protected.
- End-to-end large-file read/parse/picker test above old 256 KiB cap; verify deployed data path doesn't rely on an impossible single 8 MiB store frame.
- No game-review picker, small board, new settings panel, or NOTE control. No modifications to uploaded puzzle files.
- Provide a reproducible host gate command list and checked screenshots/layout reports for the Task 09 owner.

**Done:** All host gates pass and the physical checklist can be run without additional design decisions.
