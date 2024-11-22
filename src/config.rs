use std::error::Error;
use std::io::{BufRead, BufReader};
use std::fs;

#[derive(Debug)]
pub struct Config {
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
    fn new() -> Config {
        Config {
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

    // TODO: Implement error handling
    pub fn update(&mut self, key: &str, value: &str) {
        match key.to_lowercase().as_str() {
            "n"     => { self.n = value.parse::<i32>().unwrap() },
            "m"     => { self.m = value.parse::<i32>().unwrap() },
            "adcs"  => { self.adcs = value.parse::<i32>().unwrap() },
            "enob"  => { self.enob = value.parse::<f32>().unwrap() },
            "fs"    => { self.fs = value.parse::<f32>().unwrap() },
            "bl"    => { self.bl = Self::parse_vec(value).unwrap() },
            "wl"    => { self.wl = Self::parse_vec(value).unwrap() },
            "well"  => { self.well = Self::parse_vec(value).unwrap() },
            "cell"  => { self.cell = value.to_string() },
            _       => { }
        }
    }
}

pub fn read(filename: &std::path::PathBuf) -> Result<Config, Box<dyn Error>> {
    let mut config = Config::new();

    let file = fs::File::open(filename)?;
    let rdr = BufReader::new(file);

    for line in rdr.lines() {
        let line = line.expect("Could not parse line");
        let line = line.trim();

        if line.starts_with('#') || (line.len() == 0) { continue; }

        if let Some((param, value)) = line.split_once(':') {
            config.update(param.trim(), value.trim());
        } else {
            eprintln!("Delimeter not found in string.");
        }
    }

    Ok(config)
}