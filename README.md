# MemEA

MemEA is a simple tool for estimating the area breakdown of memory arrays.
It supports multiple array types (e.g. CAM, SRAM, compute-in-memory) and
emerging devices (e.g. FeFET).

[Read the full documentation here!](https://zmenciso.github.io/MemEA/memea/)

## Installation

Install the latest version of `rustup`, then run:

```bash
cargo build --release
```

The output executable will be `target/release/memea`.

## Usage

MemEA requires two inputs: **1)** a configuration file that describes the memory
array and **2)** a database of cells and peripheral circuits. Both the
configuration files and the cell database can be written in YAML or JSON. MemEA
also accepts **multiple configuration files**, which will be compared against
each other after running.

Command line options:

- `-e` or `--export` `[FILENAME]`: Output results to file in CSV/JSON/YAML
  format (chosen from extension)
- `-a` or `--area-only`: Only output total area (automatically toggles `-q`)
- `-q` or `--quiet`: Suppress nonessential messages
- `-d` or `--db`: Specify database (default: `./data/db.yaml`)
- `--autoscale` `[FROM]` `[TO]`: Use built-in transistor scaling data to scale
  area from source technology node (e.g. `65`) to target technology node (e.g.
  `22`)
- `--scale` `[VALUE]`: Manually specify a scaling value to scale area (e.g.
  `0.124`)

### Memory Configuration

Each memory configuration is written in YAML, and a full list of options is
provided below:

| Option | Type           | Description                                                                                            | Example           |
| ------ | -------------- | ------------------------------------------------------------------------------------------------------ | ----------------- |
| `n`    | `int`          | Number of rows                                                                                         | `64`              |
| `m`    | `int`          | Number of columns                                                                                      | `64`              |
| `bl`   | `array[float]` | Required bitline voltages.                                                                             | `[1, 2, 0, -1]`   |
| `wl`   | `array[float]` | Required wordline voltages.                                                                            | `[4, 2.5, 0]`     |
| `well` | `array[float]` | Required well voltages (to bias a row-wise, column-wise or full-array deep n-well).                    | `[0, 4]`          |
| `cell` | `string`       | Which in the database to use as the memory cell.                                                       | `2FeFET_TCAM_100` |
| `enob` | `int`          | Minimum ENOB for downstream ADCs (also supports sense-amplifiers and other single-bit data conversion) | `1`               |
| `fs`   | `float`        | ADC sampling rate                                                                                      | `1e9`             |
| `adcs` | `int`          | Number of ADCs per array                                                                               | `64`              |

"Bitline" and "wordline" represent abstract vertical and horizontal lines,
respectively. If more lines are needed (e.g. bitline **and** senseline, a cell
representing an entire word with many bitlines), then repeat voltages in the
appropriate line. For example:

```yaml
bl: [4, 4, 2.5, 0, 0]
```

An example configuration is also available: `examples/config.yaml`.

### Database

`memea` has a **database generator** that builds the database from LEF and GDS
files. For more information, scroll to [**Database
Generator**](#database-generator). Otherwise, read on for writing the database
file manually:

The database has four types of circuits: `core`, `logic`, `switch`, and `adc`,
which should be the four topmost keys in the file. Nested within each type key
are the cells themselves. For example:

```yaml
switch:
  TXGD16:
    voltage: [0, 1.3]
    dx: 16
    dims:
      size: [1.156, 0.995]
      enc: [0.2, 0.2]
```

An example database is also available: `examples/db.yaml`.

All types of circuits require the following geometric properties:

| Option | Type         | Description                                                                                                                        | Example         |
| ------ | ------------ | ---------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `size` | `floatTuple` | Minimum horizontal and vertical space between instances in an array (pitch), in μm                                                 | `[0.432, 0.12]` |
| `enc`  | `floatTuple` | horizontal and vertical spacing required between this circuit and any other circuit (e.g. considering well-to-well spacing), in μm | `[1.48, 2]`     |

> Check back for a diagram explaining these properties

> **note**: `memea` assumes that cells cannot be rotated, as this is the case in
> most advanced manufacturing nodes.

Then, each of the four types has additional properties:

#### `core`

| Option  | Type    | Description                                        | Example |
| ------- | ------- | -------------------------------------------------- | ------- |
| `dx_bl` | `float` | Relative bitline drive strength required per-cell  | `0.25`  |
| `dx_wl` | `float` | Relative wordline drive strength required per-cell | `0.25`  |

#### `logic`

For logic, include any required signal buffers and inverters. For example, a
decoder driving transmission gates will require complimentary outputs (extra
inverters).

| Option | Type    | Description                                                                    | Example |
| ------ | ------- | ------------------------------------------------------------------------------ | ------- |
| `fs`   | `float` | Maximum operating speed of the logic                                           | `1e9`   |
| `dx`   | `float` | Relative drive strength output of the logic                                    | `6`     |
| `bits` | `uint`  | Number of control bits (i.e. a 2-bit logic circuit can drive up to 4 switches) | `2`     |

#### `switch`

For all switches, include any required well biasing contacts. For high voltage
switches, include the level shifters required to drive them from core voltage
logic.

| Option    | Type         | Description                                                 | Example      |
| --------- | ------------ | ----------------------------------------------------------- | ------------ |
| `voltage` | `floatTuple` | Maximum voltage the switch can drive before oxide breakdown | `[0.8, 1.3]` |
| `dx`      | `float`      | Relative drive strength of the switch                       | `16`         |

#### `adc`

For ADCs, include any switches that might be needed to isolate the circuit from
high voltage.

| Option | Type    | Description                      | Example |
| ------ | ------- | -------------------------------- | ------- |
| `bits` | `float` | ENOB of the ADC                  | `6.2`   |
| `fs`   | `float` | Maximum sampling rate of the ADC | `2e9`   |

### Database Generator

Invoke the database generator with the `-b` or `--build-db` argument, then
follow the interactive prompts. You will need to export your cell library as a
LEF file (File > Export > LEF in Virtuoso) _and_ as a GDS file (File > Export >
Stream in Virtuoso). The database generator can be run without a GDS file by
leaving the prompt blank, but the resulting cell database will not include
enclosures.

## Helper Scripts

Check back for scripts that automate generating common configuration runs, such
as:

- Multiple array dimensions for the same memory configuration
- Write voltage sweeps

## Planned Updates

### New Features

- Combine like elements in area breakdown (i.e. only report unique
  occurrences)
- Produce example/estimated floorplan for each configuration
- Support for shared peripherals across multiple memory arrays (e.g. BL
  drivers alternate between driving two different arrays)
- Stacking multiple switches per row/column to achieve the required drive
  strength
- GUI mode
- Better configuration system (i.e. specify variable sweeps within the
  configuration file)
- Automatic enob calculation option based on the number of rows (assuming
  1-bit per cell): `enob auto`
