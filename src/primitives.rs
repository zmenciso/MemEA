use std::collections::HashMap;
use std::any::Any;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::fmt;

use crate::eliteral;
use crate::{Float, Mosaic};
use crate::{parse_float, parse_usize};

pub type CellList = HashMap<String, Cell>;

#[derive(Debug)]
pub struct DB {
    cells: HashMap<CellType, CellList>, 
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
        }
        else {
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
    ///
    /// # Panics
    /// No cells of specified type found in database
    pub fn retrieve(&self, kind: CellType) -> &CellList {
        self.cells.get(&kind)
            .expect(eliteral!("No cells found in database"))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Dims {
    width: Float,
    height: Float,
    enc: Float,
}

impl Dims {
    pub fn new() -> Dims {
        Dims {
            width: 0.0,
            height: 0.0,
            enc: 0.0
        }
    }

    pub fn area(self, (n, m): Mosaic) -> Float {
        ((m as Float * self.width) + self.enc) * 
            ((n as Float * self.height) + self.enc)
    }
}

#[derive (PartialEq, Debug, Copy, Clone)]
pub enum Cell {
    Core(Core),
    Logic(Logic),
    ADC(ADC),
    Switch(Switch)
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

#[derive (Hash, Eq, PartialEq, Debug)]
pub enum CellType {
    Core,
    Logic,
    ADC,
    Switch
}

impl fmt::Display for CellType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
           CellType::ADC => write!(f, "ADC"),
           CellType::Core => write!(f, "Core"),
           CellType::Logic => write!(f, "Logic"),
           CellType::Switch => write!(f, "Switch")
       }
    }
}

pub trait Geometry {
    fn dims(&self) -> &Dims;
    fn as_any(&self) -> &dyn Any;
}

#[derive (PartialEq, Debug, Copy, Clone)]
pub struct Core {
    pub dims: Dims,
    pub dx_wl: Float,
    pub dx_bl: Float
}

impl Geometry for Core {
    fn dims(&self) -> &Dims { &self.dims }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive (PartialEq, Debug, Copy, Clone)]
pub struct Logic {
    pub dims: Dims,
    pub dx: Float,
    pub fs: Float,
    pub bits: usize,
}

impl Geometry for Logic {
    fn dims(&self) -> &Dims { &self.dims }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive (PartialEq, Debug, Copy, Clone)]
pub struct Switch {
    pub dims: Dims,
    pub dx: Float,
    pub voltage: Float
}

impl Geometry for Switch {
    fn dims(&self) -> &Dims { &self.dims }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive (PartialEq, Debug, Copy, Clone)]
pub struct ADC {
    pub dims: Dims,
    pub bits: usize,
    pub fs: Float
}
impl Geometry for ADC {
    fn dims(&self) -> &Dims { &self.dims }
    fn as_any(&self) -> &dyn Any { self }
}

/// Add a property to a given cell
///
/// # Arguments
/// * `cell` - Cell to update
/// * `line` - Text line to read for adding properties
///
/// # Panics
/// No property or value found on database line
fn update_cell(cell: &mut Cell, line: &str) {
    let mut iter = line.split_whitespace();
    let target = iter.next()
        .expect(eliteral!("No property found on database line"))
        .to_lowercase();
    let value = iter.next()
        .expect(eliteral!("No value found on database line"))
        .to_lowercase();

    match cell {
        Cell::Core(core) => {
            match target.as_str() {
                "dx_wl" => { core.dx_wl = parse_float(&value) },
                "dx_bl" => { core.dx_bl = parse_float(&value) },
                "width" => { core.dims.width = parse_float(&value) },
                "height" => { core.dims.height = parse_float(&value) },
                "enc" => { core.dims.enc = parse_float(&value) },
                _ => {},
            }
        },
        Cell::Logic(logic) => {
            match target.as_str() {
                "dx" => { logic.dx = parse_float(&value) },
                "bits" => { logic.bits = parse_usize(&value) },
                "fs" => { logic.fs = parse_float(&value) },
                "width" => { logic.dims.width = parse_float(&value) },
                "height" => { logic.dims.height = parse_float(&value) },
                "enc" => { logic.dims.enc = parse_float(&value) },
                _ => {},
            }
        },
        Cell::Switch(switch) => {
            match target.as_str() {
                "dx" => { switch.dx = parse_float(&value) },
                "voltage" => { switch.voltage = parse_float(&value) },
                "width" => { switch.dims.width = parse_float(&value) },
                "height" => { switch.dims.height = parse_float(&value) },
                "enc" => { switch.dims.enc = parse_float(&value) },
                _ => {},
            }
        },
        Cell::ADC(adc) => {
            match target.as_str() {
                "bits" => { adc.bits = parse_usize(&value) },
                "fs" => { adc.fs = parse_float(&value) },
                "width" => { adc.dims.width = parse_float(&value) },
                "height" => { adc.dims.height = parse_float(&value) },
                "enc" => { adc.dims.enc = parse_float(&value) },
                _ => {},
            }
        }
    }
}

/// Builds a cell database from an input file
///
/// # Arguments 
/// * `filename` - Path for the file to read
///
/// # Panics
/// Cannot decode line from file
/// Could not parse cell type definition
/// Invalid cell types
pub fn build_db(filename: &std::path::PathBuf) -> Result<DB, io::Error>{
    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    let mut db = DB::new();
    let mut curr: Option<Cell> = None;
    let mut name = String::new();

    for line in rdr.lines() {
        let line = line.expect(eliteral!("Could not decode line"));
        let line = line.trim();

        if line.starts_with('#') || (line.len() == 0) { continue; }

        // New cell
        if line.contains(':') {
            // Push previous cell
            if let Some(cell) = curr.take() {
                db.update(&name, cell);
            }

            let (kind, id) = line.split_once(':')
                .expect(eliteral!("Could not parse cell type definition"));

            let (kind, id) = (kind.trim(), id.trim());

            curr = match kind.to_lowercase().as_str() {
                "core" => Some(Cell::Core( Core{
                    dims: Dims::new(),
                    dx_wl: 0.0,
                    dx_bl: 0.0,
                })),
                "logic" => Some(Cell::Logic( Logic{
                    dims: Dims::new(),
                    dx: 0.0,
                    fs: 0.0,
                    bits: 0,
                })),
                "adc" => Some(Cell::ADC( ADC{
                    dims: Dims::new(),
                    fs: 0.0,
                    bits: 0
                })),
                "switch" => Some(Cell::Switch( Switch{
                    dims: Dims::new(),
                    voltage: 0.0,
                    dx: 0.0,
                })),
                _ => { panic!(eliteral!("Invalid cell type")) }
            };

            name = id.to_owned();
        }

        else if let Some(cell) = &mut curr {
            update_cell(cell, line);
        }
    }

    // Push last cell
    if let Some(cell) = curr {
        db.update(&name, cell);
    }

    Ok(db)
}
