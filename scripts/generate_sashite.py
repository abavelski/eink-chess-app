#!/usr/bin/env python3
"""Convert the checked-in Sashité Western SVG paths to Cobalt vector commands.

The source set uses only relative m/l/h/v/q/t/z commands and solid fills.
Fail on any other SVG feature so a future upstream change cannot silently
produce different artwork.
"""

from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets" / "sashite-western"
OUTPUT = ROOT / "assets" / "sashite-western" / "generated.rs"
TOKEN = re.compile(r"[a-zA-Z]|[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][-+]?\d+)?")
PIECES = ("king", "queen", "rook", "bishop", "knight", "pawn")
SIDES = (("first", "White"), ("second", "Black"))


def scaled(value):
    return round(value * 1000 / 4096)


def commands(data):
    tokens = TOKEN.findall(data)
    if "".join(tokens) != re.sub(r"[\s,]", "", data):
        raise ValueError("unparsed SVG path data")
    result = []
    x = y = 0.0
    start = (0.0, 0.0)
    control = None
    command = None
    index = 0
    while index < len(tokens):
        if tokens[index].isalpha():
            command = tokens[index]
            index += 1
        if command not in "mlhvqtz":
            raise ValueError(f"unsupported SVG command: {command}")
        if command == "z":
            result.append("Cmd::Close")
            x, y = start
            control = None
            command = None
            continue
        count = {"m": 2, "l": 2, "h": 1, "v": 1, "q": 4, "t": 2}[command]
        values = [float(value) for value in tokens[index : index + count]]
        if len(values) != count or any(token.isalpha() for token in tokens[index : index + count]):
            raise ValueError("incomplete SVG path command")
        index += count
        if command == "m":
            x += values[0]
            y += values[1]
            start = (x, y)
            result.append(f"Cmd::Move({scaled(x)}, {scaled(y)})")
            command = "l"
            control = None
        elif command == "l":
            x += values[0]
            y += values[1]
            result.append(f"Cmd::Line({scaled(x)}, {scaled(y)})")
            control = None
        elif command == "h":
            x += values[0]
            result.append(f"Cmd::Line({scaled(x)}, {scaled(y)})")
            control = None
        elif command == "v":
            y += values[0]
            result.append(f"Cmd::Line({scaled(x)}, {scaled(y)})")
            control = None
        elif command == "q":
            cx, cy = x + values[0], y + values[1]
            x, y = x + values[2], y + values[3]
            result.append(f"Cmd::Quad({scaled(cx)}, {scaled(cy)}, {scaled(x)}, {scaled(y)})")
            control = (cx, cy)
        elif command == "t":
            cx, cy = (2 * x - control[0], 2 * y - control[1]) if control else (x, y)
            x += values[0]
            y += values[1]
            result.append(f"Cmd::Quad({scaled(cx)}, {scaled(cy)}, {scaled(x)}, {scaled(y)})")
            control = (cx, cy)
    return result


def main():
    lines = [
        "//! Generated from the Sashité Western Chess SVG set (CC0 1.0).",
        "//! Source: https://sashite.dev/assets/chess/",
        "//! Run `python3 scripts/generate_sashite.py` in eink-chess-app to regenerate.",
        "use super::Cmd;",
        "use crate::Glyph;",
        "pub(super) struct Layer { pub commands: &'static [Cmd], pub tone: u8 }",
        "pub(super) fn layers(glyph: Glyph) -> Option<&'static [Layer]> {",
        "    match glyph {",
    ]
    for side, color in SIDES:
        for piece in PIECES:
            lines.append(f"        Glyph::Chess{color}{piece.title()} => Some({side.upper()}_{piece.upper()}),")
    lines += ["        _ => None,", "    }", "}"]
    definitions = []
    for side, _ in SIDES:
        for piece in PIECES:
            root = ET.parse(ASSETS / side / f"{piece}.svg").getroot()
            if root.attrib.get("viewBox") != "0 0 4096 4096":
                raise ValueError("unexpected SVG viewBox")
            paths = root.findall("{*}path")
            name = f"{side.upper()}_{piece.upper()}"
            layers = []
            for number, path in enumerate(paths):
                if set(path.attrib) != {"d", "fill"}:
                    raise ValueError(f"unsupported path attributes in {side}/{piece}")
                tone = {"#000": 0, "#333": 51, "#fff": 255}.get(path.attrib["fill"])
                if tone is None:
                    raise ValueError(f"unsupported fill in {side}/{piece}")
                path_name = f"{name}_{number}"
                definitions.append(f"static {path_name}: &[Cmd] = &[")
                definitions += [f"    {cmd}," for cmd in commands(path.attrib["d"])]
                definitions.append("];")
                layers.append(f"    Layer {{ commands: {path_name}, tone: {tone} }},")
            definitions.append(f"static {name}: &[Layer] = &[")
            definitions += layers
            definitions.append("];")
    OUTPUT.write_text("\n".join(lines + definitions) + "\n")
    subprocess.run(["rustup", "run", "1.85.1", "rustfmt", "--edition", "2021", str(OUTPUT)], check=True)


if __name__ == "__main__":
    main()
