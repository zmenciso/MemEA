#!/usr/bin/env python

import sys


def process_file(filename):
    with open(filename, "r") as f:
        lines = f.readlines()

    width = None
    height = None
    output_lines = []

    for line in lines:
        stripped = line.strip()

        # Copy comments directly
        if stripped.startswith("#"):
            output_lines.append(line)
            continue

        if stripped.startswith("width"):
            width = stripped.split()[1]
            continue
        elif stripped.startswith("height"):
            height = stripped.split()[1]
            continue
        elif stripped.startswith("enc"):
            value = stripped.split()[1]
            # Duplicate enc value
            newline = f"    enc {value}, {value}\n"
            output_lines.append(newline)
        elif (
            stripped.startswith("dx")
            or stripped.startswith("voltage")
            or stripped.startswith("switch:")
        ):
            output_lines.append(line)
        else:
            output_lines.append(line)

        # Insert size line just before writing "enc"
        if stripped.startswith("enc") and width and height:
            size_line = f"    size {width}, {height}\n"
            output_lines.insert(len(output_lines) - 1, size_line)
            width, height = None, None  # reset for next block

    # Print to stdout
    sys.stdout.writelines(output_lines)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} input.txt", file=sys.stderr)
        sys.exit(1)
    process_file(sys.argv[1])
