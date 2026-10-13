//! Contract-corpus checks only. Semantic rich-tree validation belongs to parity task 02.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const UPSTREAM_REVISION: &str = "80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8";
const UPSTREAM_SPEC: &str = "https://github.com/abavelski/kindle-chess-puzzles/blob/80eaf892fe0c7a96e3ba603e25ee3af57e3d93c8/docs/PUZZLE_FORMAT.md";
const LEGACY_EXAMPLE_SHA256: &str =
    "0521825d87640c69b5989c16748c6d92189f3d3edb34dd2ab612884314599e88";

// Intentionally independent of manifest.json: a missing, relabeled or extra import fails.
const EXPECTED_FIXTURES: &[(&str, &str)] = &[
    ("pgn-converter/valid-book.json", "valid"),
    ("puzzles.json", "valid"),
    ("rich-analysis/invalid-cycle.json", "invalid"),
    ("rich-analysis/invalid-disconnected.json", "invalid"),
    ("rich-analysis/invalid-duplicate-id.json", "invalid"),
    ("rich-analysis/invalid-main-solution-mismatch.json", "invalid"),
    ("rich-analysis/invalid-missing-child.json", "invalid"),
    ("rich-analysis/invalid-move-ref-dangling.json", "invalid"),
    ("rich-analysis/invalid-move-ref-root.json", "invalid"),
    ("rich-analysis/invalid-root-fen-mismatch.json", "invalid"),
    ("rich-analysis/invalid-span-bad-label.json", "invalid"),
    ("rich-analysis/invalid-span-missing-text.json", "invalid"),
    ("rich-analysis/invalid-span-unknown-type.json", "invalid"),
    ("rich-analysis/invalid-uci.json", "invalid"),
    ("rich-analysis/valid-rich.json", "valid"),
];

#[derive(Deserialize)]
struct Manifest {
    upstream_repository: String,
    upstream_commit: String,
    normative_spec: String,
    normative_spec_git_blob_sha1: String,
    raw_file_limit_bytes: usize,
    legacy_control: LegacyControl,
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
struct LegacyControl {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Fixture {
    path: String,
    upstream_path: String,
    sha256: String,
    bytes: usize,
    git_blob_sha1: String,
    expected: String,
    #[serde(default)]
    invariant: Option<String>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kindle-parity")
}

fn collect_files(dir: &Path, base: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read fixture directory") {
        let entry = entry.expect("read fixture entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, base, files);
        } else {
            let relative = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative != "manifest.json" {
                files.push(relative);
            }
        }
    }
}

#[test]
fn corpus_manifest_pins_paths_labels_and_raw_byte_hashes() {
    let root = root();
    let manifest: Manifest = serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap())
        .expect("parse manifest");
    assert_eq!(
        manifest.upstream_repository,
        "https://github.com/abavelski/kindle-chess-puzzles"
    );
    assert_eq!(manifest.upstream_commit, UPSTREAM_REVISION);
    assert_eq!(manifest.normative_spec, UPSTREAM_SPEC);
    assert_eq!(
        manifest.normative_spec_git_blob_sha1,
        "1076d9a09087b7e0b56b1c8d8fd6e54f0f81dcfa"
    );
    assert_eq!(manifest.raw_file_limit_bytes, 8 * 1024 * 1024);

    let mut listed: Vec<(&str, &str)> = manifest
        .fixtures
        .iter()
        .map(|case| (case.path.as_str(), case.expected.as_str()))
        .collect();
    listed.sort_unstable();
    assert_eq!(
        listed.as_slice(),
        EXPECTED_FIXTURES,
        "manifest fixture names/labels drifted"
    );

    let mut disk = Vec::new();
    collect_files(&root, &root, &mut disk);
    disk.sort();
    let expected_paths: Vec<String> = EXPECTED_FIXTURES
        .iter()
        .map(|(path, _)| (*path).to_owned())
        .collect();
    assert_eq!(disk, expected_paths, "fixture files differ from pinned corpus");

    for case in &manifest.fixtures {
        assert_eq!(case.upstream_path, format!("tests/fixtures/{}", case.path));
        assert!(
            case.git_blob_sha1.len() == 40
                && case.git_blob_sha1.bytes().all(|b| b.is_ascii_hexdigit())
        );
        if case.expected == "invalid" {
            assert!(case.path.starts_with("rich-analysis/invalid-"));
            assert!(case.invariant.as_deref().is_some_and(|s| !s.trim().is_empty()));
        } else {
            assert!(case.invariant.is_none());
        }
        let bytes = fs::read(root.join(&case.path)).expect("read fixture");
        assert_eq!(bytes.len(), case.bytes, "fixture size changed: {}", case.path);
        assert_eq!(
            sha256_hex(&bytes),
            case.sha256,
            "fixture bytes changed: {}",
            case.path
        );
        // Invalid means invalid rich-analysis semantics, not malformed JSON.
        let collection: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(collection["version"], 1, "{}", case.path);
        assert!(
            collection["puzzles"]
                .as_array()
                .is_some_and(|p| !p.is_empty()),
            "{}",
            case.path
        );
    }
}

#[test]
fn existing_legacy_example_was_not_replaced_by_the_kindle_control() {
    let manifest: Manifest =
        serde_json::from_str(include_str!("fixtures/kindle-parity/manifest.json")).unwrap();
    assert_eq!(manifest.legacy_control.path, "examples/puzzles.json");
    assert_eq!(manifest.legacy_control.sha256, LEGACY_EXAMPLE_SHA256);
    let original = include_bytes!("../examples/puzzles.json");
    assert_eq!(sha256_hex(original), LEGACY_EXAMPLE_SHA256);
    assert_ne!(
        &original[..],
        &include_bytes!("fixtures/kindle-parity/puzzles.json")[..]
    );
}

#[test]
fn raw_fixture_bytes_are_available_to_host_cargo_tests() {
    const LEGACY: &[u8] = include_bytes!("fixtures/kindle-parity/puzzles.json");
    const RICH: &[u8] = include_bytes!("fixtures/kindle-parity/rich-analysis/valid-rich.json");
    const BOOK: &[u8] = include_bytes!("fixtures/kindle-parity/pgn-converter/valid-book.json");
    let legacy: serde_json::Value = serde_json::from_slice(LEGACY).unwrap();
    let rich: serde_json::Value = serde_json::from_slice(RICH).unwrap();
    let book: serde_json::Value = serde_json::from_slice(BOOK).unwrap();
    assert_eq!(legacy["puzzles"].as_array().unwrap().len(), 2);
    assert_eq!(rich["puzzles"][0]["id"], "rich-black-promotion");
    assert_eq!(rich["puzzles"][0]["analysis"]["root"], "n0");
    assert_eq!(book["puzzles"].as_array().unwrap().len(), 3);
    assert_eq!(
        book["puzzles"][0]["analysis"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
}

// Test-only SHA-256 keeps the corpus integrity gate independent of extra crates/tools.
fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
        0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
        0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f,
        0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (input.len() as u64)
        .checked_mul(8)
        .expect("SHA-256 length fits in u64");
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (i, four) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes(four.try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7)
                ^ w[i - 15].rotate_right(18)
                ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17)
                ^ w[i - 2].rotate_right(19)
                ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (hash, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *hash = hash.wrapping_add(value);
        }
    }
    h.iter().map(|n| format!("{n:08x}")).collect()
}

#[test]
fn test_only_sha256_has_standard_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
