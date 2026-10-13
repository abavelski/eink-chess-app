# 00 — Freeze shared Kindle puzzle contract and corpus

**Dependencies:** None. **Owner surface:** `tests/fixtures/kindle-parity/`, new host tests and references. Avoid app/UI changes.

## Deliverable
Record the reference Kindle commit `80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8` and the normative spec `docs/PUZZLE_FORMAT.md` at that commit. Copy a **small, exact, attributed** corpus from the Kindle repo: `tests/fixtures/puzzles.json`, `tests/fixtures/rich-analysis/valid-rich.json` and every `invalid-*.json` in that directory. Include rich fixtures from `tests/fixtures/pgn-converter/valid-book.json` for complex nested variations. Do not invent a competing Kobo schema. Add a manifest with upstream paths/revision and byte hashes so a future upstream change is deliberate.

Keep root `version: 1`, required legacy `solution`, additive `topic`, `description_content` and `analysis: {version:1, root, nodes}`. Pin significant ordering/defaults, exact main-path/solution equality, FEN/SAN/UCI, NAGs, move-ref target requirements and fallback projection. Document 8 MiB raw-file policy (not 256 KiB).

## Automated acceptance
- Corpus manifest test checks fixture byte hashes and expected fixture names; valid/invalid labels are table-driven.
- Add a minimal consumer smoke test showing raw fixture bytes are available in normal host `cargo test` (full validation belongs to 02).
- Legacy fixtures remain unchanged; fixture imports cannot silently overwrite them.
- No generated puzzle file, device file or progress record is modified.

**Done:** Reproducible test corpus and clear contract citation committed; no Kobo runtime implementation claimed.
