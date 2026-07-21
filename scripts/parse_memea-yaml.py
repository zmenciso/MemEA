#!/usr/bin/env python

import yaml
import sys
import os
import re

import pandas as pd
import numpy as np

from pprint import pprint

DATABASE = sys.argv[1]
OUTPUT = f"{os.path.splitext(DATABASE)[0]}_parsed.csv"
PATTERN = r'([0-9]+) x ([0-9]+) ([0-9]+)-level ([\w\-\_]+) \(Vw = ([0-9\.]+)\)'

with open(DATABASE, 'r') as f:
    data = yaml.safe_load(f)

df = pd.DataFrame(
    columns=["Rows", "Columns", "Write Voltage [V]", "States", "Device", "Array [μm²]", "ADC [μm²]", "Switch [μm²]", "Logic [μm²]", "Total [μm²]"]
)

regex = re.compile(PATTERN)

for config, cells in data.items():
    matches = regex.search(config)
    n, m, states, device, vw = matches.group(1, 2, 3, 4, 5)

    array = sum([cell['area'] for cell in cells if cell['celltype'] == 'Core'])
    adc = sum([cell['area'] for cell in cells if cell['celltype'] == 'ADC'])
    switch = sum([cell['area'] for cell in cells if cell['celltype'] == 'Switch'])
    logic = sum([cell['area'] for cell in cells if cell['celltype'] == 'Logic'])

    total = array + adc + switch + logic

    df.loc[-1] = [int(n), int(m), float(vw), int(states), device, array, adc, switch, logic, total]
    df.index = df.index + 1
    df = df.sort_index()

df['Capacity [B]'] = df['Rows'] * df['Columns'] * np.log2(df['States']) / 8
df['Capacity [kB]'] = df['Capacity [B]'] / 1000
df['Array Density [B/μm²]'] = df['Capacity [B]'] / df['Array [μm²]']
df['Effective Density [B/μm²]'] = df['Capacity [B]'] / df['Total [μm²]']

df["Array Percentage"] = df["Array [μm²]"] / df["Total [μm²]"] * 100
df["ADC Percentage"] = df["ADC [μm²]"] / df["Total [μm²]"] * 100
df["Switch Percentage"] = df["Switch [μm²]"] / df["Total [μm²]"] * 100
df["Logic Percentage"] = df["Logic [μm²]"] / df["Total [μm²]"] * 100

print(df)

df.to_csv(OUTPUT, index=False)
