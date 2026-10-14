# Task 08: reproducible host parity gate and Task 09 handoff

These are **host** checks for the exact pinned Cobalt runtime. They do not
substitute for the physical device checkpoint. No e-reader, mounted USB
filesystem, screen capture hardware or edited Kindle fixture is required.

## Commands (repository root)

Use Rust 1.85.1 as pinned in `rust-toolchain.toml` and a Python 3 interpreter:

```sh
cargo test
cargo fmt --all -- --check
rustfmt --edition 2021 --check src/parity_integration_tests.rs
cargo test parity_layout_report_for_task_09 -- --nocapture 2>&1 | tee parity-layout-report.txt

git clone https://github.com/abavelski/Cobalt.git cobalt
git -C cobalt checkout ff4ec9950e178243807082bf818e42c8538d080e
python3 -m py_compile scripts/prepare_cobalt.py scripts/cobalt_collection_range.py
python3 scripts/prepare_cobalt.py cobalt
git -C cobalt diff --binary > /tmp/first-cobalt.patch
find cobalt/examples/eink-chess -type f -print0 | sort -z | xargs -0 sha256sum > /tmp/first-package.sha256
python3 scripts/prepare_cobalt.py cobalt
git -C cobalt diff --binary > /tmp/second-cobalt.patch
find cobalt/examples/eink-chess -type f -print0 | sort -z | xargs -0 sha256sum > /tmp/second-package.sha256
cmp /tmp/first-cobalt.patch /tmp/second-cobalt.patch
cmp /tmp/first-package.sha256 /tmp/second-package.sha256

cargo test --manifest-path cobalt/Cargo.toml -p kobo-policy eink_chess_collection_range
cargo test --manifest-path cobalt/Cargo.toml -p kobod --bin kobod
cargo test --manifest-path cobalt/Cargo.toml -p kobo-eink-chess
cargo check --manifest-path cobalt/Cargo.toml -p kobo-eink-chess
cargo check --manifest-path cobalt/Cargo.toml -p kobod --features device-write
```

The GitHub [CI workflow](../.github/workflows/ci.yml) repeats these checks
against the pinned Cobalt revision and uploads `parity-layout-report.txt` as
the **parity-layout-report** artifact. CI also verifies that both shared Kindle
fixtures are copied byte-for-byte into the prepared Cobalt package. The
preparation script rejects mismatched Cobalt revisions before applying the
read-only range patch. Its second run must not introduce any new changes.

## Covered automated scenarios

`cargo test` runs the unchanged source corpus/hash manifest, all 12 invalid
rich analysis fixtures, the prior Cobalt-app interaction tests, and the Task 08
`parity_*` end-to-end tests.

| Scenario | Host assertion |
| --- | --- |
| Raw Kindle `valid-rich.json` | Collection title, exact bytes, rich prose, alternative node links, stored-FEN previews, wrong promotion rejected, main queen promotion graded, progress only after completion |
| Raw Kindle `valid-book.json` | Picker switch, three puzzles, nested variations and move_refs, each clickable target resolves its own stored FEN; closing restores the live attempt; main line grades, progress stored under the book filename |
| Raw Kobo `examples/puzzles.json` and Kindle v1 `puzzles.json` | Legacy parsing, flip, reset, ungraded Free board, graded move, description and progress |
| Invalid cycle fixture | File-specific picker error, unchanged active file/board/attempt; no progress save |
| Corrupt and future `progress.v1` | No automatic overwrite, even when changing collections |
| Rich book expanded beyond 256 KiB | Full parse/activation only after bounded two-pass 64 KiB range reads; no single large store save/frame |
| Pinned runtime package | All Rust modules, unchanged fixtures, idempotent Cobalt patch, packaged app host tests, Cobalt policy tests and host simulator, device-write compilation |

The large-file fixture in the simulator is **generated from** the rich fixture
in memory, to test the transport boundary. The checked-in Kindle fixture bytes
are never modified. The byte-exact Kindle samples are tested separately.

## Layout report to inspect for Task 09

The `parity_layout_report_for_task_09` test prints machine-readable
`PARITY_LAYOUT` rows with fixture, page state, pixel size, board rectangle,
and `issues=0`. It checks both 1080×1440 at 265 ppi (Libra H2O) and
1264×1680 at 300 ppi (simulated alternate panel) for:

- normal board; first/middle/last Analysis pages with bounded rich links;
- collection picker; Free board; four source books (Kobo legacy, Kindle v1,
  rich single puzzle and multi-puzzle book);
- original full-size chess frame and toolbar controls, with no NOTE,
  game-review picker, miniature board, or new settings action.

Layout assertions fail the test if Cobalt's screen diagnostics find problems.
Existing tests additionally check centered feedback glyphs, promotion modal,
selected-link highlights, original board geometry through reader open/close,
and touch target/link bounds. These reports are **host geometry checks**, not
photographs of an e-ink screen. Save screenshots of these same states as part
of [Task 09](../tasks/kindle-puzzle-parity/09-physical-device-checkpoint.md),
where hardware-specific contrast, legibility, refresh, taps, physical page
keys, and sleep/resume still need to be checked.

## Guardrails

The on-device data directory stays
`.adds/cobalt/state/eink-chess/puzzles*.json`; the reader only adds a
namespaced read-only Cobalt range path, never modifies uploaded collection
files, and never sends an 8 MiB value through the 256 KiB store interface.
`progress.v1` remains separate and protected against corrupt/future versions.
Do not declare Task 09 complete from a green host gate alone.
