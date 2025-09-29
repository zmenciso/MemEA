#!/usr/bin/env python3
import re
import sys
import yaml


# --- Render certain lists inline like [a, b] ---
class FlowList(list):
    pass


def _represent_flow_seq(dumper, data):
    return dumper.represent_sequence("tag:yaml.org,2002:seq", data, flow_style=True)


# Register representer for SafeDumper (the one used by yaml.safe_dump)
yaml.SafeDumper.add_representer(FlowList, _represent_flow_seq)

# Which keys are lists in dims / top-level
LIST_KEYS_IN_DIMS = {"size", "enc"}
TOPLEVEL_LIST_KEYS = {"voltage"}


def to_number(s: str):
    s = s.strip().rstrip(",")
    # keep scientific notation as string (matches examples like 1e9, 150e6)
    if re.search(r"[eE]", s):
        return s
    # integer
    if re.fullmatch(r"[+-]?\d+", s):
        try:
            return int(s)
        except ValueError:
            return s
    # float
    if re.fullmatch(r"[+-]?(?:\d+\.\d*|\d*\.\d+|\d+)", s):
        try:
            return float(s)
        except ValueError:
            return s
    return s  # leave as string


def parse_value_list(rest: str):
    # split on commas; tolerate extra spaces and trailing commas
    parts = [p for p in re.split(r"\s*,\s*", rest.strip().rstrip(",")) if p != ""]
    return FlowList([to_number(p) for p in parts])


def parse_old_format(path: str):
    out = {}  # top-level: section -> { name -> block }
    section = None
    name = None
    block = None
    dims = None

    with open(path, "r") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue

            # New block header, e.g. "switch: TXGD16"
            m = re.match(r"^(\w+):\s*(\S+)$", line)
            if m:
                # flush previous block
                if section and name and block is not None:
                    if dims:
                        block["dims"] = dims
                    out.setdefault(section, {})[name] = block
                section, name = m.group(1), m.group(2)
                block = {}
                dims = {}
                continue

            # Property line: "key value(s)"
            kv = re.match(r"^([A-Za-z_]\w*)\s+(.*)$", line)
            if not kv or block is None:
                continue
            key, rest = kv.group(1), kv.group(2).strip()

            if key in LIST_KEYS_IN_DIMS:
                dims[key] = parse_value_list(rest)
            elif key in TOPLEVEL_LIST_KEYS:
                block[key] = parse_value_list(rest)
            else:
                # If it looks comma-separated, make it a (flow) list; else scalar
                if "," in rest:
                    block[key] = parse_value_list(rest)
                else:
                    block[key] = to_number(rest)

    # flush last block
    if section and name and block is not None:
        if dims:
            block["dims"] = dims
        out.setdefault(section, {})[name] = block

    return out


def convert_to_yaml(in_path: str, out_path: str):
    data = parse_old_format(in_path)
    with open(out_path, "w") as f:
        yaml.safe_dump(data, f, sort_keys=False)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python convert_to_yaml.py <old.txt> <new.yaml>")
        sys.exit(1)
    convert_to_yaml(sys.argv[1], sys.argv[2])
