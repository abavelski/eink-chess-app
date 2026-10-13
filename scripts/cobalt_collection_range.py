"""Patch pinned Cobalt's store/provider with a bounded read-only collection endpoint."""
from pathlib import Path

METHOD = '''
    // E-Ink Chess: typed range reads of *existing* keyed puzzle files.
    // No changes to normal state/shelf write or read limits.
    pub fn read_puzzle_collection_range(&self, request: &StoreRequest) -> Option<StoreResult> {
        let StoreRequest::ShelfRead { name, offset, length } = request else {
            return None;
        };
        let Some(filename) = name.strip_prefix("eink-chess-collection-read-v1-") else {
            return None;
        };
        let valid = filename == "puzzles.json"
            || filename.strip_prefix("puzzles-")
                .and_then(|n| n.strip_suffix(".json"))
                .is_some_and(|n| !n.is_empty());
        if !valid || !is_valid_key(filename) {
            return Some(StoreResult::Denied(StoreError::BadKey));
        }
        let Some(root) = self.root.as_ref() else {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        };
        let path = root.join(filename);
        // Never follow a symlink outside the application's state directory.
        let Ok(link_info) = fs::symlink_metadata(&path) else {
            return Some(StoreResult::Denied(StoreError::Missing));
        };
        if !link_info.file_type().is_file() {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        }
        let Ok(mut file) = fs::File::open(&path) else {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        };
        let Ok(before) = file.metadata() else {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        };
        if !before.is_file() {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        }
        if before.len() > 8 * 1024 * 1024 {
            return Some(StoreResult::Denied(StoreError::TooFull));
        }
        let size = before.len() as u32;
        if *offset > size {
            return Some(StoreResult::Denied(StoreError::Missing));
        }
        let count = (*length as usize).min(64 * 1024).min((size - *offset) as usize);
        let mut bytes = vec![0; count];
        if file.seek(SeekFrom::Start(u64::from(*offset))).is_err()
            || file.read_exact(&mut bytes).is_err()
        {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        }
        let Ok(after) = file.metadata() else {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        };
        if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
            return Some(StoreResult::Denied(StoreError::Unwritable));
        }
        Some(StoreResult::ShelfRead { name: name.clone(), offset: *offset, bytes, size })
    }
'''

def replace_once(path: Path, old: str, new: str) -> None:
    data = path.read_text()
    if new in data:
        return
    if data.count(old) != 1:
        raise RuntimeError(f"Cobalt changed; cannot safely patch {path}")
    path.write_text(data.replace(old, new, 1))

def install(cobalt: Path) -> None:
    policy = cobalt / "crates/kobo-policy/src/store.rs"
    device = cobalt / "crates/kobod/src/device.rs"
    sim = cobalt / "crates/kobod/src/main.rs"
    replace_once(policy, "use std::io::{Read, Write};",
                 "use std::io::{Read, Seek, SeekFrom, Write};")
    replace_once(policy, "impl Store {\n", "impl Store {\n" + METHOD)
    replace_once(device, """.shelf
                                .handle(&request)
                                .unwrap_or_else(|| apps[index].store.handle(&request))""",
                 """.store
                                .read_puzzle_collection_range(&request)
                                .or_else(|| apps[index].shelf.handle(&request))
                                .unwrap_or_else(|| apps[index].store.handle(&request))""")
    replace_once(sim, """let result = shelf
                    .handle(&request)
                    .unwrap_or_else(|| store.handle(&request));""",
                 """let result = store
                    .read_puzzle_collection_range(&request)
                    .or_else(|| shelf.handle(&request))
                    .unwrap_or_else(|| store.handle(&request));""")
