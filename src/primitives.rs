use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};
use thiserror::Error;

use crate::MemeaError;
use crate::{Float, Mosaic, Range};

pub type CellList = HashMap<String, Cell>;

#[derive(Debug, Error)]
pub enum DBError {
    #[error("Cannot find database, or it is invalid: {0}")]
    InvalidDatabase(CellType),

    #[error("Cell is not of type {0}")]
    InvalidCellType(CellType),

    #[error("Unknown cell type: {0}")]
    UnknownCellType(String),

    #[error("Cannot find cell in database: {0}")]
    MissingCell(String),

    #[error("Malformed cell type definition line: {0}")]
    InvalidCellDefinition(String),

    #[error("Malformed property line: {0}")]
    InvalidCellProperty(String),

    #[error("Failed to find suitable cell {0} in database")]
    NoSuitableCells(CellType),

    #[error("Value does not contain two floats: {0}")]
    TupleParseError(String),
}

#[derive(Debug)]
pub struct DB {
    cells: HashMap<CellType, CellList>,
}

impl Default for DB {
    fn default() -> Self {
        Self::new()
    }
}

impl DB {
    pub fn new() -> DB {
        DB {
            cells: HashMap::new(),
        }
    }

    /// Insert cell into database
    ///
    /// # Arguments
    /// * `name` - Pointer to string containing cell name
    /// * `kind` - Type of cell (switch, adc, etc.) as `CellType`
    /// * `cell` - Cell to insert
    fn insert(&mut self, name: &str, kind: CellType, cell: Cell) {
        if let Some(d) = self.cells.get_mut(&kind) {
            d.insert(name.to_owned(), cell);
        } else {
            let mut new: HashMap<String, Cell> = HashMap::new();
            new.insert(name.to_owned(), cell);
            self.cells.insert(kind, new);
        }
    }

    /// Add new cell to database
    ///
    /// # Arguments
    /// * `name` - Pointer to string containing cell name
    /// * `cell` - Cell to insert
    pub fn update(&mut self, name: &str, cell: Cell) {
        match cell {
            Cell::Core(_) => {
                Self::insert(self, name, CellType::Core, cell);
            }
            Cell::Logic(_) => {
                Self::insert(self, name, CellType::Logic, cell);
            }
            Cell::ADC(_) => {
                Self::insert(self, name, CellType::ADC, cell);
            }
            Cell::Switch(_) => {
                Self::insert(self, name, CellType::Switch, cell);
            }
        }
    }

