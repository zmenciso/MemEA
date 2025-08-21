use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;

use crate::{errorln, Float, MemeaError};

type Configs = HashMap<String, Config>;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Cannot find option in config: {0}")]
    MissingOption(String),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: Option<String>,

    // Essential
    pub n: usize,
    pub m: usize,
    pub cell: String,

    // Optional
    pub bl: Option<Vec<Float>>,
    pub wl: Option<Vec<Float>>,
    pub well: Option<Vec<Float>>,

    // ADC
    pub adcs: Option<usize>,
    pub enob: Option<usize>,
    pub fs: Option<Float>,

    // Additional options
    pub options: Option<HashMap<String, String>>,
}

impl Config {}

/// Reads configuration from file and returns a `Config` struct result.
///
/// # Arguments
/// * `filename` - Path of the file to read (`PathBuf`)
fn read(filename: &std::path::PathBuf) -> Result<Config, MemeaError> {
    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);
    let config: Config = serde_yaml::from_reader(rdr)?;

    Ok(config)
}

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
