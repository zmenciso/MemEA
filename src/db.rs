//! Database management for MemEA memory peripheral components.
//!
//! This module provides functionality to manage and serialize component databases
//! containing memory cells, logic blocks, switches, and ADCs. The database supports
//! both YAML and JSON formats for storage and retrieval.

use dialoguer::Input;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use std::{fmt, fs, io, path};
use thiserror::Error;

use crate::{errorln, infoln, query, vprintln, Float, MemeaError, Mosaic};

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DBError {
    /// Indicates that a requested cell was not found in the database.
    #[error("Cannot find cell in database: {0}")]
    MissingCell(String),
    /// Indicates that no cells matching the criteria were found.
    #[error("Failed to find suitable cell: {0}")]
    NoSuitableCells(String),
    /// Indicates an unsupported file format was encountered.
    #[error("Unsupported file extension: {0}")]
    FileType(String),
}

/// Physical dimensions of a component including size and enclosure.
///
/// This struct represents the physical layout parameters of memory components,
/// including the core size and any required enclosure or spacing around it.
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Dims {
    /// Width and height of the component in micrometers [width, height].
    pub size: [Float; 2],
    /// Horizontal and vertical enclosure requirements in micrometers [x_enc, y_enc].
    pub enc: [Float; 2],
}

impl Default for Dims {
    fn default() -> Self {
        Self::new()
    }
}

impl Dims {
    /// Creates a new `Dims` instance with zero dimensions.
    ///
    /// # Returns
    /// A `Dims` struct with all values set to 0.0
    pub fn new() -> Dims {
        Dims {
            size: [0.0, 0.0],
            enc: [0.0, 0.0],
        }
    }

    /// Creates a new `Dims` instance with specified dimensions.
    ///
    /// # Arguments
    /// * `width` - Width of the component in micrometers
    /// * `height` - Height of the component in micrometers
    /// * `enc_x` - Horizontal enclosure requirement in micrometers
    /// * `enc_y` - Vertical enclosure requirement in micrometers
    ///
    /// # Returns
    /// A `Dims` struct with the specified values
    pub fn from(width: Float, height: Float, enc_x: Float, enc_y: Float) -> Dims {
        Dims {
            size: [width, height],
            enc: [enc_x, enc_y],
        }
    }

    /// Calculates the total area occupied by an array of components.
    ///
    /// # Arguments
    /// * `(n, m)` - Array dimensions as (rows, columns)
    ///
    /// # Returns
    /// Total area in square micrometers including enclosures
    pub fn area(&self, (n, m): Mosaic) -> Float {
        ((n as Float * self.size[0]) + (self.enc[0] * 2.0))
            * ((m as Float * self.size[1]) + (self.size[1] * 2.0))
    }

    /// Prints the dimensions in a human-readable format.
    ///
    /// Outputs the size and enclosure information to stdout with formatting.
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

/// Memory core cell parameters.
///
/// Represents the electrical and physical characteristics of a memory core cell,
/// including drive strengths for wordlines and bitlines.
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Core {
    /// Requred wordline drive strength
    pub dx_wl: Float,
    /// Required bitline drive strength
    pub dx_bl: Float,
    /// Physical dimensions of the core cell
    pub dims: Dims,
}

/// Logic block parameters.
///
/// Represents logic components such as decoders and control circuits with
/// their electrical and timing characteristics.
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Logic {
    /// Drive strength of the logic block
    pub dx: Float,
    /// Number of bits this logic block can decode
    pub bits: usize,
    /// Maximum operating frequency in Hz
    pub fs: Float,
    /// Physical dimensions of the logic block
    pub dims: Dims,
}

/// Switch component parameters.
///
/// Represents switching elements with their drive capability and voltage range.
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Switch {
    /// Drive strength of the switch
    pub dx: Float,
    /// Voltage range as [minimum, maximum] in volts
    pub voltage: [Float; 2],
    /// Physical dimensions of the switch
    pub dims: Dims,
}

/// Analog-to-Digital Converter (ADC) parameters.
///
/// Represents ADC components with their resolution and sampling characteristics.
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct ADC {
    /// Resolution as effective number of bits
    pub enob: Float,
    /// Maximum sampling frequency in Hz
    pub fs: Float,
    /// Physical dimensions of the ADC
    pub dims: Dims,
}

/// Component database containing all available peripheral elements.
///
/// The database stores collections of different component types (core cells,
/// logic blocks, switches, and ADCs) indexed by name. It supports serialization
/// to and from YAML and JSON formats.
///
/// # Examples
/// ```no_run
/// use memea::db::{Database, build_db};
/// use std::path::PathBuf;
///
/// // Load database from file
/// let db_path = PathBuf::from("components.yaml");
/// let db = build_db(&db_path).expect("Failed to load database");
///
/// // Access components
/// if let Some(core_cell) = db.core.get("sram_6t") {
///     println!("Found core cell with WL drive: {}", core_cell.dx_wl);
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    /// Collection of memory core cells indexed by name.
    pub core: HashMap<String, Core>,
    /// Collection of logic blocks indexed by name.
    pub logic: HashMap<String, Logic>,
    /// Collection of switch components indexed by name.
    pub switch: HashMap<String, Switch>,
    /// Collection of ADC components indexed by name.
    pub adc: HashMap<String, ADC>,
}

