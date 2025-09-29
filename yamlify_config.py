#!/usr/bin/env python3
"""
convert_to_yaml.py

Usage:
    ./convert_to_yaml.py <input_file>

The script reads a plain‑text file that contains key/value pairs in one of
two forms:

1. All pairs on a single line, space‑separated:
       n: 128 m: 128 bl: 0.8, 0.33, 0 wl: 1, 0.67, 0, 0.8 …

2. One pair per line (the format you just posted):
       n: 128
       m: 128
       bl: 0.8, 0.33, 0
       wl: 1, 0.67, 0, 0.8
       …

Both styles can be mixed – the parser simply scans the whole file for
“key:” markers and extracts the associated value string.

Values that contain commas are turned into proper YAML arrays; all other
values are kept as scalars (ints/floats when possible).

The output file has the same base name as the input but a “.yaml”
extension.
"""

import sys
import pathlib
import re
import yaml  # pip install pyyaml


# ----------------------------------------------------------------------
# Helper utilities
# ----------------------------------------------------------------------
def _coerce_scalar(txt: str):
    """Convert a plain string to int/float when possible, otherwise keep it."""
    txt = txt.strip()
    if re.fullmatch(r"-?\d+", txt):
        return int(txt)
    if re.fullmatch(r"-?\d*\.\d+(?:[eE][+-]?\d+)?", txt):
        return float(txt)
    return txt


def _parse_key_values(text: str) -> dict:
    """
    Scan *text* for occurrences of ``key: value`` where *value* may contain
    spaces and commas.  Returns a dict mapping keys → processed values.
    """
    # Regex explanation:
    #   (\w+):          – capture the key (letters, digits, underscore) followed by a colon
    #   \s*             – optional whitespace after the colon
    #   (               – start capture of the value
    #       (?!\w+:)    – negative look‑ahead: do NOT start a new key here
    #       .+?         – lazily consume characters (including commas/spaces)
    #   )               – end value capture
    #   (?=\s+\w+:|$)   – stop when we see whitespace + another key, or end‑of‑string
    pattern = re.compile(r"(\w+):\s*((?:(?!\w+:).)+?)(?=\s+\w+:|$)", re.DOTALL)

    result = {}
    for match in pattern.finditer(text):
        key = match.group(1)
        raw_val = match.group(2).strip()

        # If the value contains a comma → treat it as a list
        if "," in raw_val:
            parts = [p.strip() for p in raw_val.split(",")]
            result[key] = [_coerce_scalar(p) for p in parts]
        else:
            result[key] = _coerce_scalar(raw_val)

    return result


# ----------------------------------------------------------------------
# YAML flow‑style list helper (so lists appear as [a, b, …])
# ----------------------------------------------------------------------
class FlowList(list):
    """Subclass that forces PyYAML to emit flow‑style lists."""

    pass


def _represent_flow_list(dumper, data):
    return dumper.represent_sequence("tag:yaml.org,2002:seq", data, flow_style=True)


yaml.add_representer(FlowList, _represent_flow_list)


# ----------------------------------------------------------------------
# Main driver
# ----------------------------------------------------------------------
def main() -> None:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        sys.exit(1)

    in_path = pathlib.Path(sys.argv[1])
    if not in_path.is_file():
        print(f"❌  File not found: {in_path}", file=sys.stderr)
        sys.exit(1)

    # Read the whole file as a single string – this works for both layouts
    raw_text = in_path.read_text(encoding="utf-8")

    # Parse all key/value pairs
    try:
        data = _parse_key_values(raw_text)
    except Exception as exc:  # defensive – should never happen
        print(f"❌  Parsing failed: {exc}", file=sys.stderr)
        sys.exit(1)

    # Convert any list values to FlowList so they render as [a, b, …]
    for k, v in data.items():
        if isinstance(v, list):
            data[k] = FlowList(v)

    # Write the YAML file (same stem, .yaml suffix)
    out_path = in_path.with_suffix(".yaml")
    out_path.write_text(
        yaml.dump(data, sort_keys=False, default_flow_style=False), encoding="utf-8"
    )

    print(f"✅  Converted {in_path.name} → {out_path.name}")


if __name__ == "__main__":
    main()
