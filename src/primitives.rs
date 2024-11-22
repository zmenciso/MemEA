use std::collections::HashMap;
use std::fs;
use std::error::Error;
use std::io::{BufRead, BufReader};

#[derive(Debug)]
pub struct DB {
    pub cells: HashMap<String, Cell>,
    pub logic: HashMap<String, Cell>,
    pub adcs: HashMap<String, Cell>,
    pub switches: HashMap<String, Cell>,
}

impl DB {
    pub fn new() -> DB {
        DB {
            cells: HashMap::new(),
            logic: HashMap::new(),
            adcs: HashMap::new(),
            switches: HashMap::new(),
        }
    }

    pub fn insert(&mut self, kind: &str, target: &str, cell: Cell) {
        match kind {
            "adc" => { self.adcs.insert(target.to_string(), cell) },
            "switch" => { self.switches.insert(target.to_string(), cell) },
            "logic" => { self.logic.insert(target.to_string(), cell) },
            "cell" => { self.cells.insert(target.to_string(), cell) },
            _ => { self.cells.insert(target.to_string(), cell) }
        };
    }
}

#[derive(Debug, Copy, Clone)]
struct Dims {
    spc_x: f32,
    spc_y: f32,
    enc: f32,
}

impl Dims {
    pub fn new() -> Dims {
        Dims {
            spc_x: 0.0,
            spc_y: 0.0,
            enc: 0.0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Cell {
    dims: Dims,
    pub voltage: f32,
    pub dx: f32,
    pub bits: i32,
    pub fs: f32,
}

impl Cell {
    pub fn new() -> Cell {
        Cell {
            dims: Dims::new(),
            voltage: 0.0,
            dx: 0.0,
            bits: -1,
            fs: -1.0,
        }
    }

    // TODO: Implement error handling
    pub fn update(&mut self, key: &str, value: &str) {
        match key.to_lowercase().as_str() {
            "spc_x"     => { self.dims.spc_x = value.parse::<f32>().unwrap() },
            "spc_y"     => { self.dims.spc_y = value.parse::<f32>().unwrap() },
            "enc"       => { self.dims.enc= value.parse::<f32>().unwrap() },
            "voltage"   => { self.voltage = value.parse::<f32>().unwrap() },
            "dx"        => { self.dx = value.parse::<f32>().unwrap() },
            "bits"      => { self.bits = value.parse::<i32>().unwrap() },
            "fs"        => { self.fs = value.parse::<f32>().unwrap() },
            _           => { }
        }
    }

    pub fn area(self, n: i32, m: i32) -> f32 {
        ((m as f32 * self.dims.spc_x) + self.dims.enc) *
            ((n as f32 * self.dims.spc_y) + self.dims.enc)
    }

    pub fn rotate(&mut self) {
        let temp = self.dims.spc_x;

        self.dims.spc_x = self.dims.spc_y;
        self.dims.spc_y = temp;
    }
}

pub fn build_db(filename: &std::path::PathBuf) -> Result<DB, Box<dyn Error>>{
    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    let mut db = DB::new();

    let mut target = String::from("");
    let mut kind = String::from("");
    let mut temp = Cell::new();

    for line in rdr.lines() {
        let line = line.unwrap();
        let line = line.trim();

        if line.starts_with('#') || (line.len() == 0) { continue; }

        if !line.contains(':') {
            db.insert(&kind, &target, temp);
            target = line.to_string();
            continue;
        }

        if let Some((param, value)) = line.split_once(':') {
            if param.contains("type") {
                kind = value.trim().to_string(); 
                continue;
            }
            temp.update(param.trim(), value.trim());
        } else {
            eprintln!("Delimeter not found in string.");
        }
    }

    Ok(db)
}