/// Prompts the user for input and parses it to the specified type.
///
/// This function displays a formatted prompt and continues asking for input
/// until a valid value of type `T` is entered.
///
/// # Arguments
/// * `message` - The prompt message to display to the user
///
/// # Returns
/// A value of type `T` parsed from user input
///
/// # Type Parameters
/// * `T` - The type to parse the input into (must implement `FromStr`)
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
    /// Creates a new empty database.
    ///
    /// # Returns
    /// A `Database` with empty collections for all component types
    pub fn new() -> Database {
        Database {
            core: HashMap::new(),
            logic: HashMap::new(),
            switch: HashMap::new(),
            adc: HashMap::new(),
        }
    }

    /// Adds a new ADC component to the database via interactive prompts.
    ///
    /// # Arguments
    /// * `name` - Name identifier for the ADC
    /// * `dims` - Physical dimensions of the ADC
    pub fn add_adc(&mut self, name: &str, dims: Dims) {
        let enob: Float = prompt("Bits");
        let fs: f32 = prompt("Sampling rate");

        let adc = ADC { enob, fs, dims };
        self.adc.insert(name.to_string(), adc);
    }

    /// Adds a new core cell to the database via interactive prompts.
    ///
    /// # Arguments
    /// * `name` - Name identifier for the core cell
    /// * `dims` - Physical dimensions of the core cell
    pub fn add_core(&mut self, name: &str, dims: Dims) {
        let dx_wl: f32 = prompt::<f32>("WL drive strength");
        let dx_bl: f32 = prompt::<f32>("BL drive strength");

        let core = Core { dx_wl, dx_bl, dims };
        self.core.insert(name.to_string(), core);
    }

    /// Adds a new logic block to the database via interactive prompts.
    ///
    /// # Arguments
    /// * `name` - Name identifier for the logic block
    /// * `dims` - Physical dimensions of the logic block
    pub fn add_logic(&mut self, name: &str, dims: Dims) {
        let dx: f32 = prompt::<f32>("Drive strength");
        let bits: usize = prompt::<usize>("Decoding bits");
        let fs: f32 = prompt::<f32>("Sampling rate");

        let logic = Logic { dx, bits, fs, dims };
        self.logic.insert(name.to_string(), logic);
    }

    /// Adds a new switch component to the database via interactive prompts.
    ///
    /// # Arguments
    /// * `name` - Name identifier for the switch
    /// * `dims` - Physical dimensions of the switch
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

    /// Saves the database to a file in YAML or JSON format.
    ///
    /// The output format is determined by the file extension (.yaml/.yml for YAML,
    /// .json for JSON).
    ///
    /// # Arguments
    /// * `filename` - Path where the database should be saved
    /// * `verbose` - Whether to print verbose output about the save operation
    ///
    /// # Returns
    /// * `Ok(())` - Database was successfully saved
    /// * `Err(MemeaError)` - File I/O error or unsupported format
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

/// Enumeration of component types available in the database.
#[derive(Hash, Eq, PartialEq, Serialize, Debug)]
pub enum CellType {
    /// Memory core cell type.
    Core,
    /// Logic block type.
    Logic,
    /// Analog-to-Digital Converter type.
    ADC,
    /// Switch component type.
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

/// Writes a database to file with overwrite confirmation if the file exists.
///
/// # Arguments
/// * `db` - Database to write
/// * `filename` - Target file path
/// * `verbose` - Whether to show verbose output
///
/// # Returns
/// * `Ok(())` - Database was successfully written
/// * `Err(MemeaError)` - File operation failed or user canceled
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

/// Checks if a file path has a supported extension for database files.
///
/// # Arguments
/// * `path` - File path to validate
///
/// # Returns
/// `true` if the extension is supported (yaml, yml, json), `false` otherwise
pub fn valid_ext(path: &str) -> bool {
    let allowed = ["yaml", "yml", "json"]; // allowed extensions

    let path = path::Path::new(path);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => allowed.contains(&ext.to_lowercase().as_str()),
        None => false, // No extension
    }
}

/// Builds a database by deserializing from a YAML or JSON file.
///
/// # Arguments
/// * `filename` - Path to the database file to load
///
/// # Returns
/// * `Ok(Database)` - Successfully loaded database
/// * `Err(MemeaError)` - File I/O error, parsing error, or unsupported format
///
/// # Examples
/// ```no_run
/// use memea::db::build_db;
/// use std::path::PathBuf;
///
/// let db_path = PathBuf::from("my_components.yaml");
/// match build_db(&db_path) {
///     Ok(database) => println!("Loaded {} core cells", database.core.len()),
///     Err(e) => eprintln!("Failed to load database: {}", e),
/// }
/// ```
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
