#!/usr/bin/env python

import math
import os

N = [32, 64, 128, 256]
M = [32, 64, 128, 256]

CIM = True
ADC_COLUMN_MUX = 1

FS = 100e6

OUTDIR = "SUPREME_FOM_CIM"

TECH = {
    "16-level ECRAM": {"start": 1, "end": 4, "step": 0.1, "bits": 4, "cell": "ECRAM"},
    "8-level ECRAM": {"start": 1, "end": 4, "step": 0.1, "bits": 3, "cell": "ECRAM"},
    "8-level FeFET": {"start": 1, "end": 4, "step": 0.1, "bits": 3, "cell": "FeFET"},
    # "MOLECULAR": {"start": 0.41, "end": 0.41, "step": 0, "bits": 1, "cell": "MOLECULAR"},
}


# def writeout(filename, vw: float = 4, tech="ecram"):
def writeout(name, n, m, bl, wl, well, cell, enob):
    filename = os.path.join(OUTDIR, f"{name}_{n}-{m}_{wl[0]}.yaml")

    with open(filename, "w") as fout:
        enob = enob if not CIM else math.ceil(enob + math.log2(n))
        if m % ADC_COLUMN_MUX != 0:
            raise ValueError(f"Cannot assign {m} columns to ADCs with {ADC_COLUMN_MUX} column muxing.")
        adcs = m // ADC_COLUMN_MUX

        name = f"{n} x {m} {name} (Vw = {wl[0]})"

        fout.write(f"name: {name}\n\n")

        fout.write(f"n: {n}\n")
        fout.write(f"m: {m}\n\n")

        fout.write(f"bl: {bl}\n")
        fout.write(f"wl: {wl}\n")
        fout.write(f"well: {well}\n\n")

        fout.write(f"cell: {cell}\n\n")

        fout.write(f"fs: {FS}\n")
        fout.write(f"bits: {enob}\n")
        fout.write(f"adcs: {adcs}\n")


def inhibit_voltages(vw, vr=0.8):
    wl = [round(vw, 2), round(vw * (2 / 3), 2), 0, vr]
    bl = [vr, round(vw * (1 / 3), 2), 0]
    return (wl, bl)


def process_tech(desig, tech):
    for n in N:
        for m in M:
            i = tech["start"]

            while not i > tech["end"]:
                wl, bl = inhibit_voltages(i)
                writeout(desig, n, m, bl, wl, [0, i], tech["cell"], tech["bits"])
                i += tech["step"]


for k, v in TECH.items():
    process_tech(k, v)
