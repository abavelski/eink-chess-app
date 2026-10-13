//! Bounded, read-only transfer of unchanged user puzzle collection files.
//!
//! Uses the existing 256-KiB Cobalt shelf *wire* message, with a namespaced
//! read-only provider installed by scripts/prepare_cobalt.py. No store limit,
//! progress record, or serialized user file is changed.

use kobo_sdk::{Context, StoreError, StoreResult};

pub const MAX_COLLECTION_BYTES: usize = 8 * 1024 * 1024;
pub const READ_CHUNK: usize = 64 * 1024;
pub const READ_PREFIX: &str = "eink-chess-collection-read-v1-";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadMode {
    /// Only determine whether a potentially large book can be selected.
    /// An entire shelf of books must never be buffered for picker metadata.
    Metadata,
    /// Read the whole file twice and compare every byte to detect modifications
    /// even when a replacement file has the same length.
    Complete,
}

impl Default for ReadMode {
    fn default() -> Self { Self::Complete }
}

#[derive(Debug)]
pub enum ReadProgress {
    Pending,
    Metadata,
    Complete(Vec<u8>),
    Failed(String),
}

#[derive(Debug)]
pub struct CollectionReader {
    key: String,
    wire_name: String,
    mode: ReadMode,
    size: Option<usize>,
    first_pass: Vec<u8>,
    verify_offset: usize,
    verifying: bool,
}

pub fn collection_key(key: &str) -> bool {
    key == "puzzles.json"
        || key
            .strip_prefix("puzzles-")
            .and_then(|name| name.strip_suffix(".json"))
            .is_some_and(|name| {
                !name.is_empty()
                    && name.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_' || b == b'.'
                    })
            })
}

