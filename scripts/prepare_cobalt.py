#!/usr/bin/env python3
"""Add this app to the pinned Cobalt checkout's USB device package."""

from pathlib import Path
import shutil
import subprocess
import sys


PINNED_COBALT = "026ac5561add0157109dd98592272ce9c6eb9343"
APP = Path(__file__).resolve().parents[1]
COBALT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else APP.parent / "Cobalt"


def add_after(path: Path, anchor: str, addition: str) -> None:
    text = path.read_text()
    if addition in text:
        return
    if text.count(anchor) != 1:
        raise SystemExit(f"Cannot find one insertion point in {path}")
    path.write_text(text.replace(anchor, anchor + addition, 1))


if not (COBALT / "Cargo.toml").is_file():
    raise SystemExit(f"Cobalt checkout not found at {COBALT}")
revision = subprocess.check_output(
    ["git", "-C", str(COBALT), "rev-parse", "HEAD"], text=True
).strip()
if revision != PINNED_COBALT:
    raise SystemExit(f"Cobalt is at {revision}; this app requires {PINNED_COBALT}")

destination = COBALT / "examples" / "eink-chess"
(destination / "src").mkdir(parents=True, exist_ok=True)
for name in ("main.rs", "board.rs"):
    shutil.copy2(APP / "src" / name, destination / "src" / name)
(destination / "Cargo.toml").write_text(
    '[package]\n'
    'name = "kobo-eink-chess"\n'
    'version = "0.1.0"\n'
    'edition.workspace = true\n'
    'rust-version.workspace = true\n'
    'publish = false\n\n'
    '[dependencies]\n'
    'kobo-sdk = { path = "../../crates/kobo-sdk" }\n\n'
    '[lints]\n'
    'workspace = true\n'
)

add_after(
    COBALT / "Cargo.toml",
    '    "examples/tictactoe",\n',
    '    "examples/eink-chess",\n',
)
add_after(
    COBALT / "crates" / "kobo-cli" / "src" / "main.rs",
    '    ("kobo-tictactoe", None),\n',
    '    ("kobo-eink-chess", None),\n',
)
add_after(
    COBALT / "crates" / "kobod" / "src" / "app_store.rs",
    'const MANAGED_BUILTINS: &[BuiltinApp] = &[\n',
    '    BuiltinApp {\n'
    '        id: "eink-chess",\n'
    '        title: "E-Ink Chess",\n'
    '        label: "Chess",\n'
    '        summary: "Move pieces freely on a touch chessboard.",\n'
    '        version: "0.1.0",\n'
    '        glyph: Glyph::Grid,\n'
    '        capabilities: &[],\n'
    '    },\n',
)
print(f"Prepared {destination} for the Cobalt USB package")
