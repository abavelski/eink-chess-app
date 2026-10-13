# 06 — Kobo Analysis toolbar, text layout and pagination

**Dependencies:** 03, 04, 05. **Owner surface:** `src/main.rs` Cobalt screen builder and UI adapter/tests; do not add a settings page.

## Deliverable
On the normal full-size Kobo chessboard toolbar, **replace the currently visible NOTE icon/control with an always-visible Analysis control** (button/icon/label with clear accessible action); keep Free Board, orientation lock, Reset, Flip and the same large full-sized board. No small board, review tab, or settings toggle. Analysis's selected state is visually unambiguous in monochrome.

For rich puzzles, Analysis opens a paginated pane in the available lower reading area while the board remains on screen. For a legacy puzzle with no `analysis` but a description, opening Analysis shows a **description-only** pane; for a puzzle with neither analysis nor description, keep Analysis visible but inert/visibly disabled. The preface is shown on opening rich analysis even before solving, with full main/variation prose accessible through pages. Closing it restores ordinary topic/description status. Preserve the existing automatic on-solve description reveal independently of the hidden NOTE control.

Use the pinned Cobalt SDK's `ScreenBuilder::rich_text_linking` / `RichTextSpan` / `ParagraphPresentation` when sufficient; do not add a new renderer until a failing simulator test proves it necessary. Note Cobalt limits a rich paragraph to **16 text links** and **256 style spans**, and a screen to **512 nodes**. Split document across deterministic pages and/or paragraphs **before** limits; never silently truncate targets/text. Preserve byte offsets at UTF-8 character boundaries, typography (distinguishable interactive moves), clear selected state, page number and previous/next/close actions. For undersized targets/overlapping link padding prefer unambiguous hits over activating the wrong node. Do not shrink the original chessboard to fit text.

## Automated acceptance
- AppRunner/screen-node tests for visible Analysis, absent NOTE, no settings, active/inactive states, preserved other controls and board 8×8.
- Cobalt `build_checked`/simulator diagnostics at Libra H2O metrics for no clipping/overflow/tiny targets across first/middle/last pages, rich/legacy/blank, short/long Cyrillic, black start, dense links and long words.
- Page coverage assertion: every document token/ref appears exactly once across generated pages; test >16 refs per paragraph and >256 spans, UTF-8 boundaries and no missing last page.
- Snapshot or stable node-tree tests for selected vs unselected moves, old board geometry, navigation affordances and description-on-solve.

**Done:** Usable, deterministic analysis pagination with all required controls, no NOTE or new settings.
