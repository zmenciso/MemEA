use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use thiserror::Error;

use crate::decode;
use crate::{warnln, MemeaError};
use crate::{Value, ValueTypes};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Cannot find option in config: {0}")]
    MissingOption(String),
}

#[derive(Debug)]
pub struct Config {
    config: HashMap<String, Value>,
    pub path: String,
}

impl Config {
    /// Create new Config from file path
    fn new(name: &PathBuf) -> Config {
        Config {
            config: HashMap::new(),
            path: name.to_string_lossy().into_owned(),
        }
    }

    /// Inserts an option into `Config`, returns `Some(x)` if `key` is already in `Config`,
    /// where `x` is the previous value.  Returns None if `key` is a new option, or if the option
    /// is not recognized
    ///
    /// # Arguments
    /// * `key` - Name of the option to insert
    /// * `value` - Value of the option (of type Target)
    pub fn update(&mut self, key: &str, value: &str) -> Result<Option<Value>, MemeaError> {
        let option = key.to_owned();

        match key {
            "n" | "m" | "adcs" | "enob" => Ok(self
                .config
                .insert(option, decode(value, ValueTypes::Usize)?)),
            "fs" => Ok(self
                .config
                .insert(option, decode(value, ValueTypes::Float)?)),
            "bl" | "wl" | "well" => Ok(self
                .config
                .insert(option, decode(value, ValueTypes::FloatVec)?)),
            "cell" => Ok(self
                .config
                .insert(option, decode(value, ValueTypes::String)?)),
            _ => {
                warnln!("Unrecognized option {} (value {})", key, value);
                Ok(None)
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }

    pub fn retrieve(&self, key: &str) -> Result<&Value, ConfigError> {
        self.config
            .get(key)
            .ok_or(ConfigError::MissingOption(key.to_string()))
    }
}

/// Reads configuration from file and returns a `Config` struct result.
///
/// # Arguments
/// * `filename` - Path of the file to read (`PathBuf`)
pub fn read(filename: &std::path::PathBuf) -> Result<Config, MemeaError> {
    let mut config = Config::new(filename);

    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    for line in rdr.lines() {
        let line = line?;
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((param, value)) = line.split_once(':') {
            config.update(param.trim(), value.trim())?;
        } else {
            warnln!("Delimeter not found in string: {}", line);
        }
    }

    Ok(config)
}
