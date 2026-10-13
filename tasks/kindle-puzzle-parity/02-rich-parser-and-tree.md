# 02 — Add strict additive-v1 rich puzzle parser and indexed tree

**Dependencies:** 00. **Owner surface:** `src/puzzle.rs`, new platform-neutral `src/analysis.rs` (or small pure module), parser tests. Can proceed independently of 01.

## Deliverable
Keep `Puzzle.solution` required and the only graded sequence. Add optional `topic` (ignore null/non-string), `description_content` and `analysis`. Raise parser raw byte cap to exactly 8 MiB independent of loader Task 01; do not claim large-file device support until 01 completes. Parse each rich analysis node (ID, parent, ordered children, post-move six-field FEN, UCI + SAN, role, comment, structured content, NAGs) into indexed, immutable, Kobo-agnostic structs with stable node handles.

Follow [the pinned Kindle contract](https://github.com/abavelski/kindle-chess-puzzles/blob/80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8/docs/PUZZLE_FORMAT.md#phase-two-rich-analysis-extension) exactly: unique IDs; one root FEN equal to puzzle FEN; connected acyclic bidirectionally consistent graph; nonroot move and parent mandatory; role `main|alternative|sideline` with missing role = sideline; single projected main path UCI exactly equals `solution`; root cannot carry move/role; default absent content to plain comment; explicit empty content authoritative; NAGs nonnegative; reject malformed or dangling move_ref, root target and extra span keys; labels optional and nonempty when given. `description_content` absent falls back to `description`; absent analysis is legal for old books.

Keep validation of UCI/FEN syntax (not chess legality) and robust diagnostics with puzzle ID/context. Never treat plain SAN-looking words as references.

## Automated acceptance
- Accept upstream `valid-rich.json`, `valid-book.json` and legacy `puzzles.json`; reject **every** pinned invalid rich fixture.
- Focused mutations cover wrong analysis version, duplicate/missing/multi-parent/cycle/disconnected links, mismatched main line, illegal root fields, FEN/UCI syntax, empty SAN, role, NAGs, explicit empty spans, missing/empty/extra span fields, UTF-8 topics, fallback text, missing analysis.
- Parser byte boundary tests: 262,144; 262,145; 8,388,608; 8,388,609.
- Old Kobo solution/progress tests remain green; format documentation is updated when integration lands.

**Done:** All rich metadata is validated and accessible without changing solution matching.
