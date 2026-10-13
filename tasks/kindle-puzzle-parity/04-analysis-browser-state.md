# 04 — Non-mutating analysis and board-preview state machine

**Dependencies:** 02. **Owner surface:** new pure `src/analysis_browser.rs` plus narrow `ChessBoardApp` integration. Can run in parallel with 03 and 05.

## Deliverable
Keep **live solve board**, **open analysis state** (selected node, page), and **displayed preview position** separate, as on Kindle. Opening analysis does not reset solving. Selecting any valid tree node displays that node's exact stored post-move FEN, respecting the user's current Flip/orientation; it never replays rules. Selection/page changes never alter `solution_ply`, grading, progress, pending promotion or solved IDs and never trigger a progress write. Closing restores the exact live solve board, including in-progress attempt. Root selection may preview the starting position. Tree roles are browsing metadata only: alternative/sideline moves are **never** accepted as graded solutions.

Specify modal priorities and transitions: promotion/picker suppress analysis taps; Reset, collection/puzzle navigation and Free Board transitions close/clear any stale selection before changing the live position; orientation Flip/lock affect only the board view; sleeping/resume retains safe state or closes deterministically. Existing page-turn puzzle navigation must stay unchanged when analysis is closed.

Legacy puzzles may open a description-only view if they contain a description (see 06); they must never create invented post-move FENs.

## Automated acceptance
- Pure state transition table: open/close/select/root/invalid-node/page boundaries/navigation/reset/flip/free board/promotion/picker/sleep.
- Snapshot the live board, solution cursor, feedback, `progress.v1` bytes and dirty-generation before and after browse/preview/close; require identity. Confirm continuing the original solution still works afterwards.
- Test alternative promotion FEN selected vs. wrong promotion in graded mode; tests fail if selecting a sideline changes solved state.

**Done:** Browsing can be exercised entirely on host without rendering/input.
