use std::error::Error;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::fs;

#[derive(Debug)]
pub struct Config {
    pub path: String,
    pub n: i32,
    pub m: i32,
    pub bl: Vec<f32>,
    pub wl: Vec<f32>,
    pub well: Vec<f32>,
    pub cell: String,
    pub enob: f32,
    pub fs: f32,
    pub adcs: i32,
}

impl Config {
    fn new(name: &PathBuf) -> Config {
        Config {
            path: name.to_string_lossy().into_owned(),
            n: 64,
            m: 64,
            bl: Vec::new(),
            wl: Vec::new(),
            well: Vec::new(),
            cell: String::new(),
            enob: -1.0,
            fs: -1.0,
            adcs: -1
        }
    }

    fn parse_vec(input: &str) -> Result<Vec<f32>, std::num::ParseFloatError> {
        let v: Vec<&str> = input.split(',').collect();
        v.iter().map(|x| x.trim().parse::<f32>()).collect()
    }

    pub fn update(&mut self, key: &str, value: &str) {
        match key.to_lowercase().as_str() {
            "n"     => { self.n = value.parse::<i32>().expect("Could not parse n") },
            "m"     => { self.m = value.parse::<i32>().expect("Could not parse m") },
            "adcs"  => { self.adcs = value.parse::<i32>().expect("Could not parse adcs") },
            "enob"  => { self.enob = value.parse::<f32>().expect("Could not parse enob") },
            "fs"    => { self.fs = value.parse::<f32>().expect("Could not parse fs") },
            "bl"    => { self.bl = Self::parse_vec(value).expect("Could not parse bl") },
            "wl"    => { self.wl = Self::parse_vec(value).expect("Could not parse wl") },
            "well"  => { self.well = Self::parse_vec(value).expect("Could not parse well") },
            "cell"  => { self.cell = value.to_string() },
            _       => { }
        }
    }
}

pub fn read(filename: &std::path::PathBuf) -> Result<Config, Box<dyn Error>> {
    let mut config = Config::new(filename);

    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    for line in rdr.lines() {
        let line = line.expect("Could not parse line");
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || (line.len() == 0) { continue; }

        if let Some((param, value)) = line.split_once(':') {
            config.update(param.trim(), value.trim());
        } else {
            eprintln!("Delimeter not found in string.");
        }
    }

    Ok(config)
}
