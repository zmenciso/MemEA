use dialoguer::Input;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use std::{fmt, fs, io, path};
use thiserror::Error;

use crate::{errorln, infoln, query, vprintln, Float, MemeaError, Mosaic};

#[derive(Debug, Error)]
pub enum DBError {
    #[error("Cannot find cell in database: {0}")]
    MissingCell(String),
    #[error("Failed to find suitable cell: {0}")]
    NoSuitableCells(String),
    #[error("Unsupported file extension: {0}")]
    FileType(String),
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Dims {
    pub size: [Float; 2],
    pub enc: [Float; 2],
}

impl Default for Dims {
    fn default() -> Self {
        Self::new()
    }
}

impl Dims {
    pub fn new() -> Dims {
        Dims {
            size: [0.0, 0.0],
            enc: [0.0, 0.0],
        }
    }

    pub fn from(width: Float, height: Float, enc_x: Float, enc_y: Float) -> Dims {
        Dims {
            size: [width, height],
            enc: [enc_x, enc_y],
        }
    }

    pub fn area(&self, (n, m): Mosaic) -> Float {
        ((m as Float * self.size[0]) + (self.enc[0] * 2.0))
            * ((n as Float * self.size[1]) + (self.size[1] * 2.0))
    }

    pub fn dump(&self) {
        println!(
            "Size.......: {:.4} (width) by {:.4} (height)",
            self.size[0], self.size[1]
        );
        println!(
            "Enclosure..: {:.4} (horizontal) by {:.4} (vertical)",
            self.enc[0], self.enc[1]
        );
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Core {
    pub dx_wl: Float,
    pub dx_bl: Float,
    pub dims: Dims,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Logic {
    pub dx: Float,
    pub bits: usize,
    pub fs: Float,
    pub dims: Dims,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Switch {
    pub dx: Float,
    pub voltage: [Float; 2],
    pub dims: Dims,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct ADC {
    pub bits: usize,
    pub fs: Float,
    pub dims: Dims,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub core: HashMap<String, Core>,
    pub logic: HashMap<String, Logic>,
    pub switch: HashMap<String, Switch>,
    pub adc: HashMap<String, ADC>,
}

pub fn prompt<T>(message: &str) -> T
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    const WIDTH: usize = 20;

    let padded: String = if message.len() >= WIDTH {
        message.to_string()
    } else {
        let spacer = ".".repeat(WIDTH - message.len());
        format!("{message}{spacer}")
    };

    loop {
        let input: String = Input::new()
            .with_prompt(&padded)
            .interact_text()
            .unwrap_or_else(|_| {
                errorln!("Failed to read input");
                String::new()
            });

        match input.trim().parse::<T>() {
            Ok(val) => return val,
            Err(e) => {
                errorln!("Invalid input: {}", e);
                continue;
            }
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Database {
        Database {
            core: HashMap::new(),
            logic: HashMap::new(),
            switch: HashMap::new(),
            adc: HashMap::new(),
        }
    }

    pub fn add_adc(&mut self, name: &str, dims: Dims) {
        let bits: usize = prompt("Bits");
        let fs: f32 = prompt("Sampling rate");

        let adc = ADC { bits, fs, dims };
        self.adc.insert(name.to_string(), adc);
    }

    pub fn add_core(&mut self, name: &str, dims: Dims) {
        let dx_wl: f32 = prompt::<f32>("WL drive strength");
        let dx_bl: f32 = prompt::<f32>("BL drive strength");

        let core = Core { dx_wl, dx_bl, dims };
        self.core.insert(name.to_string(), core);
    }

    pub fn add_logic(&mut self, name: &str, dims: Dims) {
        let dx: f32 = prompt::<f32>("Drive strength");
        let bits: usize = prompt::<usize>("Decoding bits");
        let fs: f32 = prompt::<f32>("Sampling rate");

        let logic = Logic { dx, bits, fs, dims };
        self.logic.insert(name.to_string(), logic);
    }

    pub fn add_switch(&mut self, name: &str, dims: Dims) {
        let dx: f32 = prompt::<f32>("Drive strength");
        let vmin: f32 = prompt::<f32>("Minimum voltage");
        let vmax: f32 = prompt::<f32>("Maximum voltage");

        let switch = Switch {
            dx,
            voltage: [vmin, vmax],
            dims,
        };
        self.switch.insert(name.to_string(), switch);
    }

    pub fn save(&self, filename: &PathBuf, verbose: bool) -> Result<(), MemeaError> {
        let ext = filename
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();

        let mut file = fs::File::create(filename)?;

        match ext.as_str() {
            "yaml" | "yml" => serde_yaml::to_writer(&mut file, self)?,
            "json" => serde_json::to_writer_pretty(&mut file, self)?,
            other => {
                return Err(DBError::FileType(other.to_string()).into());
            }
        }

        vprintln!(
            verbose,
            "Wrote {} core cells, {} switches, {} logic cells, and {} ADCs to {:?}",
            self.core.len(),
            self.switch.len(),
            self.logic.len(),
            self.adc.len(),
            filename
        );

        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq, Serialize, Debug)]
pub enum CellType {
    Core,
    Logic,
    ADC,
    Switch,
}

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

pub fn write_db(db: &Database, filename: &PathBuf, verbose: bool) -> Result<(), MemeaError> {
    // If file already exists, prompt to overwrite
    if fs::metadata(filename).is_ok() {
        let allow = query(
            format!(
                "'{}' already exists. Overwrite?",
                filename.to_string_lossy()
            )
            .as_str(),
            true,
            crate::QueryDefault::Yes,
        )?;

        if !allow {
            infoln!("Aborting...");
            return Ok(());
        }
    }

    db.save(filename, verbose)
}

pub fn valid_ext(path: &str) -> bool {
    let allowed = ["yaml", "yml", "json"]; // allowed extensions

    let path = path::Path::new(path);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => allowed.contains(&ext.to_lowercase().as_str()),
        None => false, // No extension
    }
}

pub fn build_db(filename: &PathBuf) -> Result<Database, MemeaError> {
    let file = fs::File::open(filename)?;
    let rdr = io::BufReader::new(file);

    let ext = filename
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();

    let db = match ext.as_str() {
        "yaml" | "yml" => serde_yaml::from_reader(rdr)?,
        "json" => serde_json::from_reader(rdr)?,
        other => {
            return Err(DBError::FileType(other.to_string()).into());
        }
    };

    Ok(db)
}
