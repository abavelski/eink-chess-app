# 09 — Physical Kobo Libra H2O acceptance (HUMAN CHECKPOINT)

**Dependencies:** 08 plus all host gates passing. **Owner surface:** checklist/results document only until an on-device defect is reproduced. Do **not** label a simulated screenshot as physical acceptance.

## Setup
Use existing `docs/DEVICE_UPDATE_DIRECT_LAUNCH.md` procedure, record exact Kobo build commit, pinned Cobalt/runtime revision, installed binary hash, test-file SHA-256 checksums and pre/post progress checksum. Back up the existing `progress.v1` and puzzle files; close app before copying. Install one legacy collection, one small rich fixture and a genuine >256 KiB Kindle-generated book copied *unchanged* (ideally 3–5 MiB). Confirm file hash identity.

## Owner-checkable walkthrough
1. Launch via NickelMenu. Confirm the original full 8×8 board, Free Board/lock/Reset/Flip, visible Analysis and **no NOTE** or settings page; taps/refreshes responsive in monochrome.
2. Open legacy collection; grade a correct line, incorrect move rollback, automatic reply, promotion dialog and solved marker. Confirm description is revealed on completion; if legacy description exists, Analysis opens text-only view. Exit and restart; solved/current IDs intact.
3. Open rich black-to-move promotion fixture; see topic where provided and description preface. Browse SAN, NAG punctuation, inline book-style comments, main path, `(`variation`)`, alternative and sideline; check original order and paging, no clipping/ghosting.
4. Tap a main move, a variation/underpromotion and both labeled and unlabeled description/comment refs. Each previews the **exact stored** post-move FEN on the existing board. Tap unmarked move-looking prose: no navigation. Flip while previewing, then close; live graded position restored exactly.
5. Begin a partially solved rich puzzle; open Analysis, change pages and preview a sideline, close and complete the **original** graded main path. Sideline is not graded as an alternative solution; browsing alone does not set Solved or save progress.
6. Test lengthy Cyrillic topic/comment/description, a dense-link page and first/middle/last pagination; check tap targets at reading distance, board size unchanged, full redraw/sleep-wake if supported.
7. Select >256 KiB book, navigate its pages, change collections, then intentionally choose a malformed file. Verify clear error and unchanged active board; uploaded file hashes intact.
8. Exit through normal control back to Nickel and relaunch. Check active collection, current puzzle, solved IDs, orientation behavior and byte-for-byte unchanged puzzle files. Confirm progress changed **only** for actual solves/navigation as expected.

## Evidence and signoff
Record device model/firmware, date, tester, build SHA, screenshots/photos or concise tap results, paging responsiveness/ghosting observations, pre/post puzzle hashes and progress diff. Label each case PASS/FAIL/NOT TESTED; don't infer a PASS from automated tests. File defects as new tasks; mark this checkpoint complete only after explicit human acceptance.
