# 05 — Generate the same inline PGN-like analysis document

**Dependencies:** 02. **Owner surface:** pure text/fragment flow module and tests. Can run in parallel with 03/04.

## Deliverable
Project the validated ordered analysis tree and optional `description_content` into deterministic, UI-independent text runs, each with origin (plain prose, tree move, or explicit move_ref) and its target node handle. Mirror Kindle's current `crates/chess-render/src/analysis_panel.rs` behavior:
- Emit description preface (once, if nonempty), root comment/content, then moves.
- At each position the **first ordered child** continues the line; subsequent ordered children appear as parenthesized recursive variations before continuing the first child's descendants. Role does not reorder presentation; use iterative traversal to avoid stack overflow.
- Render SAN, FEN-based fullmove number and `1.`/`1...` correctly for white/black starts and after variations/comments, conventional adjacent white/black pairs, Unicode prose, and NAGs: 1–6 as `! ? !! ?? !? ?!`; unknown NAG as `$N`.
- Comments are *book-style plain prose*, without invented braces; literal authored braces remain.
- Only explicit `move_ref` spans and actual tree SAN moves are interactive; unlabeled refs display target SAN, labels override it. Do not regex-link move-looking text. Preserve ordering and all text through line breaks/pagination.

## Automated acceptance
- Golden semantic-token tests using `valid-rich.json`, `valid-book.json`, black-to-move and promotion cases, siblings/nested RAVs, NAGs, long words, Cyrillic/Unicode and duplicate same-label move refs with different targets.
- Every move and explicit reference appears once with correct node target; plain move-looking text has no target.
- Empty/legacy descriptions produce sane flow without fake tree moves.
- Stress deep/wide trees with a bounded/iterative traversal and no recursion overflow.

**Done:** UI can paginate/render the entire analysis without reimplementing PGN semantics.
