# MemEA

MemEA is a simple tool for estimating the area breakdown of memory arrays.
It supports multiple array types (e.g. CAM, SRAM, compute-in-memory) and
emerging devices (e.g. FeFET).

## Installation

Install the latest version of `rustup`, then run:

```bash
cargo build --release
```

The output executable will be `target/release/memea`.

## Usage

MemEA requires two inputs: **1)** a configuration file that describes the memory
array and **2)** a database of cells and peripheral circuits.  Both are written
in simple plain text files as key/value pairs.  Empty lines and lines starting
with `#` are always ignored.

MemEA also accepts multiple configuration files, which will be compared
against each other after running.

Command line options:

- `-e` or `--export` `[FILENAME]`: Output results to file in CSV format
- `-a` or `--area-only`: Only output total area (automatically toggles `-q`)
- `-q` or `--quiet`: Suppress nonessential messages
- `-d` or `--db`: Specify database (default: `./data/db.txt`)
- `--autoscale` `[FROM]` `[TO]`: Use built-in transistor scaling data to
scale area from source technology node (e.g. `65`) to target technology node
(e.g. `22`)
- `--scale` `[VALUE]`: Manually specify a scaling value to scale area (e.g.
`0.124`)

### Memory Configuration

Options are separated from their values by `:`.  The required options are `n`
(the number of rows), `m` (the number of columns), and `cell` (the memory cell
to use).

A full list of options is provided below:

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `n` | `int` | Number of rows | `64` |
| `m` | `int` | Number of columns | `64` |
| `bl` | `array[float]` | Required bitline voltages. | `1, 2, 0, -1` |
| `wl` | `array[float]` | Required wordline voltages. | `4, 2.5, 0` |
| `well` | `array[float]` | Required well voltages (to bias a row-wise, column-wise or full-array deep n-well). | `0, 4` |
| `cell` | `string` | Which in the database to use as the memory cell. | `2FeFET_TCAM_100` |
| `enob` | `int` | Minimum ENOB for downstream ADCs (also supports sense-amplifiers and other single-bit data conversion) | `1` |
| `fs` | `float` | ADC sampling rate | `1e9` |
| `adcs` | `int` | Number of ADCs per array | `64` |

"Bitline" and "wordline" represent abstract vertical and horizontal lines,
respectively. If more lines are needed (e.g. bitline **and** senseline, a cell
representing an entire word with many bitlines), then repeat voltages in the
appropriate line.  For example:

```
bl: 4, 4, 2.5, 0, 0
```

An example configuration is also available: `./config/example.txt`.

### Database

The database has four types of circuits: `core`, `logic`, `switch`, and `adc`.
Describing a circuit begins with the type and name of the circuit, separated by
`:` or `=`. Subsequent lines describe the properties of the circuit, separated
by whitespace, `:`, or `=`.  For example:

```
switch: TXGD16
    voltage 1.3
    dx 16
    spc_x 1.156
    spc_y 0.995
    enc 0.490
```

An example database is also available: `./data/example.txt`.

All types of circuits require the following geometric properties:

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `spc_x` | `float` | Minimum horizontal space between instances in an array (pitch), in μm | `0.432` |
| `spc_y` | `float` | Minimum vertical space between instances in an array (pitch), in μm | `0.432` |
| `enc` | `float` | Spacing required between this circuit and any other circuit (e.g. considering well-to-well spacing), in μm | `1.48` |

> Check back for a diagram explaining these properties

> **note**: `memea` assumes that cells cannot be rotated, as this is the case in
> most advanced manufacturing nodes.

Then, each of the four types has additional properties:

#### `core`

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `dx_bl` | `float` | Relative bitline drive strength required per-cell | `0.25` |
| `dx_wl` | `float` | Relative wordline drive strength required per-cell | `0.25` |

#### `logic`

For logic, include any required signal buffers and inverters.  For example, a
decoder driving transmission gates will require complimentary outputs (extra
inverters).

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `fs` | `float` | Maximum operating speed of the logic | `1e9` |
| `dx` | `float` | Relative drive strength output of the logic | `6` |
| `bits` | `int` | Number of control bits (i.e. a 2-bit logic circuit can drive up to 4 switches) | `2` |

#### `switch`

For all switches, include any required well biasing contacts.  For high voltage
switches, include the level shifters required to drive them from core voltage
logic.

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `voltage` | `floatTuple`[^1] | Maximum voltage the switch can drive before oxide breakdown | `(0.8, 1.3)` |
| `dx` | `float` | Relative drive strength of the switch | `16` |

#### `adc`

For ADCs, include any switches that might be needed to isolate the circuit from
high voltage.

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `bits` | `float` | ENOB of the ADC | `6.2` |
| `fs` | `float` | Maximum sampling rate of the ADC | `2e9` |

## Helper Scripts

### `size_sweep.fish`

This script provides a convenient way to generate all the size configurations of
`N` and `M` for a given array configuration.  The user must set the following
parameters:

```fish
set OUTPUT_DIR cim_sweep

set N 16 32 64 128 256
set M 16 32 64 128 256

set BL '1, 2, 0'
set WL '4, 2.5, 0, 1'
set WELL '0, 4'
set CELL 1FeFET_100

# Optional parameters - leave empty to exclude from output configurations
set ENOB 6
set FS 100e6

# ADCs (optional): Set to 'all' to match BLs, otherwise specify number
set ADCS all
```

Then, run the script:

```bash
fish size_sweep.fish
```

## Planned Updates

### New Features

- Interactive/automatic database builder from GDS
- Combine like elements in area breakdown (i.e. only report unique
    occurrences)
- Produce example/estimated floorplan for each configuration
- Support for shared peripherals across multiple memory arrays (e.g. BL
    drivers alternate between driving two different arrays)
- Stacking multiple switches per row/column to achieve the required drive
    strength
- Interactive mode
- Better configuration system (i.e. specify variable sweeps within the
    configuration file)
- Automatic enob calculation option based on the number of rows (assuming
    1-bit per cell): `enob auto`

### Performance

- Reduce the number of times file descriptors are opened and closed

[^1]: `floatTuple` can be enclosed in parenthesis (`(`) or braces (`[`), and the
    values may be delimited with `,`, `;`, or whitespace.
