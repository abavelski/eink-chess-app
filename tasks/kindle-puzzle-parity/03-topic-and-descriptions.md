# 03 — Present topic and puzzle descriptions without NOTE settings

**Dependencies:** 02. **Owner surface:** small pure formatting helpers + screen presentation tests; coordinate UI insertions with 06.

## Deliverable
Follow Kindle's current meaning of fields: `topic` is optional display text (preserve Unicode, Cyrillic, and line breaks), `description` is plain compatibility prose, `description_content` is the authoritative rich analysis preface when present, and node/root `content` takes precedence over `comment`. Use the same fallback rules as 02. The normal Solution status area shows `topic` when available, not the full analysis or a duplicated side-to-move label; Free Board keeps its own status. Preserve existing auto-reveal of description on solving and clear it on Reset/new puzzle/Free Board exactly as before. A hidden NOTE control does **not** mean deleting description data.

Define a pure view model for: no topic/no description; topic-only; description-only; rich spans with move refs; blank/null description; solved/revealed; long multilingual text. Description preface in an open analysis panel is available before solving without marking solved. UI/sizing is implemented in 06; do not create a settings surface.

## Automated acceptance
- Exact view-model tests for fallback/explicit empty spans, Unicode and multiline topics, blank/null/non-string topic, description-only legacy, no duplicate preface, and unchanged progress on any reveal.
- Simulator/screen tests later in 06 verify long topic and description fit or paginate rather than clip.
- Old description-on-solve behavior remains test-covered after NOTE is removed.

**Done:** Text presentation contract is testable independently of Cobalt geometry.
