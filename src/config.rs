use std::collections::HashMap;
use std::path::PathBuf;
use std::io;
use std::io::{BufRead, BufReader};
use std::fs;

use crate::eliteral;
use crate::{Value, ValueTypes};
use crate::decode;

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
            path: name.to_string_lossy().into_owned()
        }
    }

    /// Inserts an option into `Config`, returns `Some(x)` if `key` is already in `Config`, 
    /// where `x` is the previous value.  Returns None if `key` is a new option
    ///
    /// # Arguments
    /// * `key` - Name of the option to insert
    /// * `value` - Value of the option (of type Target)
    ///
    /// # Panics
    /// Unrecognized options
    pub fn update(&mut self, key: &str, value: &str) -> Option<Value> {
        let option = key.to_owned();
        match key {
            "n"     => { self.config.insert(option, decode(value, ValueTypes::Float)) },
            "m"     => { self.config.insert(option, decode(value, ValueTypes::Float)) },
            "adcs"  => { self.config.insert(option, decode(value, ValueTypes::Float)) },
            "enob"  => { self.config.insert(option, decode(value, ValueTypes::Float)) },
            "fs"    => { self.config.insert(option, decode(value, ValueTypes::Float)) },
            "bl"    => { self.config.insert(option, decode(value, ValueTypes::FloatVec)) },
            "wl"    => { self.config.insert(option, decode(value, ValueTypes::FloatVec)) },
            "well"  => { self.config.insert(option, decode(value, ValueTypes::FloatVec)) },
            "cell"  => { self.config.insert(option, decode(value, ValueTypes::String)) },
            _       => { panic!(eliteral!("Unrecognized option")) }
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }

    pub fn retrieve(&self, key: &str) -> &Value {
        self.config.get(key)
            .expect(eliteral!("Could not retrieve option"))
    }
}

/// Reads configuration from file and returns a `Config` struct result.
///
/// # Arguments
/// * `filename` - Path of the file to read (`PathBuf`)
pub fn read(filename: &std::path::PathBuf) -> Result<Config, io::Error> {
    let mut config = Config::new(filename);

    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    for line in rdr.lines() {
        let line = line.expect(eliteral!("Could not parse line"));
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || (line.len() == 0) { continue; }

        if let Some((param, value)) = line.split_once(':') {
            config.update(param.trim(), value.trim());
        } else {
            eprintln!(eliteral!("Delimeter not found in string."));
        }
    }

    Ok(config)
}