impl CollectionReader {
    pub fn begin(context: &mut Context, key: &str, mode: ReadMode) -> Result<Self, String> {
        if !collection_key(key) {
            return Err(format!("{key}: invalid collection filename."));
        }
        let reader = Self {
            key: key.to_owned(),
            wire_name: format!("{READ_PREFIX}{key}"),
            mode,
            size: None,
            first_pass: Vec::new(),
            verify_offset: 0,
            verifying: false,
        };
        context.shelf().read(reader.wire_name.clone(), 0, READ_CHUNK as u32);
        Ok(reader)
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn wire_name(&self) -> &str {
        &self.wire_name
    }

    pub fn advance(&mut self, context: &mut Context, result: StoreResult) -> ReadProgress {
        let StoreResult::ShelfRead {
            name,
            offset,
            bytes,
            size,
        } = result
        else {
            return match result {
                StoreResult::Denied(StoreError::Missing) => ReadProgress::Failed(format!(
                    "{}: installed Cobalt runtime lacks the required read-only collection range capability, or access to the collection was denied. Reinstall using scripts/prepare_cobalt.py.",
                    self.key
                )),
                StoreResult::Denied(StoreError::TooFull) => ReadProgress::Failed(format!(
                    "{}: puzzle file must be at most 8 MiB (8,388,608 bytes).", self.key
                )),
                StoreResult::Denied(error) => {
                    ReadProgress::Failed(format!("{}: collection read denied ({error}).", self.key))
                }
                _ => ReadProgress::Failed(format!("{}: unexpected range response.", self.key)),
            };
        };
        if name != self.wire_name {
            return ReadProgress::Failed(format!("{}: wrong collection range response.", self.key));
        }
        let size = size as usize;
        if size > MAX_COLLECTION_BYTES {
            return ReadProgress::Failed(format!(
                "{}: puzzle file must be at most 8 MiB (8,388,608 bytes).",
                self.key
            ));
        }
        if size == 0 || self.size.is_some_and(|original| original != size) {
            return ReadProgress::Failed(format!("{}: collection changed during read.", self.key));
        }
        self.size = Some(size);
        let expected_offset = if self.verifying {
            self.verify_offset
        } else {
            self.first_pass.len()
        };
        let wanted = READ_CHUNK.min(size.saturating_sub(expected_offset));
        if offset as usize != expected_offset || bytes.len() != wanted {
            return ReadProgress::Failed(format!(
                "{}: missing, duplicated, short, or out-of-order collection chunk.",
                self.key
            ));
        }
        if self.mode == ReadMode::Metadata {
            // The first bounded slice establishes the advertised size/readability.
            // Full JSON validation happens when a collection is actually selected.
            return ReadProgress::Metadata;
        }
        if self.verifying {
            if self.first_pass[expected_offset..expected_offset + wanted] != *bytes {
                return ReadProgress::Failed(format!(
                    "{}: collection changed while loading; try reopening it.",
                    self.key
                ));
            }
            self.verify_offset += wanted;
            if self.verify_offset == size {
                return ReadProgress::Complete(std::mem::take(&mut self.first_pass));
            }
        } else {
            if self.first_pass.capacity() == 0 {
                self.first_pass.reserve_exact(size);
            }
            self.first_pass.extend_from_slice(&bytes);
            if self.first_pass.len() == size {
                self.verifying = true;
            }
        }
        let next_offset = if self.verifying {
            self.verify_offset
        } else {
            self.first_pass.len()
        };
        context
            .shelf()
            .read(self.wire_name.clone(), next_offset as u32, READ_CHUNK as u32);
        ReadProgress::Pending
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use kobo_sdk::{AppRunner, Command, DisplayMetrics, KoboApp, StoreRequest};

    #[derive(Default)]
    struct Probe {
        read: Option<CollectionReader>,
        outcome: Option<Result<Vec<u8>, String>>,
        metadata: bool,
        mode: ReadMode,
    }

    impl Probe {
        fn new(mode: ReadMode) -> Self {
            Self { mode, ..Self::default() }
        }
    }

    impl KoboApp for Probe {
        fn on_action(&mut self, _context: &mut Context, _action: kobo_sdk::ActionId) {}

        fn on_start(&mut self, context: &mut Context) {
            self.read = Some(CollectionReader::begin(context, "puzzles.json", self.mode).unwrap());
        }

        fn on_shelf(&mut self, context: &mut Context, _name: &str, result: StoreResult) {
            let mut read = self.read.take().unwrap();
            match read.advance(context, result) {
                ReadProgress::Pending => self.read = Some(read),
                ReadProgress::Metadata => self.metadata = true,
                ReadProgress::Complete(bytes) => self.outcome = Some(Ok(bytes)),
                ReadProgress::Failed(error) => self.outcome = Some(Err(error)),
            }
        }
    }

    fn run(
        bytes: &[u8],
        mode: ReadMode,
        mut answer: impl FnMut(usize, usize, usize, &[u8]) -> StoreResult,
    ) -> (Probe, usize, usize) {
        let mut runner = AppRunner::with_metrics(Probe::new(mode), DisplayMetrics::default());
        let mut pending = runner.start();
        let mut requests = 0;
        let mut max_request = 0;
        while !pending.is_empty() {
            assert!(requests <= 300, "bounded request count");
            let mut next = Vec::new();
            for command in pending {
                let Command::Store(StoreRequest::ShelfRead { name, offset, length }) = command else {
                    panic!("unexpected Cobalt command");
                };
                assert!(name.starts_with(READ_PREFIX));
                assert!(length as usize <= READ_CHUNK);
                requests += 1;
                max_request = max_request.max(length as usize);
                next.extend(runner.store_result(answer(
                    requests,
                    offset as usize,
                    length as usize,
                    bytes,
                )));
            }
            pending = next;
        }
        let probe = std::mem::take(runner.app_mut());
        (probe, requests, max_request)
    }

    fn normal(_count: usize, offset: usize, length: usize, bytes: &[u8]) -> StoreResult {
        StoreResult::ShelfRead {
            name: format!("{READ_PREFIX}puzzles.json"),
            offset: offset as u32,
            bytes: bytes[offset..(offset + length).min(bytes.len())].to_vec(),
            size: bytes.len() as u32,
        }
    }

    #[test]
    fn all_boundaries_are_byte_exact_with_fixed_size_requests() {
        for size in [256_usize * 1024, 256 * 1024 + 1, 1024 * 1024,
            3 * 1024 * 1024, 5 * 1024 * 1024, 8 * 1024 * 1024]
        {
            let original = (0..size).map(|i| (i.wrapping_mul(37) % 251) as u8).collect::<Vec<_>>();
            let (done, requests, max_request) = run(&original, ReadMode::Complete, normal);
            assert_eq!(done.outcome.unwrap().unwrap(), original, "size {size}");
            assert_eq!(requests, (size + READ_CHUNK - 1) / READ_CHUNK * 2);
            assert_eq!(max_request, READ_CHUNK);
        }
    }

    #[test]
    fn metadata_only_asks_for_one_bounded_piece() {
        let bytes = vec![b'x'; 8 * 1024 * 1024];
        let (state, requests, _) = run(&bytes, ReadMode::Metadata, normal);
        assert!(state.metadata);
        assert!(state.outcome.is_none());
        assert_eq!(requests, 1);
    }

    #[test]
    fn refuses_oversize_before_allocating_a_collection() {
        let bytes = vec![b'x'; MAX_COLLECTION_BYTES + 1];
        let (state, requests, _) = run(&bytes, ReadMode::Complete, normal);
        assert!(state.outcome.unwrap().unwrap_err().contains("8 MiB"));
        assert_eq!(requests, 1);
    }

    #[test]
    fn detects_short_misordered_duplicate_missing_and_changed_chunks() {
        let data = vec![0x63; READ_CHUNK * 3];
        let short = |n, off, len, data: &[u8]| {
            let mut reply = normal(n, off, len, data);
            if n == 2 {
                if let StoreResult::ShelfRead { bytes, .. } = &mut reply { bytes.pop(); }
            }
            reply
        };
        let (state, _, _) = run(&data, ReadMode::Complete, short);
        assert!(state.outcome.unwrap().is_err());
        let order = |n, off, len, data: &[u8]| {
            let mut reply = normal(n, off, len, data);
            if n == 2 {
                if let StoreResult::ShelfRead { offset, .. } = &mut reply { *offset = 0; }
            }
            reply
        };
        assert!(run(&data, ReadMode::Complete, order).0.outcome.unwrap().is_err());
        let changed = |n, off, len, data: &[u8]| {
            let mut reply = normal(n, off, len, data);
            if n == 5 {
                if let StoreResult::ShelfRead { bytes, .. } = &mut reply { bytes[0] ^= 1; }
            }
            reply
        };
        assert!(run(&data, ReadMode::Complete, changed).0.outcome.unwrap().is_err());
        let size_change = |n, off, len, data: &[u8]| {
            let mut reply = normal(n, off, len, data);
            if n == 2 {
                if let StoreResult::ShelfRead { size, .. } = &mut reply { *size += 1; }
            }
            reply
        };
        assert!(run(&data, ReadMode::Complete, size_change).0.outcome.unwrap().is_err());
        let missing = |n, off, len, data: &[u8]| {
            if n == 2 { StoreResult::Denied(StoreError::Missing) }
            else { normal(n, off, len, data) }
        };
        assert!(run(&data, ReadMode::Complete, missing).0.outcome.unwrap().is_err());
    }

    #[test]
    fn denied_and_unpatched_runtime_report_named_errors() {
        for error in [StoreError::Unwritable, StoreError::Missing, StoreError::TooFull] {
            let (state, requests, _) = run(b"hello", ReadMode::Complete,
                |_n, _o, _l, _| StoreResult::Denied(error));
            let reason = state.outcome.unwrap().unwrap_err();
            assert!(reason.contains("puzzles.json"));
            if error == StoreError::Missing {
                assert!(reason.contains("runtime lacks"));
            }
            assert_eq!(requests, 1);
        }
    }

    #[test]
    fn rejects_non_collection_keys_without_sending_any_read() {
        assert!(!collection_key("../secret"));
        assert!(!collection_key("progress.v1"));
        assert!(!collection_key("puzzles-.json"));
        assert!(!collection_key("puzzles-AAA.json"));
        assert!(collection_key("puzzles-study-book.json"));
    }
}
