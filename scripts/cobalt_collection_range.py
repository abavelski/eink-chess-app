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
    data = policy.read_text()
    if "mod eink_chess_collection_range_tests" not in data:
        policy.write_text(data + TESTS)


# Device and host policy are both exercised by cargo test -p kobo-policy after
# preparation. This tests the *real* provider that the app's range client calls.
TESTS = '''
#[cfg(test)]
mod eink_chess_collection_range_tests {
    use super::*;

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "eink-chess-collection-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id(),
        ))
    }

    #[test]
    fn unchanged_bytes_are_read_in_bounded_ranges_at_every_file_size_limit() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let store = Store::new(&root);
        let filename = "puzzles-book.json";
        for size in [
            256 * 1024,
            256 * 1024 + 1,
            1024 * 1024,
            4 * 1024 * 1024,
            8 * 1024 * 1024,
        ] {
            let original = (0..size)
                .map(|i: usize| (i.wrapping_mul(17) % 251) as u8)
                .collect::<Vec<_>>();
            fs::write(root.join(filename), &original).unwrap();
            let name = format!("eink-chess-collection-read-v1-{filename}");
            let mut assembled = Vec::new();
            for offset in (0..size).step_by(64 * 1024) {
                let request = StoreRequest::ShelfRead {
                    name: name.clone(),
                    offset: offset as u32,
                    length: 64 * 1024,
                };
                let Some(StoreResult::ShelfRead {
                    name: answer_name,
                    bytes,
                    size: reported,
                    offset: answer_offset,
                }) = store.read_puzzle_collection_range(&request) else {
                    panic!("expected a bounded range reply");
                };
                assert_eq!(answer_name, name);
                assert_eq!(answer_offset, offset as u32);
                assert_eq!(reported, size as u32);
                assert!(bytes.len() <= 64 * 1024);
                assembled.extend(bytes);
            }
            assert_eq!(assembled, original);
            assert_eq!(fs::read(root.join(filename)).unwrap(), original);
        }
        fs::write(root.join(filename), vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
        assert_eq!(
            store.read_puzzle_collection_range(&StoreRequest::ShelfRead {
                name: format!("eink-chess-collection-read-v1-{filename}"),
                offset: 0,
                length: 64 * 1024,
            }),
            Some(StoreResult::Denied(StoreError::TooFull))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_bad_paths_symlinks_and_non_collection_operations() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let store = Store::new(&root);
        fs::write(root.join("progress.v1"), b"durable").unwrap();
        for name in ["../progress.v1", "progress.v1", "puzzles-.json"] {
            assert_eq!(
                store.read_puzzle_collection_range(&StoreRequest::ShelfRead {
                    name: format!("eink-chess-collection-read-v1-{name}"),
                    offset: 0,
                    length: 42,
                }),
                Some(StoreResult::Denied(StoreError::BadKey))
            );
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("progress.v1"), root.join("puzzles-link.json"))
                .unwrap();
            assert_eq!(
                store.read_puzzle_collection_range(&StoreRequest::ShelfRead {
                    name: "eink-chess-collection-read-v1-puzzles-link.json".into(),
                    offset: 0,
                    length: 42,
                }),
                Some(StoreResult::Denied(StoreError::Unwritable))
            );
        }
        assert!(store
            .read_puzzle_collection_range(&StoreRequest::Load { key: "progress.v1".into() })
            .is_none());
        assert_eq!(fs::read(root.join("progress.v1")).unwrap(), b"durable");
        let _ = fs::remove_dir_all(&root);
    }
}
'''
