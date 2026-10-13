# 07 — Wire Cobalt text/move taps to exact FEN previews

**Dependencies:** 04, 06. **Owner surface:** `src/main.rs` action dispatch and action-target mapping; host Cobalt AppRunner tests.

## Deliverable
Attach distinct stable action IDs to every interactive **tree move** and **explicit prose move_ref**, even repeated visible SAN labels. Resolve each through a validated node handle to the correct stored FEN. Use the same selected treatment for tree and inline references, with a clear back/close action. Plain prose (including unmarked `Nd5`, `e2e4`, etc.) is inert. Page navigation must never activate an off-page target or mutate solving. While analysis is open, use on-screen previous/next page controls; Kobo physical page-turn keys may page analysis (when closed they must continue browsing puzzles). Avoid collisions with square, toolbar, promotion, collection and progress actions.

For duplicate labels, ensure link byte ranges map to IDs, not `str::find` of the label; if Cobalt link geometry is ambiguous, either use explicit safe controls or suppress ambiguous padding. Keep board-preview actions separate from square taps, which remain graded only on the live board when analysis is closed.

## Automated acceptance
- Event tests tap first/last/middle tree move, nested variation, alternative, labeled and unlabeled prose refs, repeated labels and promotion; verify displayed FEN exactly equals stored node FEN.
- Plain move-looking prose and blank areas dispatch no analysis move; invalid/stale/off-page IDs do nothing.
- Previous/next/close, physical page turn, Reset and navigation tests; verify live board, solution cursor, solved IDs, progress persistence/dirty status unchanged throughout browsing.
- Test the same move after Flip; square identities/preview FEN unchanged.

**Done:** Kobo analysis is interactive, not merely a read-only notation dump.
