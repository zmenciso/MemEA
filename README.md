# Periph_gen

Periph_gen is a simple tool for estimating the area breakdown of memory arrays.
It supports multiple array types (e.g. CAM, SRAM, compute-in-memory) and
emerging devices (e.g. FeFET).

# Installation

Install the latest version of `rustup`, then run:

```bash
cargo build --release
```

The output executable will be `target/release/periph_gen`.

# Usage

Periph_gen requires two inputs: **1)** a configuration file that describes the
memory array and **2)** a database of cells and peripheral circuits.  Both are
written in simple plain text files as key/value pairs.  A single colon `:`
separates the key from the value, and lines starting with `#` are ignored.

Periph_gen also accepts multiple configuration files, which will be compared
against each other after running.

Command line options:
  - `-e` or `--export`: Output results to file
  - `-a` or `--area-only`: Only output total area
  - `-q` or `--quiet`: Suppress nonessential messages
  - `-d` or `--db`: Specify database (default: `./data/db.txt`)

## Memory Configuration

The required options are `n` (the number of rows), `m` (the number of columns),
and `cell` (the memory cell).  Additional options are listed below:

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `n` | `int` | Number of rows | `64` |
| `m` | `int` | Number of columns | `64` |
| `bl` | `array[float]` | Required bitline voltages.  A voltage of -1 V indicates a high-Z mode. | `1, 2, 0, -1` |
| `wl` | `array[float]` | Required wordline voltages.  A voltage of -1 V indicates a high-Z mode. | `4, 2.5, 0` |
| `well` | `array[float]` | Required well voltages (to bias a row-wise, column-wise or full-array deep n-well). | `0, 4` |
| `cell` | `string` | Which in the database to use as the memory cell. | `2FeFET_TCAM_100` |
| `enob` | `float` | Minimum ENOB for downstream ADCs (also supports sense-amplifiers and other single-bit data conversion) | `1` |
| `fs` | `float` | ADC sampling rate | `1e9` |
| `adcs` | `int` | Number of ADCs per array | `64` |

An example configuration is also available: `./config/example.txt`.

## Database

The database has four types of circuits: `cell`, `logic`, `switch`, and `adc`.
Describing a circuit begins with the name of the circuit, followed by the
key/value pairs for the circuit properties.  The database file should end with
`ENDDB`, and the `type` property is mandatory for each circuit. For example:

```
TXGD16
	type: switch
	voltage: 1.3
	dx: 16
	spc_x: 1.156
	spc_y: 0.995
	enc: 0.490
```

An example database is also available: `./data/example.txt`.

All types of circuits require the following geometric properties:

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `spc_x` | `float` | Minimum horizontal space between instances in an array, in μm | `0.432` |
| `spc_y` | `float` | Minimum vertical space between instances in an array, in μm | `0.432` |
| `enc` | `float` | Spacing required between this circuit and any other circuit (e.g. considering well-to-well spacing), in μm | `1.48` |

> Check back for a diagram explaining these properties

Then, each of the four types has additional properties:

### `cell`

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `dx` | `float` | Relative drive strength required per-cell | `0.25` |

### `logic`

For logic, include any required signal buffers and inverters.  For example, a
decoder driving transmission gates will require complimentary outputs (extra
inverters).

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `fs` | `float` | Maximum operating speed of the logic | `1e9` |
| `dx` | `float` | Relative drive strength output of the logic | `6` |
| `bits` | `int` | Number of control bits (i.e. a 2-bit logic circuit can drive
up to 4 switches) | `2` |

### `switch`

For all switches, include any required well biasing contacts.  For high voltage
switches, include the level shifters required to drive them from core voltage
logic.

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `voltage` | `float` | Maximum voltage the switch can drive before oxide breakdown | `2.5` |
| `dx` | `float` | Relative drive strength of the switch | `16` |

### `adc`

| Option | Type | Description | Example |
|--------|------|-------------|---------|
| `bits` | `float` | ENOB of the ADC | `6.2` |
| `fs` | `float` | Maximum sampling rate of the ADC | `2e9` |