    /// Retrieve a list of cells based on specified type
    ///
    /// # Arguments
    /// * `kind` - Which type of cells to return.  Filters output list
    pub fn retrieve(&self, kind: CellType) -> Result<&CellList, DBError> {
        self.cells.get(&kind).ok_or(DBError::InvalidDatabase(kind))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Dims {
    width: Float,
    height: Float,
    enc: Float,
}

impl Default for Dims {
    fn default() -> Self {
        Self::new()
    }
}

impl Dims {
    pub fn new() -> Dims {
        Dims {
            width: 0.0,
            height: 0.0,
            enc: 0.0,
        }
    }

    pub fn area(self, (n, m): Mosaic) -> Float {
        ((m as Float * self.width) + self.enc) * ((n as Float * self.height) + self.enc)
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Cell {
    Core(Core),
    Logic(Logic),
    ADC(ADC),
    Switch(Switch),
}

impl Cell {
    pub fn area(&self, mos: Mosaic) -> Float {
        match self {
            Cell::Core(core) => core.dims.area(mos),
            Cell::Logic(logic) => logic.dims.area(mos),
            Cell::ADC(adc) => adc.dims.area(mos),
            Cell::Switch(switch) => switch.dims.area(mos),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum CellType {
    Core,
    Logic,
    ADC,
    Switch,
}

pub const CELLS: [&str; 4] = ["core", "logic", "adc", "switch"];

impl fmt::Display for CellType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CellType::ADC => write!(f, "ADC"),
            CellType::Core => write!(f, "Core"),
            CellType::Logic => write!(f, "Logic"),
            CellType::Switch => write!(f, "Switch"),
        }
    }
}

pub trait Geometry {
    fn dims(&self) -> &Dims;
    fn as_any(&self) -> &dyn Any;
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Core {
    pub dims: Dims,
    pub dx_wl: Float,
    pub dx_bl: Float,
}

impl Geometry for Core {
    fn dims(&self) -> &Dims {
        &self.dims
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Logic {
    pub dims: Dims,
    pub dx: Float,
    pub fs: Float,
    pub bits: usize,
}

impl Geometry for Logic {
    fn dims(&self) -> &Dims {
        &self.dims
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Switch {
    pub dims: Dims,
    pub dx: Float,
    pub voltage: Range,
}

impl Geometry for Switch {
    fn dims(&self) -> &Dims {
        &self.dims
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct ADC {
    pub dims: Dims,
    pub bits: usize,
    pub fs: Float,
}
impl Geometry for ADC {
    fn dims(&self) -> &Dims {
        &self.dims
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn parse_range(line: &str) -> Result<Range, MemeaError> {
    let (min, max) = line
        .trim()
        .trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != ',' && c != ';' && c != '-')
        .split_once(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .ok_or(DBError::TupleParseError(line.to_string()))?;

    let min: Float = min.trim().parse::<Float>()?;
    let max: Float = max.trim().parse::<Float>()?;

    Ok(Range { min, max })
}

/// Add a property to a given cell
///
/// # Arguments
/// * `cell` - Cell to update
/// * `line` - Text line to read for adding properties
fn update_cell(cell: &mut Cell, line: &str) -> Result<(), MemeaError> {
    let (target, value) = line
        .trim()
        .split_once(|c: char| c == ':' || c == '=' || c.is_whitespace())
        .ok_or(DBError::InvalidCellProperty(line.to_string()))?;

    let target = target.trim().to_lowercase();
    let value = value.trim();

    match cell {
        Cell::Core(core) => match target.as_str() {
            "dx_wl" => core.dx_wl = value.parse::<Float>()?,
            "dx_bl" => core.dx_bl = value.parse::<Float>()?,
            "width" => core.dims.width = value.parse::<Float>()?,
            "height" => core.dims.height = value.parse::<Float>()?,
            "enc" => core.dims.enc = value.parse::<Float>()?,
            _ => {}
        },
        Cell::Logic(logic) => match target.as_str() {
            "dx" => logic.dx = value.parse::<Float>()?,
            "bits" => logic.bits = value.parse::<usize>()?,
            "fs" => logic.fs = value.parse::<Float>()?,
            "width" => logic.dims.width = value.parse::<Float>()?,
            "height" => logic.dims.height = value.parse::<Float>()?,
            "enc" => logic.dims.enc = value.parse::<Float>()?,
            _ => {}
        },
        Cell::Switch(switch) => match target.as_str() {
            "dx" => switch.dx = value.parse::<Float>()?,
            "voltage" => switch.voltage = parse_range(value)?,
            "width" => switch.dims.width = value.parse::<Float>()?,
            "height" => switch.dims.height = value.parse::<Float>()?,
            "enc" => switch.dims.enc = value.parse::<Float>()?,
            _ => {}
        },
        Cell::ADC(adc) => match target.as_str() {
            "bits" => adc.bits = value.parse::<usize>()?,
            "fs" => adc.fs = value.parse::<Float>()?,
            "width" => adc.dims.width = value.parse::<Float>()?,
            "height" => adc.dims.height = value.parse::<Float>()?,
            "enc" => adc.dims.enc = value.parse::<Float>()?,
            _ => {}
        },
    }

    Ok(())
}

/// Builds a cell database from an input file
///
/// # Arguments
/// * `filename` - Path for the file to read
pub fn build_db(filename: &std::path::PathBuf) -> Result<DB, MemeaError> {
    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    let mut db = DB::new();
    let mut curr: Option<Cell> = None;
    let mut name = String::new();

    for line in rdr.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // New cell
        if CELLS.iter().any(|&t| line.to_lowercase().contains(t)) {
            // Push previous cell
            if let Some(cell) = curr.take() {
                db.update(&name, cell);
            }

            let (kind, id) = line
                .split_once(|c: char| c == ':' || c == '=' || c.is_whitespace())
                .ok_or(DBError::InvalidCellDefinition(line.to_string()))?;

            let (kind, id) = (kind.trim(), id.trim());

            curr = match kind.to_lowercase().as_str() {
                "core" => Some(Cell::Core(Core {
                    dims: Dims::new(),
                    dx_wl: 0.0,
                    dx_bl: 0.0,
                })),
                "logic" => Some(Cell::Logic(Logic {
                    dims: Dims::new(),
                    dx: 0.0,
                    fs: 0.0,
                    bits: 0,
                })),
                "adc" => Some(Cell::ADC(ADC {
                    dims: Dims::new(),
                    fs: 0.0,
                    bits: 0,
                })),
                "switch" => Some(Cell::Switch(Switch {
                    dims: Dims::new(),
                    voltage: Range { min: 0.0, max: 0.0 },
                    dx: 0.0,
                })),
                _ => return Err(DBError::UnknownCellType(kind.to_string()).into()),
            };

            name = id.to_owned();
        } else if let Some(cell) = &mut curr {
            update_cell(cell, line)?;
        }
    }

    // Push last cell
    if let Some(cell) = curr {
        db.update(&name, cell);
    }

    Ok(db)
}
