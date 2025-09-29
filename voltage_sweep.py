#!/usr/bin/env python

OUTDIR = "./ecram_voltage"
TECH = ["ECRAM", "ECRAMopt", "FeFET"]
START = 1
END = 4.01
STEP = 0.5


def writeout(filename, vw: float = 4, tech="ecram"):
    with open(filename, "w") as fout:
        fout.write("n: 128\n")
        fout.write("m: 128\n")

        fout.write(f"bl: 0.8, {round(vw * (1 / 3), 2)}, 0, -1\n")
        fout.write(f"wl: {round(vw, 2)}, {round(vw * (2 / 3), 2)}, 0, 0.8\n")

        fout.write(f"well: 0, {round(vw, 2)}\n")
        fout.write(f"cell: {tech}\n")
        fout.write("enob: 1\n")
        fout.write("fs: 1e9\n")
        fout.write("adcs: 128\n")


i = START

while not i > END:
    for tech in TECH:
        filename = f"{OUTDIR}/{tech}_{i:.2f}.txt"
        writeout(filename, i, tech)

    i = i + STEP
