# 01 — Load the same 8 MiB Kindle collection file on Kobo

**Dependencies:** None (parallel to parser and UI). **Owner surface:** Cobalt data access/transport integration, Kobo collection loading adapter, isolated tests; minimize edits to `src/main.rs`.

## Why this is a separate blocking task
Kobo `src/puzzle.rs` caps files at 256 KiB, and the pinned Cobalt `kobo-protocol::MAX_STORE_VALUE` is 256 KiB. Cobalt `MAX_FRAME_LEN` is 8 MiB, so a naive raise to an 8 MiB single-value store response would still exceed a complete transport frame. Current `context.store().list/load` discovery and metadata reads use that restricted path.

## Deliverable
Implement a bounded *read-only* large-collection adapter allowing unmodified files named `puzzles.json` and `puzzles-<name>.json` in the current Kobo user collection directory. Prefer a narrowly-scoped chunked/ranged read capability in the pinned Cobalt integration (or prove and document an equally safe supported file-read API). Do not silently split/repackage files or require a Kindle-specific conversion step. Keep small-file store behavior and `progress.v1` untouched. The final loader must accept a complete raw UTF-8 collection up to and including 8,388,608 bytes, reject anything bigger, cap total accumulation and per-request allocations, detect short/duplicate/out-of-order chunks and modified files, and never partially activate a failed collection. Use an explicit error for Cobalt runtime versions lacking the large-read capability. Treat metadata discovery as bounded (do not cache all books fully just to populate the picker).

**Integration caution:** Don't patch only `MAX_STORE_VALUE`; account for frame envelopes and tests across the real Cobalt protocol/runtime boundary. If the chosen API needs a Cobalt fork/patch, document the exact pin, packaging/update procedure, and failure fallback under `scripts/prepare_cobalt.py`. Don't introduce arbitrary filesystem traversal, unbounded paths, or an app-wide store limit change.

## Automated acceptance
- Integration tests with the actual or faithfully simulated Cobalt store/protocol: valid 256 KiB, 256 KiB+1, 1 MiB, representative 3–5 MiB, exactly 8 MiB; reject 8 MiB+1.
- Large files copied as-is: byte-for-byte equality (SHA-256) between source and reconstructed buffer. Verify filenames remain unchanged and the collector uses bounded memory.
- Simulate denied read, missing mid-stream chunk, file changing mid-read, malformed UTF-8/JSON, over-cap file, missing/invalid selection. Active board/file/progress remain intact; named error shown, valid neighbors still load.
- Run `cargo test` and the Kobo simulator without device access; smoke test direct on-device file naming during Task 09.

**Done:** Size limit no longer blocks genuine Kindle books, *through the complete Kobo loading path*, not merely parser unit tests.
