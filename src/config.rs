//! Configuration management for MemEA memory peripheral estimation.
//!
//! This module provides functionality to read and manage memory configurations
//! from YAML files. Each configuration specifies memory array parameters,
//! cell types, voltages, and ADC settings used for peripheral estimation.

use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;

use crate::{errorln, Float, MemeaError};

/// A collection of memory configurations indexed by name.
type Configs = HashMap<String, Config>;

/// Errors that can occur during configuration processing.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Indicates that a required configuration option was not found.
    #[error("Cannot find option in config: {0}")]
    MissingOption(String),
}

/// Represents one memory configuration for peripheral estimation.
///
/// This struct is typically deserialized from YAML or JSON using `serde`. It
/// contains both essential and optional parameters that control the peripheral
/// estimation process.
///
/// # Examples
///
/// ```yaml
/// name: 64-64
/// n: 128
/// m: 64
/// bl: [1, 2, 0]
/// wl: [4, 2.5, 0, 1]
/// well: [0, 4]
/// cell: 1FeFET_100
/// enob: 1
/// fs: 1e9
/// adcs: 64
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Name of the configuration. If not supplied, the file path will be used.
    pub name: Option<String>,

    /// Number of rows in the memory array.
    pub n: usize,
    /// Number of columns in the memory array.
    pub m: usize,
    /// Memory cell type to use for estimation.
    pub cell: String,

    /// Bitline voltages
    pub bl: Option<Vec<Float>>,
    /// Wordline voltages
    pub wl: Option<Vec<Float>>,
    /// Voltages required for well biasing
    pub well: Option<Vec<Float>>,

    /// Number of downstream analog-to-digital converters.
    pub adcs: Option<usize>,
    /// Number of bits required for ADCs.
    pub bits: Option<usize>,
    /// Sampling rate of the ADCs in Hz.
    pub fs: Option<Float>,

    /// Additional configuration options as key-value pairs.
    pub options: Option<HashMap<String, String>>,
}

/// Deserializes a configuration from a YAML file.
///
/// # Arguments
/// * `filename` - Path of the YAML file to read
///
/// # Returns
/// * `Ok(Config)` - Successfully parsed configuration
/// * `Err(MemeaError)` - File I/O error or YAML parsing error
///
/// # Examples
/// ```no_run
/// use std::path::PathBuf;
/// # use memea::config::read;
///
/// let config_path = PathBuf::from("config.yaml");
/// let config = read(&config_path).expect("Failed to read config");
/// ```
fn read(filename: &std::path::PathBuf) -> Result<Config, MemeaError> {
    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);
    let config: Config = serde_yaml::from_reader(rdr)?;

    Ok(config)
}

/// Reads multiple configuration files and returns them indexed by name.
///
/// This function attempts to read all provided configuration files. If a file
/// fails to parse, an error is logged and that file is skipped. The resulting
/// HashMap uses either the configured name or the file path as the key.
///
/// # Arguments
/// * `paths` - Vector of configuration file paths to read
///
/// # Returns
/// * `HashMap<String, Config>` - Successfully parsed configurations indexed by name
///
/// # Examples
/// ```no_run
/// use std::path::PathBuf;
/// # use memea::config::read_all;
///
/// let paths = vec![
///     PathBuf::from("config1.yaml"),
///     PathBuf::from("config2.yaml"),
/// ];
/// let configs = read_all(&paths);
/// println!("Loaded {} configurations", configs.len());
/// ```
pub fn read_all(paths: &Vec<PathBuf>) -> Configs {
    let mut configs: Configs = HashMap::new();
    for c in paths {
        match read(c) {
            Ok(r) => {
                let name = match &r.name {
                    Some(s) => s.clone(),
                    None => c.to_string_lossy().into(),
                };

                configs.insert(name, r);
            }
            Err(e) => errorln!("Failed to read config {:?} ({})", &c, e),
        }
    }

    configs
}
